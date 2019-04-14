use std::fmt;

#[derive(Debug)]
pub struct Flags {
    pub sign: bool,
    pub zero: bool,
    pub aux_carry: bool,
    pub parity: bool,
    pub carry: bool,
}

impl fmt::Display for Flags {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        let mut flags = String::new();

        if self.sign {
            flags += "S,"
        }
        if self.zero {
            flags += "Z,"
        }
        if self.aux_carry {
            flags += "A,"
        }
        if self.parity {
            flags += "P,"
        }
        if self.carry {
            flags += "C,"
        }

        write!(f, "{}", flags)
    }
}

impl Flags {
    /// Returns flags as a single byte
    pub fn psw(&self) -> u8 {
        let mut psw = 0;

        if self.sign {
            psw |= 1 << 7
        }
        if self.zero {
            psw |= 1 << 6
        }
        if self.aux_carry {
            psw |= 1 << 4
        }
        if self.parity {
            psw |= 1 << 2
        }
        if self.carry {
            psw |= 1
        }

        psw
    }

    /// Sets flags from a byte
    pub fn set_psw(&mut self, psw: u8) {
        self.carry = (psw & 1) != 0;
        self.parity = (psw & 1 << 2) != 0;
        self.aux_carry = (psw & 1 << 4) != 0;
        self.zero = (psw & 1 << 6) != 0;
        self.sign = (psw & 1 << 7) != 0;
    }

    pub fn set_all(&mut self, value: u16, aux_value: u8) {
        self.set_sign(value as u8);
        self.set_zero(value as u8);
        self.set_aux_carry(aux_value);
        self.set_parity(value as u8);
        self.set_carry(value);
    }

    pub fn set_all_but_aux_carry(&mut self, value: u16) {
        self.set_sign(value as u8);
        self.set_zero(value as u8);
        self.set_parity(value as u8);
        self.set_carry(value);
    }

    pub fn set_all_but_carry(&mut self, value: u8) {
        self.set_sign(value);
        self.set_zero(value);
        self.set_aux_carry(value);
        self.set_parity(value);
    }

    fn set_sign(&mut self, value: u8) {
        self.sign = value & (1 << 7) != 0;
    }

    fn set_zero(&mut self, value: u8) {
        self.zero = value == 0;
    }

    fn set_aux_carry(&mut self, value: u8) {
        self.aux_carry = value > 0xf;
    }

    fn set_parity(&mut self, value: u8) {
        self.parity = value.count_ones() % 2 == 0;
    }

    pub fn set_carry(&mut self, value: u16) {
        self.carry = value > 0xff;
    }
}
