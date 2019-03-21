use std::{fmt, process};

#[derive(Debug)]
struct Flags {
    zero: bool,
    sign_negative: bool,
    even_parity: bool,
    carry: bool,
    aux_carry: bool,
}

impl Flags {
    /// Returns flags as a single byte
    pub fn psw(&self) -> u8 {
        let mut psw = 0;

        if self.carry {
            psw |= 1
        }
        if self.even_parity {
            psw |= 1 << 2
        }
        if self.aux_carry {
            psw |= 1 << 4
        }
        if self.zero {
            psw |= 1 << 6
        }
        if self.sign_negative {
            psw |= 1 << 7
        }

        psw
    }

    /// Sets flags from a byte
    pub fn set_psw(&mut self, psw: u8) {
        self.carry = (psw & 1) != 0;
        self.even_parity = (psw & 1 << 2) != 0;
        self.aux_carry = (psw & 1 << 4) != 0;
        self.zero = (psw & 1 << 6) != 0;
        self.sign_negative = (psw & 1 << 7) != 0;
    }
}

pub struct State8080 {
    a: u8,
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    h: u8,
    l: u8,
    sp: usize,
    pc: usize,
    memory: [u8; 16384],
    flags: Flags,
    interrupts_enabled: bool,
}

impl fmt::Display for State8080 {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(
            f,
            "AF:{:04x}\nBC:{:04x}\nDE:{:04x}\nHL:{:04x}\nsp:{:04x}\npc:{:04x}\nflags:{:?}",
            self.af(),
            self.bc(),
            self.de(),
            self.hl(),
            self.sp,
            self.pc,
            self.flags
        )
    }
}

impl State8080 {
    // Public

    pub fn af(&self) -> u16 {
        (u16::from(self.a) << 8) | u16::from(self.flags.psw())
    }

    pub fn bc(&self) -> u16 {
        (u16::from(self.b) << 8) | u16::from(self.c)
    }

    pub fn de(&self) -> u16 {
        (u16::from(self.d) << 8) | u16::from(self.e)
    }

    pub fn hl(&self) -> u16 {
        (u16::from(self.h) << 8) | u16::from(self.l)
    }

    pub fn pc(&self) -> usize {
        self.pc
    }

    pub fn sp(&self) -> usize {
        self.sp
    }

    pub fn next_opcode(&self) -> String {
        op_name(&self.memory, self.pc)
    }

    // Private

    /// Returns the two bytes following the current instruction
    fn address(&self) -> u16 {
        (u16::from(self.memory[self.pc + 2]) << 8) | u16::from(self.memory[self.pc + 1])
    }

    fn set_bc(&mut self, val: u16) {
        self.b = (val >> 8) as u8;
        self.c = val as u8;
    }

    fn set_de(&mut self, val: u16) {
        self.d = (val >> 8) as u8;
        self.e = val as u8;
    }

    fn set_hl(&mut self, val: u16) {
        self.h = (val >> 8) as u8;
        self.l = val as u8;
    }

    pub fn new(rom: &[u8]) -> Self {
        let mut memory = [0; 16384];
        for (i, &b) in rom.iter().enumerate() {
            memory[i] = b;
        }

        Self {
            a: 0,
            b: 0,
            c: 0,
            d: 0,
            e: 0,
            h: 0,
            l: 0,
            sp: 0,
            pc: 0,
            memory,
            flags: Flags {
                zero: false,
                sign_negative: false,
                even_parity: false,
                carry: false,
                aux_carry: false,
            },
            interrupts_enabled: true,
        }
    }

    /// Steps the emulator `nanos` nanoseconds.
    /// Returns the number of cycles that were executed.
    pub fn step(&mut self, nanos: u128) -> u128 {
        // Simulates 2 MHz
        let freq = 2_000_000;
        let step_cycles = (freq * nanos) / 1_000_000_000;

        let mut spent_cycles = 0;

        while spent_cycles < step_cycles {
            spent_cycles += u128::from(self.emulate());
        }
        spent_cycles
    }

