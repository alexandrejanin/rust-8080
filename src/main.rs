use std::{fmt, io, process};
use std::io::Read;

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

        if self.carry { psw |= 1 }
        if self.even_parity { psw |= 1 << 2 }
        if self.aux_carry { psw |= 1 << 4 }
        if self.zero { psw |= 1 << 6 }
        if self.sign_negative { psw |= 1 << 7 }

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

struct State8080 {
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
    interrupts_enabled: bool
}

impl fmt::Display for State8080 {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(
            f, "AF:{:04x}\nBC:{:04x}\nDE:{:04x}\nHL:{:04x}\nsp:{:04x}\npc:{:04x}\nflags:{:?}",
            self.af(), self.bc(), self.de(), self.hl(), self.sp, self.pc, self.flags
        )
    }
}

impl State8080 {
    /// Returns the two bytes following the current instruction
    fn address(&self) -> u16 {
        (u16::from(self.memory[self.pc + 2]) << 8) | u16::from(self.memory[self.pc + 1])
    }

    fn af(&self) -> u16 {
        (u16::from(self.a) << 8) | u16::from(self.flags.psw())
    }

    fn bc(&self) -> u16 {
        (u16::from(self.b) << 8) | u16::from(self.c)
    }

    fn set_bc(&mut self, val: u16) {
        self.b = (val >> 8) as u8;
        self.c = val as u8;
    }

    fn de(&self) -> u16 {
        (u16::from(self.d) << 8) | u16::from(self.e)
    }

    fn set_de(&mut self, val: u16) {
        self.d = (val >> 8) as u8;
        self.e = val as u8;
    }

