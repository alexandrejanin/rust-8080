use std;

use i8080;

use crate::cpu::{CpuState, RegisterPair};

/// Interface between the emulator's IO functions and the machine state
pub trait IOState {
    fn input(&self, port: u8) -> u8;
    fn output(&mut self, port: u8, value: u8);
}

pub struct SpaceInvaders {
    ref_cpu: i8080::Cpu,
    ref_io_state: SpaceInvadersIO,
    cpu: CpuState,
    io_state: SpaceInvadersIO,
    window_buffer: [u32; 224 * 256],
    instructions: u64,
    cycles: u64,
    frames: u64,
}

impl SpaceInvaders {
    const CYCLES_PER_FRAME: u64 = 4_000_000 / 60;
    pub const SCREEN_WIDTH: usize = 224;
    pub const SCREEN_HEIGHT: usize = 256;

    pub fn new() -> Self {
        Self::from_rom(include_bytes!("invaders.rom"))
    }

    pub fn from_rom(rom: &[u8]) -> Self {
        let mut ref_cpu = i8080::Cpu::new();
        ref_cpu.load_into_rom(rom, 0);
        ref_cpu.pc = 0_u16.into();

        Self {
            ref_cpu,
            ref_io_state: SpaceInvadersIO::new(),
            cpu: CpuState::from_rom(rom, 0, 0),
            io_state: SpaceInvadersIO::new(),
            window_buffer: [0; 224 * 256],
            instructions: 0,
            cycles: 0,
            frames: 0,
        }
    }

    // Proceeds one frame of the emulator
    pub fn step(&mut self, window: &mut minifb::Window) {
        self.half_step(window, true);
        self.half_step(window, false);

        self.frames += 1;

        // Lastly, update input
        self.ref_io_state.update_input(&window);
        self.io_state.update_input(window);

        std::thread::sleep(std::time::Duration::from_millis(16));
    }

    fn half_step(&mut self, window: &mut minifb::Window, top_half: bool) {
        let mut cycles_spent = 0;
        while cycles_spent < Self::CYCLES_PER_FRAME / 2 {
            self.ref_cpu.emulate(&mut self.ref_io_state);
            let cycles = self.cpu.emulate(&mut self.io_state);

            self.cpu_compare();

            cycles_spent += cycles;

            // For monitoring/debug purposes
            self.instructions += 1;
            self.cycles += cycles;
        }

        // Render half of the screen
        window.update_with_buffer(&self.screen(top_half))
              .unwrap_or_else(|e| println!("Failed to update window buffer: {}", e));

        // Middle/end of frame interrupt
        if self.ref_cpu.int_enable {
            self.ref_cpu.interrupt(if top_half { 8 } else { 16 });
        }
        self.cpu.interrupt(if top_half { 1 } else { 2 });
    }

    fn screen(&mut self, top_half: bool) -> &[u32] {
        let (start_memory, start_pixel) = if top_half {
            (0x2400, 0)
        } else {
            (0x3200, 0x7000)
        };

        // Iterate half the screen
        for offset in 0..0xE00 {
            let byte = self.cpu.memory()[start_memory + offset];

            for bit in 0..8 {
                let color: u32 = if byte & (1 << bit) == 0 {
                    0x00_00_00_00
                } else {
                    0xff_ff_ff_ff
                };

                let x = (start_pixel + 8 * offset + bit) / Self::SCREEN_HEIGHT;
                let y = Self::SCREEN_HEIGHT - 1 - (start_pixel + 8 * offset + bit) % Self::SCREEN_HEIGHT;

                self.window_buffer[x + y * Self::SCREEN_WIDTH] = color;
            }
        }

        &self.window_buffer
    }