    /// Executes the next instruction.
    /// Advances PC apporpriately, and returns the number of cycles taken.
    fn emulate(&mut self) -> u32 {
        let op_code = self.memory[self.pc];

        let (pc_incr, cycles) = match op_code {
            // NOP
            0x00 => (1, 4),
            // LXI B, D16
            0x01 => {
                self.b = self.memory[self.pc + 2];
                self.c = self.memory[self.pc + 1];
                (3, 10)
            }
            // STAX B
            0x02 => {
                self.memory[self.bc() as usize] = self.a;
                (1, 7)
            }
            // INX B
            0x03 => {
                self.set_bc(self.bc() + 1);
                (1, 5)
            }
            // INR B
            0x04 => {
                self.b = self.b.wrapping_add(1);
                self.set_flags(self.b);
                (1, 5)
            }
            // DCR B
            0x05 => {
                self.b = self.b.wrapping_sub(1);
                self.set_flags(self.b);
                (1, 5)
            }
            // MVI B, D8
            0x06 => {
                self.b = self.memory[self.pc + 1];
                (2, 7)
            }
            // RLC
            0x07 => {
                let bit7: u8 = self.a & (1 << 7);
                self.a <<= 1;
                self.a |= bit7 >> 7;
                self.flags.carry = bit7 != 0;
                (1, 4)
            }
            // DAD B
            0x09 => {
                self.set_hl(self.hl() + self.bc());
                (1, 10)
            }
            // LDAX B
            0x0a => {
                self.a = self.memory[self.bc() as usize];
                (1, 7)
            }
            // INR C
            0x0c => {
                self.c = self.c.wrapping_add(1);
                self.set_flags(self.c);
                (1, 5)
            }
            // DCR C
            0x0d => {
                self.c = self.c.wrapping_sub(1);
                self.set_flags(self.c);
                (1, 5)
            }
            // MVI C, D8
            0x0e => {
                self.c = self.memory[self.pc + 1];
                (2, 7)
            }
            // RRC
            0x0f => {
                let bit0: u8 = self.a & 1;
                self.a >>= 1;
                self.a |= bit0 << 7;
                self.flags.carry = bit0 != 0;
                (1, 4)
            }
            // LXI D, D16
            0x11 => {
                self.d = self.memory[self.pc + 2];
                self.e = self.memory[self.pc + 1];
                (3, 10)
            }
            // INX D
            0x13 => {
                self.set_de(self.de() + 1);
                (1, 5)
            }
            // MVI D, D8
            0x16 => {
                self.d = self.memory[self.pc + 1];
                (2, 7)
            }
            // RAL
            0x17 => {
                let bit7: u8 = self.a & (1 << 7);
                self.a <<= 1;
                self.a |= self.flags.carry as u8;
                self.flags.carry = bit7 != 0;
                (1, 4)
            }
            // DAD D
            0x19 => {
                self.set_hl(self.hl() + self.de());
                (1, 10)
            }
            // LDAX D
            0x1a => {
                self.a = self.memory[self.de() as usize];
                (1, 7)
            }
            // MVI E, D8
            0x1e => {
                self.e = self.memory[self.pc + 1];
                (2, 7)
            }
            // RAR
            0x1f => {
                let bit0: u8 = self.a & 1;
                let bit7: u8 = self.a & (1 << 7);
                self.a >>= 1;
                self.a |= bit7;
                self.flags.carry = bit0 != 0;
                (1, 4)
            }
            // NOP
            0x20 => (1, 4),
            // LXI H, D16
            0x21 => {
                self.h = self.memory[self.pc + 2];
                self.l = self.memory[self.pc + 1];
                (3, 10)
            }
            // INX H
            0x23 => {
                self.set_hl(self.hl() + 1);
                (1, 5)
            }
            // MVI H, D8
            0x26 => {
                self.h = self.memory[self.pc + 1];
                (2, 7)
            }
            // DAD H
            0x29 => {
                self.set_hl(2 * self.hl());
                (1, 10)
            }
            // MVI L, D8
            0x2e => {
                self.l = self.memory[self.pc + 1];
                (2, 7)
            }
            // CMA
            0x2f => {
                self.a = !self.a;
                (1, 4)
            }
            // LXI SP, D16
            0x31 => {
                self.sp = self.address() as usize;
                (3, 10)
            }
            // STA adr
            0x32 => {
                self.memory[self.address() as usize] = self.a;
                (3, 13)
            }
            // MVI M, D8
            0x36 => {
                self.memory[self.hl() as usize] = self.memory[self.pc + 1];
                (2, 10)
            }
            // STC
            0x37 => {
                self.flags.carry = true;
                (1, 4)
            }
            // LDA adr
            0x3a => {
                self.a = self.memory[self.address() as usize];
                (3, 13)
            }
            // MVI A, D8
            0x3e => {
                self.a = self.memory[self.pc + 1];
                (2, 7)
            }
            // CMC
            0x3f => {
                self.flags.carry = !self.flags.carry;
                (1, 4)
            }
            // MOV D,M
            0x56 => {
                self.d = self.memory[self.hl() as usize];
                (1, 7)
            }
            // MOV E,M
            0x5e => {
                self.e = self.memory[self.hl() as usize];
                (1, 7)
            }
            // MOV H,M
            0x66 => {
                self.h = self.memory[self.hl() as usize];
                (1, 7)
            }
            // MOV L,A
            0x6f => {
                self.l = self.a;
                (1, 5)
            }
            // MOV M,A
            0x77 => {
                self.memory[self.hl() as usize] = self.a;
                (1, 7)
            }
            // MOV A,D
            0x7a => {
                self.a = self.d;
                (1, 5)
            }
            // MOV A,E
            0x7b => {
                self.a = self.e;
                (1, 5)
            }
            // MOV A,H
            0x7c => {
                self.a = self.h;
                (1, 5)
            }
            // MOV A,M
            0x7e => {
                self.a = self.memory[self.hl() as usize];
                (1, 7)
            }
            // HLT
            0x76 => {
                println!("Halt code received");
                process::exit(0)
            }
            // ADD B
            0x80 => {
                self.add(self.b);
                (1, 4)
            }
            // ADD C
            0x81 => {
                self.add(self.c);
                (1, 4)
            }
            // ADD D
            0x82 => {
                self.add(self.d);
                (1, 4)
            }
            // ADD E
            0x83 => {
                self.add(self.e);
                (1, 4)
            }
            // ADD H
            0x84 => {
                self.add(self.h);
                (1, 4)
            }
            // ADD L
            0x85 => {
                self.add(self.l);
                (1, 4)
            }
            // ADD M
            0x86 => {
                self.add(self.memory[self.hl() as usize]);
                (1, 7)
            }
            // ADD A
            0x87 => {
                self.add(self.a);
                (1, 4)
            }
            // ANA B
            0xa0 => {
                self.and(self.b);
                (1, 4)
            }
            // ANA C
            0xa1 => {
                self.and(self.c);
                (1, 4)
            }
            // ANA D
            0xa2 => {
                self.and(self.d);
                (1, 4)
            }
            // ANA E
            0xa3 => {
                self.and(self.e);
                (1, 4)
            }
            // ANA H
            0xa4 => {
                self.and(self.h);
                (1, 4)
            }
            // ANA L
            0xa5 => {
                self.and(self.l);
                (1, 4)
            }
            // ANA M
            0xa6 => {
                self.and(self.memory[self.hl() as usize]);
                (1, 7)
            }
            // ANA A
            0xa7 => {
                self.and(self.a);
                (1, 4)
            }
            // XRA B
            0xa8 => {
                self.xor(self.b);
                (1, 4)
            }
            // XRA C
            0xa9 => {
                self.xor(self.c);
                (1, 4)
            }
            // XRA D
            0xaa => {
                self.xor(self.d);
                (1, 4)
            }
            // XRA E
            0xab => {
                self.xor(self.e);
                (1, 4)
            }
            // XRA H
            0xac => {
                self.xor(self.h);
                (1, 4)
            }
            // XRA L
            0xad => {
                self.xor(self.l);
                (1, 4)
            }
            // XRA M
            0xae => {
                self.xor(self.memory[self.hl() as usize]);
                (1, 7)
            }
            // XRA A
            0xaf => {
                self.xor(self.a);
                (1, 4)
            }
            // ORA B
            0xb0 => {
                self.or(self.b);
                (1, 4)
            }
            // ORA C
            0xb1 => {
                self.or(self.c);
                (1, 4)
            }
            // ORA D
            0xb2 => {
                self.or(self.d);
                (1, 4)
            }
            // ORA E
            0xb3 => {
                self.or(self.e);
                (1, 4)
            }
            // ORA H
            0xb4 => {
                self.or(self.h);
                (1, 4)
            }
            // ORA L
            0xb5 => {
                self.or(self.l);
                (1, 4)
            }
            // ORA M
            0xb6 => {
                self.or(self.memory[self.hl() as usize]);
                (1, 7)
            }
            // ORA A
            0xb7 => {
                self.or(self.a);
                (1, 4)
            }
            // CMP B
            0xb8 => {
                self.cmp(self.b);
                (1, 4)
            }
            // CMP C
            0xb9 => {
                self.cmp(self.c);
                (1, 4)
            }
            // CMP D
            0xba => {
                self.cmp(self.d);
                (1, 4)
            }
            // CMP E
            0xbb => {
                self.cmp(self.e);
                (1, 4)
            }
            // CMP H
            0xbc => {
                self.cmp(self.h);
                (1, 4)
            }
            // CMP L
            0xbd => {
                self.cmp(self.l);
                (1, 4)
            }
            // CMP M
            0xbe => {
                self.cmp(self.memory[self.hl() as usize]);
                (1, 7)
            }
            // CMP A
            0xbf => {
                self.cmp(self.a);
                (1, 4)
            }
            // POP B
            0xc1 => {
                self.b = self.memory[self.sp + 1];
                self.c = self.memory[self.sp];
                self.sp += 2;
                (1, 10)
            }
            // PUSH B
            0xc5 => {
                self.memory[self.sp - 1] = self.b;
                self.memory[self.sp - 2] = self.c;
                self.sp -= 2;
                (1, 11)
            }
            // JNZ adr
            0xc2 => {
                if !self.flags.zero {
                    self.jmp();
                    (0, 10)
                } else {
                    (3, 10)
                }
            }
            // JMP adr
            0xc3 => {
                self.jmp();
                (0, 10)
            }
            // ADI D8
            0xc6 => {
                self.add(self.memory[self.pc + 1]);
                (2, 7)
            }
            // RET
            0xc9 => {
                self.pc =
                    self.memory[self.sp] as usize | ((self.memory[self.sp + 1] as usize) << 8);
                self.sp += 2;
                (0, 10)
            }
            // JZ adr
            0xca => {
                if self.flags.zero {
                    self.jmp();
                    (0, 10)
                } else {
                    (3, 10)
                }
            }
            // CALL adr
            0xcd => {
                let ret = self.pc + 2;
                self.memory[self.sp - 1] = (ret >> 8) as u8;
                self.memory[self.sp - 2] = ret as u8;
                self.sp -= 2;
                self.pc = self.address() as usize;
                (0, 17)
            }
            // POP D
            0xd1 => {
                self.d = self.memory[self.sp + 1];
                self.e = self.memory[self.sp];
                self.sp += 2;
                (1, 10)
            }
            // JNC adr
            0xd2 => {
                if !self.flags.carry {
                    self.jmp();
                    (0, 10)
                } else {
                    (3, 10)
                }
            }
            // OUT D8
            0xd3 => {
                //TODO
                (2, 10)
            }
            // PUSH D
            0xd5 => {
                self.memory[self.sp - 1] = self.d;
                self.memory[self.sp - 2] = self.e;
                self.sp -= 2;
                (1, 11)
            }
            // JC adr
            0xda => {
                if self.flags.carry {
                    self.jmp();
                    (0, 10)
                } else {
                    (3, 10)
                }
            }
            // IN D8
            0xdb => {
                //TODO
                (2, 10)
            }
            // POP H
            0xe1 => {
                self.h = self.memory[self.sp + 1];
                self.l = self.memory[self.sp];
                self.sp += 2;
                (1, 10)
            }
            // JPO adr
            0xe2 => {
                if !self.flags.even_parity {
                    self.jmp();
                    (0, 10)
                } else {
                    (3, 10)
                }
            }
            // PUSH H
            0xe5 => {
                self.memory[self.sp - 1] = self.h;
                self.memory[self.sp - 2] = self.l;
                self.sp -= 2;
                (1, 11)
            }
            // ANI D8
            0xe6 => {
                self.and(self.memory[self.pc + 1]);
                (2, 7)
            }
            // JPE adr
            0xea => {
                if self.flags.even_parity {
                    self.jmp();
                    (0, 10)
                } else {
                    (3, 10)
                }
            }
            // XCHG
            0xeb => {
                let d = self.d;
                let e = self.e;
                self.d = self.h;
                self.e = self.l;
                self.h = d;
                self.l = e;
                (1, 5)
            }
            // POP PSW
            0xf1 => {
                self.a = self.memory[self.sp + 1];
                self.flags.set_psw(self.memory[self.sp]);
                self.sp += 2;
                (1, 10)
            }
            // JP adr
            0xf2 => {
                if !self.flags.sign_negative {
                    self.jmp();
                    (0, 10)
                } else {
                    (3, 10)
                }
            }
            // DI
            0xf3 => {
                self.interrupts_enabled = false;
                (1, 4)
            }
            // PUSH PSW
            0xf5 => {
                self.memory[self.sp - 1] = self.a;
                self.memory[self.sp - 2] = self.flags.psw();
                self.sp -= 2;
                (1, 11)
            }
            // JM adr
            0xfa => {
                if self.flags.sign_negative {
                    self.jmp();
                    (0, 10)
                } else {
                    (3, 10)
                }
            }
            // EI
            0xfb => {
                self.interrupts_enabled = true;
                (1, 4)
            }
            // CPI D8
            0xfe => {
                self.cmp(self.memory[self.pc + 1]);
                (2, 7)
            }
            // Unimplemented
            _ => {
                println!("Unimplemented instruction: {:02x}", op_code);
                process::exit(0)
            }
        };

        self.pc += pc_incr;
        cycles
    }

