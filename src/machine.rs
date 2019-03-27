use crate::i8080::{RegisterPair, State8080};

pub trait Machine {
    fn screen(&self) -> Vec<u32>;
    fn width(&self) -> usize;
    fn height(&self) -> usize;
    fn step(&mut self, dt: f64);
    fn interrupt(&mut self, interrupt_num: u16);
    fn update_input(&mut self, window: &minifb::Window);
    fn debug_text(&self) -> Vec<String>;
}

/// Interface between the emulator's IO functions and the machine state
pub trait IOState {
    fn input(&self, port: u8) -> u8;
    fn output(&mut self, port: u8, value: u8);
}

pub struct SpaceInvaders {
    state: State8080,
    io_state: SpaceInvadersIO,
    instructions: u64,
    cycles: u64,
}

impl SpaceInvaders {
    pub fn new() -> Self {
        Self::from_rom(include_bytes!("invaders.rom"))
    }

    pub fn from_rom(rom: &[u8]) -> Self {
        Self {
            state: State8080::new(rom),
            io_state: SpaceInvadersIO::new(),
            instructions: 0,
            cycles: 0,
        }
    }
}

impl Machine for SpaceInvaders {
    fn screen(&self) -> Vec<u32> {
        let mut x = 0;
        let mut y = self.height() - 1;

        let mut buffer = vec![0u32; self.width() * self.height()];

        for &byte in &self.state.memory()[0x2400..0x4000] {
            for bit in 0..8 {
                let pixel_on = byte & (1 << bit) != 0;
                let pixel: u32 = if pixel_on {
                    0xff_ff_ff_ff
                } else {
                    0x00_00_00_00
                };

                buffer[x + y * self.width()] = pixel;

                if y > 0 {
                    y -= 1;
                } else {
                    y = self.height() - 1;
                    x += 1;
                }
            }
        }

        buffer
    }

    fn width(&self) -> usize {
        224
    }

    fn height(&self) -> usize {
        256
    }

    fn step(&mut self, dt: f64) {
        let (instructions, cycles) = self.state.step(dt, &mut self.io_state);
        self.instructions += instructions;
        self.cycles += cycles;
    }

    fn interrupt(&mut self, interrupt_num: u16) {
        self.state.interrupt(interrupt_num)
    }

    fn update_input(&mut self, window: &minifb::Window) {
        self.io_state.update_input(window)
    }

    fn debug_text(&self) -> Vec<String> {
        vec![
            format!("Cycles: {}", self.cycles),
            format!("Seconds: {}", self.cycles / 2_000_000),
            self.state.next_opcode(),
            format!("AF: {:04x} BC: {:04x}", self.state.af(), self.state.bc()),
            format!("DE: {:04x} HL: {:04x}", self.state.de(), self.state.hl()),
            format!("PC: {:04x} SP: {:04x}", self.state.pc(), self.state.sp()),
        ]
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
    fn new() -> Self {
        Self {
            shift_register: RegisterPair::new(),
            shift_amount: 0,
            port0: 0b01110000,
            port1: 0b00010000,
            port2: 0b00000000,
        }
    }

    fn update_input(&mut self, window: &minifb::Window) {
        // Fire
        Self::set_key(&mut self.port0, 4, window.is_key_down(minifb::Key::Space));
        // Left
        Self::set_key(&mut self.port0, 5, window.is_key_down(minifb::Key::Left));
        // Right
        Self::set_key(&mut self.port0, 6, window.is_key_down(minifb::Key::Right));
    }

    fn set_key(port: &mut u8, bit: u8, on: bool) {
        match on {
            true => *port |= 1 << bit,
            false => *port &= !(1 << bit),
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
            3 => {}
            4 => {
                *self.shift_register.lsb_mut() = self.shift_register.msb();
                *self.shift_register.msb_mut() = value;
            }
            5 => {}
            6 => {}
            _ => panic!("Cannot write to port: {}", port),
        }
    }
}