    fn cpu_compare(&self) {
        assert_eq!(self.cpu.pc(), self.ref_cpu.pc.into(), "PC mismatch\n{:?}\n{:?}", self.cpu, self.ref_cpu);
        assert_eq!(self.cpu.a(), self.ref_cpu.a.into(), "A mismatch\n{:?}\n{:?}", self.cpu, self.ref_cpu);
        assert_eq!(self.cpu.b(), self.ref_cpu.b.into(), "B mismatch\n{:?}\n{:?}", self.cpu, self.ref_cpu);
        assert_eq!(self.cpu.c(), self.ref_cpu.c.into(), "C mismatch\n{:?}\n{:?}", self.cpu, self.ref_cpu);
        assert_eq!(self.cpu.d(), self.ref_cpu.d.into(), "D mismatch\n{:?}\n{:?}", self.cpu, self.ref_cpu);
        assert_eq!(self.cpu.e(), self.ref_cpu.e.into(), "E mismatch\n{:?}\n{:?}", self.cpu, self.ref_cpu);
        assert_eq!(self.cpu.h(), self.ref_cpu.h.into(), "H mismatch\n{:?}\n{:?}", self.cpu, self.ref_cpu);
        assert_eq!(self.cpu.l(), self.ref_cpu.l.into(), "L mismatch\n{:?}\n{:?}", self.cpu, self.ref_cpu);
        assert_eq!(self.cpu.sp(), self.ref_cpu.sp.into(), "SP mismatch\n{:?}\n{:?}", self.cpu, self.ref_cpu);
        assert_eq!(self.cpu.flags().sign, self.ref_cpu.conditions.s, "Sign mismatch\n{:?}\n{:?}", self.cpu, self.ref_cpu);
        assert_eq!(self.cpu.flags().zero, self.ref_cpu.conditions.z, "Zero mismatch\n{:?}\n{:?}", self.cpu, self.ref_cpu);
        assert_eq!(self.cpu.flags().aux_carry, self.ref_cpu.conditions.ac, "Aux mismatch\n{:?}\n{:?}", self.cpu, self.ref_cpu);
        assert_eq!(self.cpu.flags().parity, self.ref_cpu.conditions.p, "Parity mismatch\n{:?}\n{:?}", self.cpu, self.ref_cpu);
        assert_eq!(self.cpu.flags().carry, self.ref_cpu.conditions.cy, "Carry mismatch\n{:?}\n{:?}", self.cpu, self.ref_cpu);
    }
}

pub struct SpaceInvadersIO {
    shift_register: RegisterPair,
    shift_amount: u8,
    port0: u8,
    port1: u8,
    port2: u8,
}

impl SpaceInvadersIO {
    pub fn new() -> Self {
        Self {
            shift_register: RegisterPair::new(),
            shift_amount: 0,
            port0: 0b0111_0000,
            port1: 0b0001_0000,
            port2: 0b0000_0000,
        }
    }

    fn update_input(&mut self, window: &minifb::Window) {
        // Credit
        Self::set_key(&mut self.port1, 0, window.is_key_down(minifb::Key::C));
        // P2 Start
        Self::set_key(&mut self.port1, 1, window.is_key_down(minifb::Key::W));
        // P1 Start
        Self::set_key(&mut self.port1, 2, window.is_key_down(minifb::Key::Q));
        // Always 1
        Self::set_key(&mut self.port1, 3, true);

        // P1 Fire
        Self::set_key(&mut self.port1, 4, window.is_key_down(minifb::Key::Space));
        // P1 Left
        Self::set_key(&mut self.port1, 5, window.is_key_down(minifb::Key::A));
        // P1 Right
        Self::set_key(&mut self.port1, 6, window.is_key_down(minifb::Key::D));

        // P2 Fire
        Self::set_key(&mut self.port2, 4, window.is_key_down(minifb::Key::Enter));
        // P2 Left
        Self::set_key(&mut self.port2, 5, window.is_key_down(minifb::Key::Left));
        // P2 Right
        Self::set_key(&mut self.port2, 6, window.is_key_down(minifb::Key::Right));
    }

    fn set_key(port: &mut u8, bit: u8, on: bool) {
        if on {
            *port |= 1 << bit
        } else {
            *port &= !(1 << bit)
        }
    }
}

impl IOState for SpaceInvadersIO {
    fn input(&self, port: u8) -> u8 {
        match port {
            1 => self.port1,
            2 => self.port2,
            3 => (self.shift_register.both() >> (8 - self.shift_amount)) as u8,
            _ => panic!("Cannot read port: {}", port),
        }
    }

    fn output(&mut self, port: u8, value: u8) {
        match port {
            2 => self.shift_amount = value & 0b111,
            4 => {
                *self.shift_register.lsb_mut() = self.shift_register.msb();
                *self.shift_register.msb_mut() = value;
            }
            3 | 5 | 6 => {}
            _ => panic!("Cannot write to port: {}", port),
        }
    }
}

impl i8080::Machine for SpaceInvadersIO {
    fn input(&mut self, port: u8) -> u8 {
        IOState::input(self, port)
    }

    fn output(&mut self, port: u8, byte: u8) {
        IOState::output(self, port, byte)
    }
}