    /// Jumps the PC, using the two bytes following the current PC
    fn jmp(&mut self) {
        self.pc = self.address() as usize;
    }

    fn set_flags(&mut self, value: u8) {
        // true when result is zero
        self.flags.zero = value == 0;

        // true when result is negative (sign bit is set)
        self.flags.sign_negative = (value & (1 << 7)) != 0;

        // true when instruction resulted in a carry out
        self.flags.carry = false;

        // true when the result is even
        self.flags.even_parity = Self::parity(value);
    }

    /// Adds `operand` to the A register, setting flags appropriately
    fn add(&mut self, operand: u8) {
        let result: u16 = u16::from(self.a) + u16::from(operand);

        // true when result is zero
        self.flags.zero = result.trailing_zeros() >= 8;

        // true when result is negative (sign bit is set)
        self.flags.sign_negative = (result & (1 << 7)) != 0;

        // true when instruction resulted in a carry out
        self.flags.carry = result > 0xff;

        // true when the result is even
        self.flags.even_parity = Self::parity((result & 0xff) as u8);

        self.a = result as u8;
    }

    /// Bitwise AND between the A register and `operand`
    fn and(&mut self, operand: u8) {
        let result: u8 = self.a & operand;

        // true when result is zero
        self.flags.zero = result == 0;

        // true when result is negative (sign bit is set)
        self.flags.sign_negative = (result & (1 << 7)) != 0;

        // true when instruction resulted in a carry out
        self.flags.carry = false;

        // true when the result is even
        self.flags.even_parity = Self::parity(result);

        self.a = result;
    }

