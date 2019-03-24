use crate::i8080::{RegisterPair, State8080};
use image::{Rgba, RgbaImage};
use piston_window::{Button, ButtonState, Key};

pub trait Machine {
    fn screen(&self) -> RgbaImage;
    fn width(&self) -> u32;
    fn height(&self) -> u32;

    fn step(&mut self, dt: f64);

    fn interrupt(&mut self, interrupt_num: u16);

    fn key_press(&mut self, button: Button, button_state: ButtonState);

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
            cycles: 0,
        }
    }
}

impl Machine for SpaceInvaders {
    fn screen(&self) -> RgbaImage {
        let mut x = 0;
        let mut y = self.height() - 1;

        let mut buffer = RgbaImage::new(self.width(), self.height());

        for &byte in &self.state.memory()[0x2400..0x4000] {
            for bit in 0..8 {
                let pixel_on = byte & (1 << bit) != 0;
                let pixel: [u8; 4] = if pixel_on {
                    [0xff, 0xff, 0xff, 0xff]
                } else {
                    [0x00, 0x00, 0x00, 0xff]
                };

                buffer.put_pixel(x, y, Rgba { data: pixel });

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

    fn width(&self) -> u32 {
        224
    }

    fn height(&self) -> u32 {
        256
    }

    fn step(&mut self, dt: f64) {
        self.cycles += self.state.step(dt, &mut self.io_state);
    }

    fn interrupt(&mut self, interrupt_num: u16) {
        self.state.interrupt(interrupt_num)
    }

    fn key_press(&mut self, button: Button, button_state: ButtonState) {
        self.io_state.key_press(button, button_state)
    }

    fn debug_text(&self) -> Vec<String> {
        vec![
            format!("Cycles: {}", self.cycles),
            format!("Seconds: {}", self.cycles / 2_000_000),
            self.state.next_opcode(),
            format!("A: {:02x} B: {:02x} C: {:02x} D: {:02x}", self.state.a(), self.state.b(), self.state.c(), self.state.d()),
            format!("E: {:02x} H: {:02x} L: {:02x}", self.state.e(), self.state.h(), self.state.l()),
            format!("AF: {:04x} BC: {:04x}", self.state.af(), self.state.bc()),
            format!("DE: {:04x} HL: {:04x}", self.state.de(), self.state.hl()),
            format!("PC: {:04x} SP: {:04x}", self.state.pc(), self.state.sp()),
        ]
    }
}

pub struct SpaceInvadersIO {
    shift_register: RegisterPair,
    shift_amount: u8,
    port1: u8,
    port2: u8,
}

impl SpaceInvadersIO {
    fn new() -> Self {
        Self {
            shift_register: RegisterPair { both: 0 },
            shift_amount: 0,
            port1: 0b10010000,
            port2: 0b00100000,
        }
    }

    fn key_press(&mut self, button: Button, button_state: ButtonState) {
        if let Button::Keyboard(key) = button {
            match key {
                // Coin
                Key::C => Self::set_key(&mut self.port1, 0, button_state),
                // P1 Start
                Key::Return => Self::set_key(&mut self.port1, 2, button_state),
                // P1 shoot
                Key::Space => Self::set_key(&mut self.port1, 4, button_state),
                // P1 left
                Key::Left => Self::set_key(&mut self.port1, 5, button_state),
                // P1 right
                Key::Right => Self::set_key(&mut self.port1, 6, button_state),
                _ => {}
            }
        }
    }

    fn set_key(port: &mut u8, bit: u8, button_state: ButtonState) {
        match button_state {
            ButtonState::Press => *port |= 1 << bit,
            ButtonState::Release => *port &= !(1 << bit),
        }
    }
}

impl IOState for SpaceInvadersIO {
    fn input(&self, port: u8) -> u8 {
        match port {
            1 => self.port1,
            2 => self.port2,
            3 => (unsafe { self.shift_register.both } >> (8 - self.shift_amount)) as u8,
            _ => panic!("Cannot read port: {}", port),
        }
    }

    fn output(&mut self, port: u8, value: u8) {
        match port {
            2 => self.shift_amount = value & 0b111,
            4 => unsafe {
                self.shift_register.one.0 = self.shift_register.one.1;
                self.shift_register.one.1 = value;
            },
            _ => {}
        }
    }
}