    fn hl(&self) -> u16 {
        (u16::from(self.h) << 8) | u16::from(self.l)
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
                aux_carry: false
            },
            interrupts_enabled: true
        }
    }

    pub fn emulate(&mut self) {
        let op_code = self.memory[self.pc];

        self.pc += match op_code {
            // NOP
            0x00 => 1,
            // LXI B, D16
            0x01 => {
                self.b = self.memory[self.pc + 2];
                self.c = self.memory[self.pc + 1];
                3
            },
            // STAX B
            0x02 => {
                self.memory[self.bc() as usize] = self.a;
                1
            },
            // INX B
            0x03 => {
                self.set_bc(self.bc() + 1);
                1
            },
            // INR B
            0x04 => {
                self.b = self.b.wrapping_add(1);
                self.set_flags(self.b);
                1
            },
            // DCR B
            0x05 => {
                self.b = self.b.wrapping_sub(1);
                self.set_flags(self.b);
                1
            },
            // MVI B, D8
            0x06 => {
                self.b = self.memory[self.pc + 1];
                2
            },
            // RLC
            0x07 => {
                let bit7: u8 = self.a & (1 << 7);
                self.a <<= 1;
                self.a |= bit7 >> 7;
                self.flags.carry = bit7 != 0;
                1
            },
            // DAD B
            0x09 => {
                self.set_hl(self.hl() + self.bc());
                1
            },
            // LDAX B
            0x0a => {
                self.a = self.memory[self.bc() as usize];
                1
            },
            // INR C
            0x0c => {
                self.c = self.c.wrapping_add(1);
                self.set_flags(self.c);
                1
            },
            // DCR C
            0x0d => {
                self.c = self.c.wrapping_sub(1);
                self.set_flags(self.c);
                1
            },
            // MVI C, D8
            0x0e => {
                self.c = self.memory[self.pc + 1];
                2
            },
            // RRC
            0x0f => {
                let bit0: u8 = self.a & 1;
                self.a >>= 1;
                self.a |= bit0 << 7;
                self.flags.carry = bit0 != 0;
                1
            },
            // LXI D, D16
            0x11 => {
                self.d = self.memory[self.pc + 2];
                self.e = self.memory[self.pc + 1];
                3
            },
            // INX D
            0x13 => {
                self.set_de(self.de() + 1);
                1
            },
            // MVI D, D8
            0x16 => {
                self.d = self.memory[self.pc + 1];
                2
            },
            // RAL
            0x17 => {
                let bit7: u8 = self.a & (1 << 7);
                self.a <<= 1;
                self.a |= self.flags.carry as u8;
                self.flags.carry = bit7 != 0;
                1
            },
            // DAD D
            0x19 => {
                self.set_hl(self.hl() + self.de());
                1
            },
            // LDAX D
            0x1a => {
                self.a = self.memory[self.de() as usize];
                1
            },
            // MVI E, D8
            0x1e => {
                self.e = self.memory[self.pc + 1];
                2
            },
            // RAR
            0x1f => {
                let bit0: u8 = self.a & 1;
                let bit7: u8 = self.a & (1 << 7);
                self.a >>= 1;
                self.a |= bit7;
                self.flags.carry = bit0 != 0;
                1
            },
            // NOP
            0x20 => {
                1
            },
            // LXI H, D16
            0x21 => {
                self.h = self.memory[self.pc + 2];
                self.l = self.memory[self.pc + 1];
                3
            },
            // INX B
            0x23 => {
                self.set_hl(self.hl() + 1);
                1
            },
            // MVI H, D8
            0x26 => {
                self.h = self.memory[self.pc + 1];
                2
            },
            // DAD H
            0x29 => {
                self.set_hl(2 * self.hl());
                1
            },
            // MVI L, D8
            0x2e => {
                self.l = self.memory[self.pc + 1];
                2
            },
            // CMA
            0x2f => {
                self.a = !self.a;
                1
            },
            // LXI SP, D16
            0x31 => {
                self.sp = self.address() as usize;
                3
            },
            // STA adr
            0x32 => {
                self.memory[self.address() as usize] = self.a;
                3
            },
            // MVI M, D8
            0x36 => {
                self.memory[self.hl() as usize] = self.memory[self.pc + 1];
                2
            },
            // STC
            0x37 => {
                self.flags.carry = true;
                1
            },
            // LDA adr
            0x3a => {
                self.a = self.memory[self.address() as usize];
                3
            },
            // MVI A, D8
            0x3e => {
                self.a = self.memory[self.pc + 1];
                2
            },
            // CMC
            0x3f => {
                self.flags.carry = !self.flags.carry;
                1
            },
            // MOV D,M
            0x56 => {
                self.d = self.memory[self.hl() as usize];
                1
            },
            // MOV E,M
            0x5e => {
                self.e = self.memory[self.hl() as usize];
                1
            },
            // MOV H,M
            0x66 => {
                self.h = self.memory[self.hl() as usize];
                1
            },
            // MOV L,A
            0x6f => {
                self.l = self.a;
                1
            },
            // MOV M,A
            0x77 => {
                self.memory[self.hl() as usize] = self.a;
                1
            },
            // MOV A,D
            0x7a => {
                self.a = self.d;
                1
            },
            // MOV A,E
            0x7b => {
                self.a = self.e;
                1
            },
            // MOV A,H
            0x7c => {
                self.a = self.h;
                1
            },
            // MOV A,M
            0x7e => {
                self.a = self.memory[self.hl() as usize];
                1
            },
            // HLT
            0x76 => {
                println!("Halt code received");
                process::exit(0)
            },
            // ADD B
            0x80 => {
                self.add(self.b);
                1
            },
            // ADD C
            0x81 => {
                self.add(self.c);
                1
            },
            // ADD D
            0x82 => {
                self.add(self.d);
                1
            },
            // ADD E
            0x83 => {
                self.add(self.e);
                1
            },
            // ADD H
            0x84 => {
                self.add(self.h);
                1
            },
            // ADD L
            0x85 => {
                self.add(self.l);
                1
            },
            // ADD M
            0x86 => {
                self.add(self.memory[self.hl() as usize]);
                1
            },
            // ADD A
            0x87 => {
                self.add(self.a);
                1
            },
            // ANA B
            0xa0 => {
                self.and(self.b);
                1
            },
            // ANA C
            0xa1 => {
                self.and(self.c);
                1
            },
            // ANA D
            0xa2 => {
                self.and(self.d);
                1
            },
            // ANA E
            0xa3 => {
                self.and(self.e);
                1
            },
            // ANA H
            0xa4 => {
                self.and(self.h);
                1
            },
            // ANA L
            0xa5 => {
                self.and(self.l);
                1
            },
            // ANA M
            0xa6 => {
                self.and(self.memory[self.hl() as usize]);
                1
            },
            // ANA A
            0xa7 => {
                self.and(self.a);
                1
            },
            // XRA B
            0xa8 => {
                self.xor(self.b);
                1
            },
            // XRA C
            0xa9 => {
                self.xor(self.c);
                1
            },
            // XRA D
            0xaa => {
                self.xor(self.d);
                1
            },
            // XRA E
            0xab => {
                self.xor(self.e);
                1
            },
            // XRA H
            0xac => {
                self.xor(self.h);
                1
            },
            // XRA L
            0xad => {
                self.xor(self.l);
                1
            },
            // XRA M
            0xae => {
                self.xor(self.memory[self.hl() as usize]);
                1
            },
            // XRA A
            0xaf => {
                self.xor(self.a);
                1
            },
            // ORA B
            0xb0 => {
                self.or(self.b);
                1
            },
            // ORA C
            0xb1 => {
                self.or(self.c);
                1
            },
            // ORA D
            0xb2 => {
                self.or(self.d);
                1
            },
            // ORA E
            0xb3 => {
                self.or(self.e);
                1
            },
            // ORA H
            0xb4 => {
                self.or(self.h);
                1
            },
            // ORA L
            0xb5 => {
                self.or(self.l);
                1
            },
            // ORA M
            0xb6 => {
                self.or(self.memory[self.hl() as usize]);
                1
            },
            // ORA A
            0xb7 => {
                self.or(self.a);
                1
            },
            // CMP B
            0xb8 => {
                self.cmp(self.b);
                1
            },
            // CMP C
            0xb9 => {
                self.cmp(self.c);
                1
            },
            // CMP D
            0xba => {
                self.cmp(self.d);
                1
            },
            // CMP E
            0xbb => {
                self.cmp(self.e);
                1
            },
            // CMP H
            0xbc => {
                self.cmp(self.h);
                1
            },
            // CMP L
            0xbd => {
                self.cmp(self.l);
                1
            },
            // CMP M
            0xbe => {
                self.cmp(self.memory[self.hl() as usize]);
                1
            },
            // CMP A
            0xbf => {
                self.cmp(self.a);
                1
            },
            // POP B
            0xc1 => {
                self.b = self.memory[self.sp + 1];
                self.c = self.memory[self.sp];
                self.sp += 2;
                1
            },
            // PUSH B
            0xc5 => {
                self.memory[self.sp - 1] = self.b;
                self.memory[self.sp - 2] = self.c;
                self.sp -= 2;
                1
            },
            // JNZ adr
            0xc2 => {
                if !self.flags.zero {
                    self.jmp();
                    0
                } else {
                    3
                }
            },
            // JMP adr
            0xc3 => {
                self.jmp();
                0
            },
            // ADI D8
            0xc6 => {
                self.add(self.memory[self.pc + 1]);
                2
            },
            // RET
            0xc9 => {
                self.pc = self.memory[self.sp] as usize | ((self.memory[self.sp + 1] as usize) << 8);
                self.sp += 2;
                1
            },
            // JZ adr
            0xca => {
                if self.flags.zero {
                    self.jmp();
                    0
                } else {
                    3
                }
            },
            // CALL adr
            0xcd => {
                let ret = self.pc + 2;
                self.memory[self.sp - 1] = (ret >> 8) as u8;
                self.memory[self.sp - 2] = ret as u8;
                self.sp -= 2;
                self.pc = self.address() as usize;
                0
            },
            // POP D
            0xd1 => {
                self.d = self.memory[self.sp + 1];
                self.e = self.memory[self.sp];
                self.sp += 2;
                1
            },
            // JNC adr
            0xd2 => {
                if !self.flags.carry {
                    self.jmp();
                    0
                } else {
                    3
                }
            },
            // OUT D8
            0xd3 => {
                //TODO
                2
            },
            // PUSH D
            0xd5 => {
                self.memory[self.sp - 1] = self.d;
                self.memory[self.sp - 2] = self.e;
                self.sp -= 2;
                1
            },
            // JC adr
            0xda => {
                if self.flags.carry {
                    self.jmp();
                    0
                } else {
                    3
                }
            },
            // IN D8
            0xdb => {
                //TODO
                2
            },
            // POP H
            0xe1 => {
                self.h = self.memory[self.sp + 1];
                self.l = self.memory[self.sp];
                self.sp += 2;
                1
            },
            // JPO adr
            0xe2 => {
                if !self.flags.even_parity {
                    self.jmp();
                    0
                } else {
                    3
                }
            },
            // PUSH H
            0xe5 => {
                self.memory[self.sp - 1] = self.h;
                self.memory[self.sp - 2] = self.l;
                self.sp -= 2;
                1
            },
            // ANI D8
            0xe6 => {
                self.and(self.memory[self.pc + 1]);
                2
            },
            // JPE adr
            0xea => {
                if self.flags.even_parity {
                    self.jmp();
                    0
                } else {
                    3
                }
            },
            // XCHG
            0xeb => {
                let d = self.d;
                let e = self.e;
                self.d = self.h;
                self.e = self.l;
                self.h = d;
                self.l = e;
                1
            },
            // POP PSW
            0xf1 => {
                self.a = self.memory[self.sp + 1];
                self.flags.set_psw(self.memory[self.sp]);
                self.sp += 2;
                1
            },
            // JP adr
            0xf2 => {
                if !self.flags.sign_negative {
                    self.jmp();
                    0
                } else {
                    3
                }
            },
            // DI
            0xf3 => {
                self.interrupts_enabled = false;
                1
            },
            // PUSH PSW
            0xf5 => {
                self.memory[self.sp - 1] = self.a;
                self.memory[self.sp - 2] = self.flags.psw();
                self.sp -= 2;
                1
            },
            // JM adr
            0xfa => {
                if self.flags.sign_negative {
                    self.jmp();
                    0
                } else {
                    3
                }
            },
            // EI
            0xfb => {
                self.interrupts_enabled = true;
                1
            },
            // CPI D8
            0xfe => {
                self.cmp(self.memory[self.pc + 1]);
                2
            },
            // Unimplemented
            _ => {
                println!("Unimplemented instruction: {:02x}", op_code);
                decode(&self.memory, self.pc);
                process::exit(0)
            },
        };
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

fn main() {
    let mut stdin = io::stdin();

    let rom = include_bytes!("invaders.rom");

    let mut state = State8080::new(rom);

    let mut i = 0;

    while state.pc < state.memory.len() {
        state.emulate();
        println!("{}", state);

        i += 1;
        println!("{} instructions done", i);

        print!("Next instruction: ");
        decode(&state.memory, state.pc);

        //stdin.read(&mut [0]);
    }
}

fn disassemble(buffer: &[u8]) {
    let mut pc = 0;

    while pc < buffer.len() {
        pc += decode(buffer, pc);
    }
}

fn decode(buffer: &[u8], pc: usize) -> usize {
    print!("{:04x}  {:02x}  ", pc, buffer[pc]);
    match buffer[pc] {
        0x00 => {
            println!("NOP");
            1
        },
        0x01 => {
            println!("LXI B,D16");
            3
        },
        0x02 => {
            println!("STAX B");
            1
        },
        0x03 => {
            println!("INX B");
            1
        },
        0x04 => {
            println!("INR B");
            1
        },
        0x05 => {
            println!("DCR B");
            1
        },
        0x06 => {
            println!("MVI B,D8");
            2
        },
        0x07 => {
            println!("RLC");
            1
        },
        0x08 => {
            println!("-");
            1
        },
        0x09 => {
            println!("DAD B");
            1
        },
        0x0a => {
            println!("LDAX B");
            1
        },
        0x0b => {
            println!("DCX B");
            1
        },
        0x0c => {
            println!("INR C");
            1
        },
        0x0d => {
            println!("DCR C");
            1
        },
        0x0e => {
            println!("MVI C,D8");
            2
        },
        0x0f => {
            println!("RRC");
            1
        },
        0x10 => {
            println!("-");
            1
        },
        0x11 => {
            println!("LXI D,D16");
            3
        },
        0x12 => {
            println!("STAX D");
            1
        },
        0x13 => {
            println!("INX D");
            1
        },
        0x14 => {
            println!("INR D");
            1
        },
        0x15 => {
            println!("DCR D");
            1
        },
        0x16 => {
            println!("MVI D,D8");
            2
        },
        0x17 => {
            println!("RAL");
            1
        },
        0x18 => {
            println!("-");
            1
        },
        0x19 => {
            println!("DAD D");
            1
        },
        0x1a => {
            println!("LDAX D");
            1
        },
        0x1b => {
            println!("DCX D");
            1
        },
        0x1c => {
            println!("INR E");
            1
        },
        0x1d => {
            println!("DCR E");
            1
        },
        0x1e => {
            println!("MVI E,D8");
            2
        },
        0x1f => {
            println!("RAR");
            1
        },
        0x20 => {
            println!("-");
            1
        },
        0x21 => {
            println!("LXI H,D16");
            3
        },
        0x22 => {
            println!("SHLD ${:02x}{:02x}", buffer[pc + 2], buffer[pc + 1]);
            3
        },
        0x23 => {
            println!("INX H");
            1
        },
        0x24 => {
            println!("INR H");
            1
        },
        0x25 => {
            println!("DCR H");
            1
        },
        0x26 => {
            println!("MVI H,D8");
            2
        },
        0x27 => {
            println!("DAA");
            1
        },
        0x28 => {
            println!("-");
            1
        },
        0x29 => {
            println!("DAD H");
            1
        },
        0x2a => {
            println!("LHLD ${:02x}{:02x}", buffer[pc + 2], buffer[pc + 1]);
            3
        },
        0x2b => {
            println!("DCX H");
            1
        },
        0x2c => {
            println!("INR L");
            1
        },
        0x2e => {
            println!("MVI L,D8");
            2
        },
        0x2f => {
            println!("CMA");
            1
        },
        0x30 => {
            println!("-");
            1
        },
        0x31 => {
            println!("LXI SP,D16");
            3
        },
        0x32 => {
            println!("STA ${:02x}{:02x}", buffer[pc + 2], buffer[pc + 1]);
            3
        },
        0x33 => {
            println!("INX SP");
            1
        },
        0x34 => {
            println!("INR M");
            1
        },
        0x35 => {
            println!("DCR M");
            1
        },
        0x36 => {
            println!("MVI M,D8");
            2
        },
        0x37 => {
            println!("STC");
            1
        },
        0x38 => {
            println!("-");
            1
        },
        0x39 => {
            println!("DAD SP");
            1
        },
        0x3a => {
            println!("LDA ${:02x}{:02x}", buffer[pc + 2], buffer[pc + 1]);
            3
        },
        0x3c => {
            println!("INR A");
            1
        },
        0x3d => {
            println!("DCR A");
            1
        },
        0x3e => {
            println!("MVI A,D8");
            2
        },
        0x3f => {
            println!("CMC");
            1
        },
        0x40 => {
            println!("MOV B,B");
            1
        },
        0x41 => {
            println!("MOV B,C");
            1
        },
        0x42 => {
            println!("MOV B,D");
            1
        },
        0x43 => {
            println!("MOV B,E");
            1
        },
        0x44 => {
            println!("MOV B,H");
            1
        },
        0x45 => {
            println!("MOV B,L");
            1
        },
        0x46 => {
            println!("MOV B,M");
            1
        },
        0x47 => {
            println!("MOV B,A");
            1
        },
        0x48 => {
            println!("MOV C,B");
            1
        },
        0x49 => {
            println!("MOV C,C");
            1
        },
        0x4a => {
            println!("MOV C,D");
            1
        },
        0x4b => {
            println!("MOV C,E");
            1
        },
        0x4c => {
            println!("MOV C,H");
            1
        },
        0x4d => {
            println!("MOV C,L");
            1
        },
        0x4e => {
            println!("MOV C,M");
            1
        },
        0x4f => {
            println!("MOV C,A");
            1
        },
        0x50 => {
            println!("MOV D,B");
            1
        },
        0x51 => {
            println!("MOV D,C");
            1
        },
        0x52 => {
            println!("MOV D,D");
            1
        },
        0x53 => {
            println!("MOV D,E");
            1
        },
        0x54 => {
            println!("MOV D,H");
            1
        },
        0x55 => {
            println!("MOV D,L");
            1
        },
        0x56 => {
            println!("MOV D,M");
            1
        },
        0x57 => {
            println!("MOV D,A");
            1
        },
        0x58 => {
            println!("MOV E,B");
            1
        },
        0x59 => {
            println!("MOV E,C");
            1
        },
        0x5a => {
            println!("MOV E,D");
            1
        },
        0x5b => {
            println!("MOV E,E");
            1
        },
        0x5c => {
            println!("MOV E,H");
            1
        },
        0x5d => {
            println!("MOV E,L");
            1
        },
        0x5e => {
            println!("MOV E,M");
            1
        },
        0x5f => {
            println!("MOV E,A");
            1
        },
        0x60 => {
            println!("MOV H,B");
            1
        },
        0x61 => {
            println!("MOV H,C");
            1
        },
        0x62 => {
            println!("MOV H,D");
            1
        },
        0x63 => {
            println!("MOV H,E");
            1
        },
        0x64 => {
            println!("MOV H,H");
            1
        },
        0x65 => {
            println!("MOV H,L");
            1
        },
        0x66 => {
            println!("MOV H,M");
            1
        },
        0x67 => {
            println!("MOV H,A");
            1
        },
        0x68 => {
            println!("MOV L,B");
            1
        },
        0x69 => {
            println!("MOV L,C");
            1
        },
        0x6a => {
            println!("MOV L,D");
            1
        },
        0x6b => {
            println!("MOV L,E");
            1
        },
        0x6c => {
            println!("MOV L,H");
            1
        },
        0x6d => {
            println!("MOV L,L");
            1
        },
        0x6e => {
            println!("MOV L,M");
            1
        },
        0x6f => {
            println!("MOV L,A");
            1
        },
        0x70 => {
            println!("MOV M,B");
            1
        },
        0x71 => {
            println!("MOV M,C");
            1
        },
        0x72 => {
            println!("MOV M,D");
            1
        },
        0x73 => {
            println!("MOV M,E");
            1
        },
        0x74 => {
            println!("MOV M,H");
            1
        },
        0x75 => {
            println!("MOV M,L");
            1
        },
        0x76 => {
            println!("HLT");
            1
        },
        0x77 => {
            println!("MOV M,A");
            1
        },
        0x78 => {
            println!("MOV A,B");
            1
        },
        0x79 => {
            println!("MOV A,C");
            1
        },
        0x7a => {
            println!("MOV A,D");
            1
        },
        0x7b => {
            println!("MOV A,E");
            1
        },
        0x7c => {
            println!("MOV A,H");
            1
        },
        0x7d => {
            println!("MOV A,L");
            1
        },
        0x7e => {
            println!("MOV A,M");
            1
        },
        0x7f => {
            println!("MOV A,A");
            1
        },
        0x80 => {
            println!("ADD B");
            1
        },
        0x81 => {
            println!("ADD C");
            1
        },
        0x82 => {
            println!("ADD D");
            1
        },
        0x83 => {
            println!("ADD E");
            1
        },
        0x84 => {
            println!("ADD H");
            1
        },
        0x85 => {
            println!("ADD L");
            1
        },
        0x86 => {
            println!("ADD M");
            1
        },
        0x87 => {
            println!("ADD A");
            1
        },
        0x88 => {
            println!("ADC B");
            1
        },
        0x89 => {
            println!("ADC C");
            1
        },
        0x8a => {
            println!("ADC D");
            1
        },
        0x8b => {
            println!("ADC E");
            1
        },
        0x8c => {
            println!("ADC H");
            1
        },
        0x8d => {
            println!("ADC L");
            1
        },
        0x8e => {
            println!("ADC M");
            1
        },
        0x8f => {
            println!("ADC A");
            1
        },
        0x90 => {
            println!("SUB B");
            1
        },
        0x91 => {
            println!("SUB C");
            1
        },
        0x92 => {
            println!("SUB D");
            1
        },
        0x93 => {
            println!("SUB E");
            1
        },
        0x94 => {
            println!("SUB H");
            1
        },
        0x95 => {
            println!("SUB L");
            1
        },
        0x96 => {
            println!("SUB M");
            1
        },
        0x97 => {
            println!("SUB A");
            1
        },
        0x98 => {
            println!("SBB B");
            1
        },
        0x99 => {
            println!("SBB C");
            1
        },
        0x9a => {
            println!("SBB D");
            1
        },
        0x9b => {
            println!("SBB E");
            1
        },
        0x9c => {
            println!("SBB H");
            1
        },
        0x9d => {
            println!("SBB L");
            1
        },
        0x9e => {
            println!("SBB M");
            1
        },
        0x9f => {
            println!("SBB A");
            1
        },
        0xa0 => {
            println!("ANA B");
            1
        },
        0xa1 => {
            println!("ANA C");
            1
        },
        0xa2 => {
            println!("ANA D");
            1
        },
        0xa3 => {
            println!("ANA E");
            1
        },
        0xa4 => {
            println!("ANA H");
            1
        },
        0xa5 => {
            println!("ANA L");
            1
        },
        0xa6 => {
            println!("ANA M");
            1
        },
        0xa7 => {
            println!("ANA A");
            1
        },
        0xa8 => {
            println!("XRA B");
            1
        },
        0xa9 => {
            println!("XRA C");
            1
        },
        0xaa => {
            println!("XRA D");
            1
        },
        0xab => {
            println!("XRA E");
            1
        },
        0xac => {
            println!("XRA H");
            1
        },
        0xad => {
            println!("XRA L");
            1
        },
        0xae => {
            println!("XRA M");
            1
        },
        0xaf => {
            println!("XRA A");
            1
        },
        0xb0 => {
            println!("ORA B");
            1
        },
        0xb1 => {
            println!("ORA C");
            1
        },
        0xb2 => {
            println!("ORA D");
            1
        },
        0xb3 => {
            println!("ORA E");
            1
        },
        0xb4 => {
            println!("ORA H");
            1
        },
        0xb5 => {
            println!("ORA L");
            1
        },
        0xb6 => {
            println!("ORA M");
            1
        },
        0xb7 => {
            println!("ORA A");
            1
        },
        0xb8 => {
            println!("CMP B");
            1
        },
        0xb9 => {
            println!("CMP C");
            1
        },
        0xba => {
            println!("CMP D");
            1
        },
        0xbb => {
            println!("CMP E");
            1
        },
        0xbc => {
            println!("CMP H");
            1
        },
        0xbd => {
            println!("CMP L");
            1
        },
        0xbe => {
            println!("CMP M");
            1
        },
        0xbf => {
            println!("CMP A");
            1
        },
        0xc0 => {
            println!("RNZ");
            1
        },
        0xc1 => {
            println!("POP B");
            1
        },
        0xc2 => {
            println!("JNZ ${:02x}{:02x}", buffer[pc + 2], buffer[pc + 1]);
            3
        },
        0xc3 => {
            println!("JMP ${:02x}{:02x}", buffer[pc + 2], buffer[pc + 1]);
            3
        },
        0xc4 => {
            println!("CNZ ${:02x}{:02x}", buffer[pc + 2], buffer[pc + 1]);
            3
        },
        0xc5 => {
            println!("PUSH B");
            1
        },
        0xc6 => {
            println!("ADI D8");
            2
        },
        0xc8 => {
            println!("RZ");
            1
        },
        0xca => {
            println!("JZ ${:02x}{:02x}", buffer[pc + 2], buffer[pc + 1]);
            3
        },
        0xcc => {
            println!("CZ ${:02x}{:02x}", buffer[pc + 2], buffer[pc + 1]);
            3
        },
        0xcd => {
            println!("CALL ${:02x}{:02x}", buffer[pc + 2], buffer[pc + 1]);
            3
        },
        0xc9 => {
            println!("RET");
            1
        },
        0xd0 => {
            println!("RNC");
            1
        },
        0xd1 => {
            println!("POP D");
            1
        },
        0xd2 => {
            println!("JNC ${:02x}{:02x}", buffer[pc + 2], buffer[pc + 1]);
            3
        },
        0xd3 => {
            println!("OUT D8");
            2
        },
        0xd4 => {
            println!("CNC ${:02x}{:02x}", buffer[pc + 2], buffer[pc + 1]);
            3
        },
        0xd5 => {
            println!("PUSH D");
            1
        },
        0xd6 => {
            println!("SUI D8");
            2
        },
        0xd8 => {
            println!("RC");
            1
        },
        0xda => {
            println!("JC ${:02x}{:02x}", buffer[pc + 2], buffer[pc + 1]);
            3
        },
        0xdb => {
            println!("IN D8");
            2
        },
        0xdc => {
            println!("CC ${:02x}{:02x}", buffer[pc + 2], buffer[pc + 1]);
            3
        },
        0xdd => {
            println!("-");
            1
        },
        0xde => {
            println!("SBI D8");
            2
        },
        0xe0 => {
            println!("RPO");
            1
        },
        0xe1 => {
            println!("POP H");
            1
        },
        0xe2 => {
            println!("JPO ${:02x}{:02x}", buffer[pc + 2], buffer[pc + 1]);
            3
        },
        0xe3 => {
            println!("XTHL");
            1
        },
        0xe4 => {
            println!("CPO ${:02x}{:02x}", buffer[pc + 2], buffer[pc + 1]);
            3
        },
        0xe5 => {
            println!("PUSH H");
            1
        },
        0xe6 => {
            println!("ANI D8");
            2
        },
        0xe9 => {
            println!("PCHL");
            1
        },
        0xeb => {
            println!("XCHG");
            2
        },
        0xec => {
            println!("CPE ${:02x}{:02x}", buffer[pc + 2], buffer[pc + 1]);
            3
        },
        0xee => {
            println!("XRI D8");
            2
        },
        0xf0 => {
            println!("RP");
            1
        },
        0xf1 => {
            println!("POP PSW");
            1
        },
        0xf5 => {
            println!("PUSH PSW");
            1
        },
        0xf6 => {
            println!("ORI D8");
            2
        },
        0xf7 => {
            println!("RST 6");
            1
        },
        0xf8 => {
            println!("RM");
            1
        },
        0xfa => {
            println!("JM ${:02x}{:02x}", buffer[pc + 2], buffer[pc + 1]);
            3
        },
        0xfb => {
            println!("EI");
            1
        },
        0xfc => {
            println!("CM ${:02x}{:02x}", buffer[pc + 2], buffer[pc + 1]);
            3
        },
        0xfe => {
            println!("CPI D8");
            2
        },
        0xff => {
            println!("RST 7");
            1
        },
        _ => {
            println!("Unknown opcode: {:02x}", buffer[pc]);
            process::exit(0)
        }
    }
}