    /// Bitwise OR between the A register and `operand`
    fn or(&mut self, operand: u8) {
        let result: u8 = self.a | operand;

        // true when result is zero
        self.flags.zero = result == 0;

        // true when result is negative (sign bit is set)
        self.flags.sign_negative = (result & (1 << 7)) != 0;

        // true when instruction resulted in a carry out
        self.flags.carry = false;

        // true when the result is even
        self.flags.even_parity = Self::parity(result);

        self.a = result;
    }

    /// Bitwise XOR between the A register and `operand`
    fn xor(&mut self, operand: u8) {
        let result: u8 = self.a ^ operand;

        // true when result is zero
        self.flags.zero = result == 0;

        // true when result is negative (sign bit is set)
        self.flags.sign_negative = (result & (1 << 7)) != 0;

        // true when instruction resulted in a carry out
        self.flags.carry = false;

        // true when the result is even
        self.flags.even_parity = Self::parity(result);

        self.a = result;
    }

    fn cmp(&mut self, operand: u8) {
        let result: u8 = self.a.wrapping_sub(operand);

        // true when result is zero
        self.flags.zero = result == 0;

        // true when result is negative (sign bit is set)
        self.flags.sign_negative = (result & (1 << 7)) != 0;

        // true when instruction resulted in a carry out
        self.flags.carry = self.a < operand;

        // true when the result is even
        self.flags.even_parity = Self::parity(result);
    }

    /// Returns true if the byte has an even number of 1s
    fn parity(mut x: u8) -> bool {
        let mut parity = 0u8;
        while x != 0 {
            parity ^= x & 1;
            x >>= 1;
        }

        parity != 0
    }
}

pub fn op_name(buffer: &[u8], pc: usize) -> String {
    match buffer[pc] {
        0x00 => "NOP".into(),
        0x01 => format!("LXI B, ${:02x}{:02x}", buffer[pc + 2], buffer[pc + 1]),
        0x02 => "STAX B".into(),
        0x03 => "INX B".into(),
        0x04 => "INR B".into(),
        0x05 => "DCR B".into(),
        0x06 => format!("MVI B, ${:02x}", buffer[pc + 1]),
        0x07 => "RLC".into(),
        0x08 => "NOP".into(),
        0x09 => "DAD B".into(),
        0x0a => "LDAX B".into(),
        0x0b => "DCX B".into(),
        0x0c => "INR C".into(),
        0x0d => "DCR C".into(),
        0x0e => format!("MVI C, ${:02x}", buffer[pc + 1]),
        0x0f => "RRC".into(),
        0x10 => "NOP".into(),
        0x11 => format!("LXI D, ${:02x}{:02x}", buffer[pc + 2], buffer[pc + 1]),
        0x12 => "STAX D".into(),
        0x13 => "INX D".into(),
        0x14 => "INR D".into(),
        0x15 => "DCR D".into(),
        0x16 => format!("MVI D, ${:02x}", buffer[pc + 1]),
        0x17 => "RAL".into(),
        0x18 => "NOP".into(),
        0x19 => "DAD D".into(),
        0x1a => "LDAX D".into(),
        0x1b => "DCX D".into(),
        0x1c => "INR E".into(),
        0x1d => "DCR E".into(),
        0x1e => format!("MVI E, ${:02x}", buffer[pc + 1]),
        0x1f => "RAR".into(),
        0x20 => "NOP".into(),
        0x21 => format!("LXI H, ${:02x}{:02x}", buffer[pc + 2], buffer[pc + 1]),
        0x22 => format!("SHLD ${:02x}{:02x}", buffer[pc + 2], buffer[pc + 1]),
        0x23 => "INX H".into(),
        0x24 => "INR H".into(),
        0x25 => "DCR H".into(),
        0x26 => format!("MVI H, ${:02x}", buffer[pc + 1]),
        0x27 => "DAA".into(),
        0x28 => "NOP".into(),
        0x29 => "DAD H".into(),
        0x2a => format!("LHLD ${:02x}{:02x}", buffer[pc + 2], buffer[pc + 1]),
        0x2b => "DCX H".into(),
        0x2c => "INR L".into(),
        0x2e => format!("MVI L, ${:02x}", buffer[pc + 1]),
        0x2f => "CMA".into(),
        0x30 => "NOP".into(),
        0x31 => format!("LXI SP, ${:02x}{:02x}", buffer[pc + 2], buffer[pc + 1]),
        0x32 => format!("STA ${:02x}{:02x}", buffer[pc + 2], buffer[pc + 1]),
        0x33 => "INX SP".into(),
        0x34 => "INR M".into(),
        0x35 => "DCR M".into(),
        0x36 => format!("MVI M, ${:02x}", buffer[pc + 1]),
        0x37 => "STC".into(),
        0x38 => "NOP".into(),
        0x39 => "DAD SP".into(),
        0x3a => format!("LDA ${:02x}{:02x}", buffer[pc + 2], buffer[pc + 1]),
        0x3c => "INR A".into(),
        0x3d => "DCR A".into(),
        0x3e => format!("MVI A, ${:02x}", buffer[pc + 1]),
        0x3f => "CMC".into(),
        0x40 => "MOV B,B".into(),
        0x41 => "MOV B,C".into(),
        0x42 => "MOV B,D".into(),
        0x43 => "MOV B,E".into(),
        0x44 => "MOV B,H".into(),
        0x45 => "MOV B,L".into(),
        0x46 => "MOV B,M".into(),
        0x47 => "MOV B,A".into(),
        0x48 => "MOV C,B".into(),
        0x49 => "MOV C,C".into(),
        0x4a => "MOV C,D".into(),
        0x4b => "MOV C,E".into(),
        0x4c => "MOV C,H".into(),
        0x4d => "MOV C,L".into(),
        0x4e => "MOV C,M".into(),
        0x4f => "MOV C,A".into(),
        0x50 => "MOV D,B".into(),
        0x51 => "MOV D,C".into(),
        0x52 => "MOV D,D".into(),
        0x53 => "MOV D,E".into(),
        0x54 => "MOV D,H".into(),
        0x55 => "MOV D,L".into(),
        0x56 => "MOV D,M".into(),
        0x57 => "MOV D,A".into(),
        0x58 => "MOV E,B".into(),
        0x59 => "MOV E,C".into(),
        0x5a => "MOV E,D".into(),
        0x5b => "MOV E,E".into(),
        0x5c => "MOV E,H".into(),
        0x5d => "MOV E,L".into(),
        0x5e => "MOV E,M".into(),
        0x5f => "MOV E,A".into(),
        0x60 => "MOV H,B".into(),
        0x61 => "MOV H,C".into(),
        0x62 => "MOV H,D".into(),
        0x63 => "MOV H,E".into(),
        0x64 => "MOV H,H".into(),
        0x65 => "MOV H,L".into(),
        0x66 => "MOV H,M".into(),
        0x67 => "MOV H,A".into(),
        0x68 => "MOV L,B".into(),
        0x69 => "MOV L,C".into(),
        0x6a => "MOV L,D".into(),
        0x6b => "MOV L,E".into(),
        0x6c => "MOV L,H".into(),
        0x6d => "MOV L,L".into(),
        0x6e => "MOV L,M".into(),
        0x6f => "MOV L,A".into(),
        0x70 => "MOV M,B".into(),
        0x71 => "MOV M,C".into(),
        0x72 => "MOV M,D".into(),
        0x73 => "MOV M,E".into(),
        0x74 => "MOV M,H".into(),
        0x75 => "MOV M,L".into(),
        0x76 => "HLT".into(),
        0x77 => "MOV M,A".into(),
        0x78 => "MOV A,B".into(),
        0x79 => "MOV A,C".into(),
        0x7a => "MOV A,D".into(),
        0x7b => "MOV A,E".into(),
        0x7c => "MOV A,H".into(),
        0x7d => "MOV A,L".into(),
        0x7e => "MOV A,M".into(),
        0x7f => "MOV A,A".into(),
        0x80 => "ADD B".into(),
        0x81 => "ADD C".into(),
        0x82 => "ADD D".into(),
        0x83 => "ADD E".into(),
        0x84 => "ADD H".into(),
        0x85 => "ADD L".into(),
        0x86 => "ADD M".into(),
        0x87 => "ADD A".into(),
        0x88 => "ADC B".into(),
        0x89 => "ADC C".into(),
        0x8a => "ADC D".into(),
        0x8b => "ADC E".into(),
        0x8c => "ADC H".into(),
        0x8d => "ADC L".into(),
        0x8e => "ADC M".into(),
        0x8f => "ADC A".into(),
        0x90 => "SUB B".into(),
        0x91 => "SUB C".into(),
        0x92 => "SUB D".into(),
        0x93 => "SUB E".into(),
        0x94 => "SUB H".into(),
        0x95 => "SUB L".into(),
        0x96 => "SUB M".into(),
        0x97 => "SUB A".into(),
        0x98 => "SBB B".into(),
        0x99 => "SBB C".into(),
        0x9a => "SBB D".into(),
        0x9b => "SBB E".into(),
        0x9c => "SBB H".into(),
        0x9d => "SBB L".into(),
        0x9e => "SBB M".into(),
        0x9f => "SBB A".into(),
        0xa0 => "ANA B".into(),
        0xa1 => "ANA C".into(),
        0xa2 => "ANA D".into(),
        0xa3 => "ANA E".into(),
        0xa4 => "ANA H".into(),
        0xa5 => "ANA L".into(),
        0xa6 => "ANA M".into(),
        0xa7 => "ANA A".into(),
        0xa8 => "XRA B".into(),
        0xa9 => "XRA C".into(),
        0xaa => "XRA D".into(),
        0xab => "XRA E".into(),
        0xac => "XRA H".into(),
        0xad => "XRA L".into(),
        0xae => "XRA M".into(),
        0xaf => "XRA A".into(),
        0xb0 => "ORA B".into(),
        0xb1 => "ORA C".into(),
        0xb2 => "ORA D".into(),
        0xb3 => "ORA E".into(),
        0xb4 => "ORA H".into(),
        0xb5 => "ORA L".into(),
        0xb6 => "ORA M".into(),
        0xb7 => "ORA A".into(),
        0xb8 => "CMP B".into(),
        0xb9 => "CMP C".into(),
        0xba => "CMP D".into(),
        0xbb => "CMP E".into(),
        0xbc => "CMP H".into(),
        0xbd => "CMP L".into(),
        0xbe => "CMP M".into(),
        0xbf => "CMP A".into(),
        0xc0 => "RNZ".into(),
        0xc1 => "POP B".into(),
        0xc2 => format!("JNZ ${:02x}{:02x}", buffer[pc + 2], buffer[pc + 1]),
        0xc3 => format!("JMP ${:02x}{:02x}", buffer[pc + 2], buffer[pc + 1]),
        0xc4 => format!("CNZ ${:02x}{:02x}", buffer[pc + 2], buffer[pc + 1]),
        0xc5 => "PUSH B".into(),
        0xc6 => "ADI D8".into(),
        0xc8 => "RZ".into(),
        0xca => format!("JZ ${:02x}{:02x}", buffer[pc + 2], buffer[pc + 1]),
        0xcc => format!("CZ ${:02x}{:02x}", buffer[pc + 2], buffer[pc + 1]),
        0xcd => format!("CALL ${:02x}{:02x}", buffer[pc + 2], buffer[pc + 1]),
        0xc9 => "RET".into(),
        0xd0 => "RNC".into(),
        0xd1 => "POP D".into(),
        0xd2 => format!("JNC ${:02x}{:02x}", buffer[pc + 2], buffer[pc + 1]),
        0xd3 => "OUT D8".into(),
        0xd4 => format!("CNC ${:02x}{:02x}", buffer[pc + 2], buffer[pc + 1]),
        0xd5 => "PUSH D".into(),
        0xd6 => "SUI D8".into(),
        0xd8 => "RC".into(),
        0xda => format!("JC ${:02x}{:02x}", buffer[pc + 2], buffer[pc + 1]),
        0xdb => "IN D8".into(),
        0xdc => format!("CC ${:02x}{:02x}", buffer[pc + 2], buffer[pc + 1]),
        0xdd => "NOP".into(),
        0xde => "SBI D8".into(),
        0xe0 => "RPO".into(),
        0xe1 => "POP H".into(),
        0xe2 => format!("JPO ${:02x}{:02x}", buffer[pc + 2], buffer[pc + 1]),
        0xe3 => "XTHL".into(),
        0xe4 => format!("CPO ${:02x}{:02x}", buffer[pc + 2], buffer[pc + 1]),
        0xe5 => "PUSH H".into(),
        0xe6 => "ANI D8".into(),
        0xe9 => "PCHL".into(),
        0xeb => "XCHG".into(),
        0xec => format!("CPE ${:02x}{:02x}", buffer[pc + 2], buffer[pc + 1]),
        0xee => "XRI D8".into(),
        0xf0 => "RP".into(),
        0xf1 => "POP PSW".into(),
        0xf5 => "PUSH PSW".into(),
        0xf6 => "ORI D8".into(),
        0xf7 => "RST 6".into(),
        0xf8 => "RM".into(),
        0xfa => format!("JM ${:02x}{:02x}", buffer[pc + 2], buffer[pc + 1]),
        0xfb => "EI".into(),
        0xfc => format!("CM ${:02x}{:02x}", buffer[pc + 2], buffer[pc + 1]),
        0xfe => "CPI D8".into(),
        0xff => "RST 7".into(),
        _ => format!("Unknown opcode: {:02x}", buffer[pc]),
    }
}
