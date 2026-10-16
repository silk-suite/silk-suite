use crate::backends::nes::bus::BusLike;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AddressingMode {
  Implied,
  Immediate,
  ZeroPage,
  ZeroPageX,
  ZeroPageY,
  Relative,
  Absolute,
  AbsoluteX,
  AbsoluteY,
  Indirect,
  IndexedIndirect,
  IndirectIndexed,
}

#[derive(Default)]
pub struct Flags {
  /// The carry flag is set if the last operation caused an overflow
  /// from bit 7 of the result or an underflow from bit 0.
  pub carry: bool,
  /// The zero flag is set if the result of the last operation as was zero.
  pub zero: bool,
  /// The interrupt disable flag is set if the program has executed
  /// a 'Set Interrupt Disable' (SEI) instruction.
  pub interrupt_disable: bool,
  /// While the decimal mode flag is set the processor will obey the rules of
  /// Binary Coded Decimal (BCD) arithmetic during addition and subtraction.
  pub decimal_mode: bool,
  /// The break command bit is set when a BRK instruction has been executed
  /// and an interrupt has been generated to process it.
  pub break_command: bool,
  /// The overflow flag is set during arithmetic operations if the result has yielded an invalid 2's complement result
  /// (e.g. adding to positive numbers and ending up with a negative result: 64 + 64 => -128).
  pub overflow: bool,
  /// The negative flag is set if the result of the last operation had bit 7 set to a one.
  pub negative: bool,
}

impl Flags {
  pub fn to_u8(&self) -> u8 {
    (self.carry as u8) << 0 |
    (self.zero as u8) << 1 |
    (self.interrupt_disable as u8) << 2 |
    (self.decimal_mode as u8) << 3 |
    (self.break_command as u8) << 4 |
    1 << 5 |
    (self.overflow as u8) << 6 |
    (self.negative as u8) << 7
  }

  pub fn from_u8(byte: u8) -> Self {
    Self {
      carry: (byte & (1 << 0)) != 0,
      zero: (byte & (1 << 1)) != 0,
      interrupt_disable: (byte & (1 << 2)) != 0,
      decimal_mode: (byte & (1 << 3)) != 0,
      break_command: (byte & (1 << 4)) != 0,
      overflow: (byte & (1 << 6)) != 0,
      negative: (byte & (1 << 7)) != 0,
    }
  }
}

pub struct NES6502 {
  pub a: u8,
  pub x: u8,
  pub y: u8,
  pub sp: u8,
  pub pc: u16,
  pub flags: Flags,
  pub cycles: usize,
  pub fetched_data: u8,
  pub current_address_abs: u16,
  pub current_address_rel: u16,
  pub total_cycles: u32,
}

impl NES6502 {
  pub fn new() -> Self {
    Self {
      a: 0,
      x: 0,
      y: 0,
      sp: 0xFD,
      pc: 0,
      flags: Default::default(),
      cycles: 0,
      fetched_data: 0,
      current_address_abs: 0,
      current_address_rel: 0,
      total_cycles: 0,
    }
  }

  pub fn step(&mut self, bus: &mut dyn BusLike) {
    self.total_cycles += 1;
    if self.cycles == 0 {
      let opcode = self.read(bus, self.pc);
      //println!("PC: {:#04X}, opcode: {:02X}", self.pc, opcode);
      self.pc = self.pc.wrapping_add(1);

      match opcode {
        // ADC
        0x69 => self.adc(bus, AddressingMode::Immediate, 2),
        0x65 => self.adc(bus, AddressingMode::ZeroPage, 3),
        0x75 => self.adc(bus, AddressingMode::ZeroPageX, 4),
        0x6D => self.adc(bus, AddressingMode::Absolute, 4),
        0x7D => self.adc(bus, AddressingMode::AbsoluteX, 4),
        0x79 => self.adc(bus, AddressingMode::AbsoluteY, 4),
        0x61 => self.adc(bus, AddressingMode::IndexedIndirect, 6),
        0x71 => self.adc(bus, AddressingMode::IndirectIndexed, 5),
        // AND
        0x29 => self.and(bus, AddressingMode::Immediate, 2),
        0x25 => self.and(bus, AddressingMode::ZeroPage, 3),
        0x35 => self.and(bus, AddressingMode::ZeroPageX, 4),
        0x2D => self.and(bus, AddressingMode::Absolute, 4),
        0x3D => self.and(bus, AddressingMode::AbsoluteX, 4),
        0x39 => self.and(bus, AddressingMode::AbsoluteY, 4),
        0x21 => self.and(bus, AddressingMode::IndexedIndirect, 6),
        0x31 => self.and(bus, AddressingMode::IndirectIndexed, 5),
        // ASL
        0x0A => self.asl(bus, AddressingMode::Implied, 2),
        0x06 => self.asl(bus, AddressingMode::ZeroPage, 5),
        0x16 => self.asl(bus, AddressingMode::ZeroPageX, 6),
        0x0E => self.asl(bus, AddressingMode::Absolute, 6),
        0x1E => self.asl(bus, AddressingMode::AbsoluteX, 7),
        // BCC
        0x90 => self.bcc(bus, AddressingMode::Relative, 2),
        // BCS
        0xB0 => self.bcs(bus, AddressingMode::Relative, 2),
        // BEQ
        0xF0 => self.beq(bus, AddressingMode::Relative, 2),
        // BIT
        0x24 => self.bit(bus, AddressingMode::ZeroPage, 3),
        0x2C => self.bit(bus, AddressingMode::Absolute, 4),
        // BMI
        0x30 => self.bmi(bus, AddressingMode::Relative, 2),
        // BNE
        0xD0 => self.bne(bus, AddressingMode::Relative, 2),
        // BPL
        0x10 => self.bpl(bus, AddressingMode::Relative, 2),
        // BRK
        0x00 => self.brk(bus, AddressingMode::Implied, 7),
        // BVC
        0x50 => self.bvc(bus, AddressingMode::Relative, 2),
        // BVS
        0x70 => self.bvs(bus, AddressingMode::Relative, 2),
        // CLC
        0x18 => self.clc(bus, AddressingMode::Implied, 2),
        // CLD
        0xD8 => self.cld(bus, AddressingMode::Implied, 2),
        // CLI
        0x58 => self.cli(bus, AddressingMode::Implied, 2),
        // CLV
        0xB8 => self.clv(bus, AddressingMode::Implied, 2),
        // CMP
        0xC9 => self.cmp(bus, AddressingMode::Immediate, 2),
        0xC5 => self.cmp(bus, AddressingMode::ZeroPage, 3),
        0xD5 => self.cmp(bus, AddressingMode::ZeroPageX, 4),
        0xCD => self.cmp(bus, AddressingMode::Absolute, 4),
        0xDD => self.cmp(bus, AddressingMode::AbsoluteX, 4),
        0xD9 => self.cmp(bus, AddressingMode::AbsoluteY, 4),
        0xC1 => self.cmp(bus, AddressingMode::IndexedIndirect, 6),
        0xD1 => self.cmp(bus, AddressingMode::IndirectIndexed, 5),
        // CPX
        0xE0 => self.cpx(bus, AddressingMode::Immediate, 2),
        0xE4 => self.cpx(bus, AddressingMode::ZeroPage, 3),
        0xEC => self.cpx(bus, AddressingMode::Absolute, 4),
        // CPY
        0xC0 => self.cpy(bus, AddressingMode::Immediate, 2),
        0xC4 => self.cpy(bus, AddressingMode::ZeroPage, 3),
        0xCC => self.cpy(bus, AddressingMode::Absolute, 4),
        // DEC
        0xC6 => self.dec(bus, AddressingMode::ZeroPage, 5),
        0xD6 => self.dec(bus, AddressingMode::ZeroPageX, 6),
        0xCE => self.dec(bus, AddressingMode::Absolute, 6),
        0xDE => self.dec(bus, AddressingMode::AbsoluteX, 7),
        // DEX
        0xCA => self.dex(bus, AddressingMode::Implied, 2),
        // DEY
        0x88 => self.dey(bus, AddressingMode::Implied, 2),
        // EOR
        0x49 => self.eor(bus, AddressingMode::Immediate, 2),
        0x45 => self.eor(bus, AddressingMode::ZeroPage, 3),
        0x55 => self.eor(bus, AddressingMode::ZeroPageX, 4),
        0x4D => self.eor(bus, AddressingMode::Absolute, 4),
        0x5D => self.eor(bus, AddressingMode::AbsoluteX, 4),
        0x59 => self.eor(bus, AddressingMode::AbsoluteY, 4),
        0x41 => self.eor(bus, AddressingMode::IndexedIndirect, 6),
        0x51 => self.eor(bus, AddressingMode::IndirectIndexed, 5),
        // INC
        0xE6 => self.inc(bus, AddressingMode::ZeroPage, 5),
        0xF6 => self.inc(bus, AddressingMode::ZeroPageX, 6),
        0xEE => self.inc(bus, AddressingMode::Absolute, 6),
        0xFE => self.inc(bus, AddressingMode::AbsoluteX, 7),
        // INX
        0xE8 => self.inx(bus, AddressingMode::Implied, 2),
        // INY
        0xC8 => self.iny(bus, AddressingMode::Implied, 2),
        // JMP
        0x4C => self.jmp(bus, AddressingMode::Absolute, 3),
        0x6C => self.jmp(bus, AddressingMode::Indirect, 5),
        // JSR
        0x20 => self.jsr(bus, AddressingMode::Absolute, 6),
        // LDA
        0xA9 => self.lda(bus, AddressingMode::Immediate, 2),
        0xA5 => self.lda(bus, AddressingMode::ZeroPage, 3),
        0xB5 => self.lda(bus, AddressingMode::ZeroPageX, 4),
        0xAD => self.lda(bus, AddressingMode::Absolute, 4),
        0xBD => self.lda(bus, AddressingMode::AbsoluteX, 4),
        0xB9 => self.lda(bus, AddressingMode::AbsoluteY, 4),
        0xA1 => self.lda(bus, AddressingMode::IndexedIndirect, 6),
        0xB1 => self.lda(bus, AddressingMode::IndirectIndexed, 5),
        // LDX
        0xA2 => self.ldx(bus, AddressingMode::Immediate, 2),
        0xA6 => self.ldx(bus, AddressingMode::ZeroPage, 3),
        0xB6 => self.ldx(bus, AddressingMode::ZeroPageY, 4),
        0xAE => self.ldx(bus, AddressingMode::Absolute, 4),
        0xBE => self.ldx(bus, AddressingMode::AbsoluteY, 4),
        // LDY
        0xA0 => self.ldy(bus, AddressingMode::Immediate, 2),
        0xA4 => self.ldy(bus, AddressingMode::ZeroPage, 3),
        0xB4 => self.ldy(bus, AddressingMode::ZeroPageX, 4),
        0xAC => self.ldy(bus, AddressingMode::Absolute, 4),
        0xBC => self.ldy(bus, AddressingMode::AbsoluteX, 4),
        // LSR
        0x4A => self.lsr(bus, AddressingMode::Implied, 2),
        0x46 => self.lsr(bus, AddressingMode::ZeroPage, 5),
        0x56 => self.lsr(bus, AddressingMode::ZeroPageX, 6),
        0x4E => self.lsr(bus, AddressingMode::Absolute, 6),
        0x5E => self.lsr(bus, AddressingMode::AbsoluteX, 7),
        // NOP
        0xEA => self.nop(bus, AddressingMode::Implied, 2),
        // ORA
        0x09 => self.ora(bus, AddressingMode::Immediate, 2),
        0x05 => self.ora(bus, AddressingMode::ZeroPage, 3),
        0x15 => self.ora(bus, AddressingMode::ZeroPageX, 4),
        0x0D => self.ora(bus, AddressingMode::Absolute, 4),
        0x1D => self.ora(bus, AddressingMode::AbsoluteX, 4),
        0x19 => self.ora(bus, AddressingMode::AbsoluteY, 4),
        0x01 => self.ora(bus, AddressingMode::IndexedIndirect, 6),
        0x11 => self.ora(bus, AddressingMode::IndirectIndexed, 5),
        // PHA
        0x48 => self.pha(bus, AddressingMode::Implied, 3),
        // PHP
        0x08 => self.php(bus, AddressingMode::Implied, 3),
        // PLA
        0x68 => self.pla(bus, AddressingMode::Implied, 4),
        // PLP
        0x28 => self.plp(bus, AddressingMode::Implied, 4),
        // ROL
        0x2A => self.rol(bus, AddressingMode::Implied, 2),
        0x26 => self.rol(bus, AddressingMode::ZeroPage, 5),
        0x36 => self.rol(bus, AddressingMode::ZeroPageX, 6),
        0x2E => self.rol(bus, AddressingMode::Absolute, 6),
        0x3E => self.rol(bus, AddressingMode::AbsoluteX, 7),
        // ROR
        0x6A => self.ror(bus, AddressingMode::Implied, 2),
        0x66 => self.ror(bus, AddressingMode::ZeroPage, 5),
        0x76 => self.ror(bus, AddressingMode::ZeroPageX, 6),
        0x6E => self.ror(bus, AddressingMode::Absolute, 6),
        0x7E => self.ror(bus, AddressingMode::AbsoluteX, 7),
        // RTI
        0x40 => self.rti(bus, AddressingMode::Implied, 6),
        // RTS
        0x60 => self.rts(bus, AddressingMode::Implied, 6),
        // SBC
        0xE9 => self.sbc(bus, AddressingMode::Immediate, 2),
        0xE5 => self.sbc(bus, AddressingMode::ZeroPage, 3),
        0xF5 => self.sbc(bus, AddressingMode::ZeroPageX, 4),
        0xED => self.sbc(bus, AddressingMode::Absolute, 4),
        0xFD => self.sbc(bus, AddressingMode::AbsoluteX, 4),
        0xF9 => self.sbc(bus, AddressingMode::AbsoluteY, 4),
        0xE1 => self.sbc(bus, AddressingMode::IndexedIndirect, 6),
        0xF1 => self.sbc(bus, AddressingMode::IndirectIndexed, 5),
        // SEC
        0x38 => self.sec(bus, AddressingMode::Implied, 2),
        // SED
        0xF8 => self.sed(bus, AddressingMode::Implied, 2),
        // SEI
        0x78 => self.sei(bus, AddressingMode::Implied, 2),
        // STA
        0x85 => self.sta(bus, AddressingMode::ZeroPage, 3),
        0x95 => self.sta(bus, AddressingMode::ZeroPageX, 4),
        0x8D => self.sta(bus, AddressingMode::Absolute, 4),
        0x9D => self.sta(bus, AddressingMode::AbsoluteX, 5),
        0x99 => self.sta(bus, AddressingMode::AbsoluteY, 5),
        0x81 => self.sta(bus, AddressingMode::IndexedIndirect, 6),
        0x91 => self.sta(bus, AddressingMode::IndirectIndexed, 6),
        // STX
        0x86 => self.stx(bus, AddressingMode::ZeroPage, 3),
        0x96 => self.stx(bus, AddressingMode::ZeroPageY, 4),
        0x8E => self.stx(bus, AddressingMode::Absolute, 4),
        // STY
        0x84 => self.sty(bus, AddressingMode::ZeroPage, 3),
        0x94 => self.sty(bus, AddressingMode::ZeroPageX, 4),
        0x8C => self.sty(bus, AddressingMode::Absolute, 4),
        // TAX
        0xAA => self.tax(bus, AddressingMode::Implied, 2),
        // TAY
        0xA8 => self.tay(bus, AddressingMode::Implied, 2),
        // TSX
        0xBA => self.tsx(bus, AddressingMode::Implied, 2),
        // TXA
        0x8A => self.txa(bus, AddressingMode::Implied, 2),
        // TXS
        0x9A => self.txs(bus, AddressingMode::Implied, 2),
        // TYA
        0x98 => self.tya(bus, AddressingMode::Implied, 2),
        // Any other opcode gets caught here
        _ => {
          println!("Invalid opcode: {:02X} at PC: {:04X}", opcode, self.pc);
          self.cycles = 1;
        },
      }
    }

    self.cycles -= 1;
  }

  pub fn read(&self, bus: &mut dyn BusLike, address: u16) -> u8 {
    bus.cpu_read(address)
  }

  pub fn write(&mut self, bus: &mut dyn BusLike, address: u16, value: u8) {
    bus.cpu_write(address, value);
  }

  fn fetch(&mut self, bus: &mut dyn BusLike, mode: AddressingMode, requires_data: bool, add_cycle_for_page_cross: bool) {
    match mode {
      // Data has an implicit source, potentially the accumulator
      AddressingMode::Implied => {
        self.fetched_data = self.a;
      },
      // The data is immediately available in the following byte
      AddressingMode::Immediate => {
        self.current_address_abs = self.pc;
        self.pc = self.pc.wrapping_add(1);
      },
      // Addressing 0x0000 to 0x00FF only
      AddressingMode::ZeroPage => {
        self.current_address_abs = self.read(bus, self.pc) as u16;
        self.pc = self.pc.wrapping_add(1);
        self.current_address_abs &= 0x00FF;
      },
      // Index into the zero page with X offset
      AddressingMode::ZeroPageX => {
        self.current_address_abs = (self.read(bus, self.pc).wrapping_add(self.x)) as u16 % 0xFFFF;
        self.pc = self.pc.wrapping_add(1);
        self.current_address_abs &= 0x00FF;
      },
      // Index into the zero page with Y offset
      AddressingMode::ZeroPageY => {
        self.current_address_abs = (self.read(bus, self.pc).wrapping_add(self.y)) as u16;
        self.pc = self.pc.wrapping_add(1);
        self.current_address_abs &= 0x00FF;
      },
      AddressingMode::Relative => {
        self.current_address_rel = self.read(bus, self.pc) as u16;
        self.pc = self.pc.wrapping_add(1);

        // Check if relative address is negative
        if self.current_address_rel & 0x80 != 0 {
          self.current_address_rel |= 0xFF00
        }
      },
      // Read the next two bytes as a 16-bit address
      AddressingMode::Absolute => {
        let low = self.read(bus, self.pc) as u16;
        self.pc = self.pc.wrapping_add(1);
        let high = self.read(bus, self.pc) as u16;
        self.pc = self.pc.wrapping_add(1);

        self.current_address_abs = (high << 8) | low;
      },
      // Read the next two bytes as a 16-bit address, and add X offset
      AddressingMode::AbsoluteX => {
        let low = self.read(bus, self.pc) as u16;
        self.pc = self.pc.wrapping_add(1);
        let high = self.read(bus, self.pc) as u16;
        self.pc = self.pc.wrapping_add(1);

        self.current_address_abs = (high << 8) | low;
        self.current_address_abs = self.current_address_abs.wrapping_add(self.x as u16);

        if add_cycle_for_page_cross && (self.current_address_abs & 0xFF00) != (high << 8) {
          // Crossed page boundary, add an additional clock cycle
          self.cycles += 1;
        }
      },
      // Read the next two bytes as a 16-bit address, and add Y offset
      AddressingMode::AbsoluteY => {
        let low = self.read(bus, self.pc) as u16;
        self.pc = self.pc.wrapping_add(1);
        let high = self.read(bus, self.pc) as u16;
        self.pc = self.pc.wrapping_add(1);

        self.current_address_abs = (high << 8) | low;
        self.current_address_abs = self.current_address_abs.wrapping_add(self.y as u16);

        if add_cycle_for_page_cross && (self.current_address_abs & 0xFF00) != (high << 8) {
          // Crossed page boundary, add an additional clock cycle
          self.cycles += 1;
        }
      },
      AddressingMode::Indirect => {
        let ptr_low = self.read(bus, self.pc) as u16;
        self.pc = self.pc.wrapping_add(1);
        let ptr_high = self.read(bus, self.pc) as u16;
        self.pc = self.pc.wrapping_add(1);

        let ptr = (ptr_high << 8) | ptr_low;

        if ptr_low == 0x00FF {
          // Simulates hardware page boundary bug
          self.current_address_abs = (self.read(bus, ptr & 0xFF00) as u16) << 8 | self.read(bus, ptr) as u16;
        } else {
          self.current_address_abs = (((self.read(bus, ptr + 1) as u16) << 8) | self.read(bus, ptr) as u16) as u16;
        }
      },
      // Index into address table on the zero page and offset by X
      // val = PEEK(PEEK((arg + X) % 256) + PEEK((arg + X + 1) % 256) * 256)
      AddressingMode::IndexedIndirect => {
        let operand = self.read(bus, self.pc) as u16;
        self.pc = self.pc.wrapping_add(1);

        let low = self.read(bus, (operand.wrapping_add(self.x as u16)) & 0xFF) as u16;
        let high = self.read(bus, (operand.wrapping_add(self.x as u16 + 1)) & 0xFF) as u16;

        self.current_address_abs = (high << 8) | low;
      },
      // Index into the zero page, read 16-bit address, and add Y offset to it
      // val = PEEK(PEEK(arg) + PEEK((arg + 1) % 256) * 256 + Y)
      AddressingMode::IndirectIndexed => {
        let table = self.read(bus, self.pc) as u16;
        self.pc = self.pc.wrapping_add(1);

        let low = self.read(bus, (table as u16) & 0x00FF) as u16;
        let high = self.read(bus, (table.wrapping_add(1)) as u16 & 0x00FF) as u16;

        self.current_address_abs = (high << 8) | low;
        self.current_address_abs = self.current_address_abs.wrapping_add(self.y as u16);

        if add_cycle_for_page_cross && (self.current_address_abs & 0xFF00) != (high << 8) {
          // Crossed page boundary, add an additional clock cycle
          self.cycles += 1;
        }
      },
    }

    if mode != AddressingMode::Implied && requires_data {
      self.fetched_data = self.read(bus, self.current_address_abs);
    }
  }

  // region: Instructions

  /// Add with carry
  fn adc(&mut self, bus: &mut dyn BusLike, mode: AddressingMode, initial_cycle_count: usize) {
    self.cycles += initial_cycle_count;
    self.fetch(bus, mode, true, true);

    let temp = self.a as u16 + self.fetched_data as u16 + self.flags.carry as u16;
    self.flags.carry = temp > 255;
    self.flags.zero = (temp & 0x00FF) == 0;
    self.flags.negative = temp & 0x80 != 0;
    self.flags.overflow = (!(self.a as u16 ^ self.fetched_data as u16) & (self.a as u16 ^ temp)) & 0x0080 != 0;

    self.a = (temp & 0x00FF) as u8;
  }

  /// Logical AND accumulator with given data
  fn and(&mut self, bus: &mut dyn BusLike, mode: AddressingMode, initial_cycle_count: usize) {
    self.cycles += initial_cycle_count;
    self.fetch(bus, mode, true, true);

    self.a &= self.fetched_data;

    self.flags.zero = self.a == 0;
    self.flags.negative = self.a & 0x80 != 0;
  }

  /// Arithmetic shift left
  fn asl(&mut self, bus: &mut dyn BusLike, mode: AddressingMode, initial_cycle_count: usize) {
    self.cycles += initial_cycle_count;
    self.fetch(bus, mode, true, false);

    let value = (self.fetched_data as u16) << 1;

    self.flags.carry = value & 0xFF00 != 0;
    self.flags.zero = value & 0x00FF == 0;
    self.flags.negative = value & 0x80 != 0;

    if mode == AddressingMode::Implied {
      self.a = (value & 0x00FF) as u8;
    } else {
      self.write(bus, self.current_address_abs, (value & 0x00FF) as u8);
    }
  }

  /// Branch if carry flag is clear
  fn bcc(&mut self, bus: &mut dyn BusLike, mode: AddressingMode, initial_cycle_count: usize) {
    self.cycles += initial_cycle_count;
    self.fetch(bus, mode, false, false);

    if !self.flags.carry {
      self.cycles += 1;
      self.current_address_abs = self.pc.wrapping_add(self.current_address_rel);

      if (self.current_address_abs & 0xFF00) != (self.pc & 0xFF00) {
        // Crossed page boundary, add an additional clock cycle
        self.cycles += 1;
      }

      self.pc = self.current_address_abs;
    }
  }

  /// Branch if carry flag is set
  fn bcs(&mut self, bus: &mut dyn BusLike, mode: AddressingMode, initial_cycle_count: usize) {
    self.cycles += initial_cycle_count;
    self.fetch(bus, mode, false, false);

    if self.flags.carry {
      self.cycles += 1;
      self.current_address_abs = self.pc.wrapping_add(self.current_address_rel);

      if (self.current_address_abs & 0xFF00) != (self.pc & 0xFF00) {
        // Crossed page boundary, add an additional clock cycle
        self.cycles += 1;
      }

      self.pc = self.current_address_abs;
    }
  }

  /// Branch if zero flag is set
  fn beq(&mut self, bus: &mut dyn BusLike, mode: AddressingMode, initial_cycle_count: usize) {
    self.cycles += initial_cycle_count;
    self.fetch(bus, mode, false, false);

    if self.flags.zero {
      self.cycles += 1;
      self.current_address_abs = self.pc.wrapping_add(self.current_address_rel);

      if (self.current_address_abs & 0xFF00) != (self.pc & 0xFF00) {
        // Crossed page boundary, add an additional clock cycle
        self.cycles += 1;
      }

      self.pc = self.current_address_abs;
    }
  }

  /// AND the contents of A with the value in memory and check if bits are set
  fn bit(&mut self, bus: &mut dyn BusLike, mode: AddressingMode, initial_cycle_count: usize) {
    self.cycles += initial_cycle_count;
    self.fetch(bus, mode, true, true);

    let temp = self.a & self.fetched_data;

    self.flags.zero = (temp & 0x00FF) == 0;
    self.flags.overflow = self.fetched_data & (1 << 6) != 0;
    self.flags.negative = self.fetched_data & (1 << 7) != 0;
  }

  /// Branch if negative flag is set
  fn bmi(&mut self, bus: &mut dyn BusLike, mode: AddressingMode, initial_cycle_count: usize) {
    self.cycles += initial_cycle_count;
    self.fetch(bus, mode, false, false);

    if self.flags.negative {
      self.cycles += 1;
      self.current_address_abs = self.pc.wrapping_add(self.current_address_rel);

      if (self.current_address_abs & 0xFF00) != (self.pc & 0xFF00) {
        // Crossed page boundary, add an additional clock cycle
        self.cycles += 1;
      }

      self.pc = self.current_address_abs;
    }
  }

  /// Branch if zero flag is clear
  fn bne(&mut self, bus: &mut dyn BusLike, mode: AddressingMode, initial_cycle_count: usize) {
    self.cycles += initial_cycle_count;
    self.fetch(bus, mode, false, false);

    if !self.flags.zero {
      self.cycles += 1;
      self.current_address_abs = self.pc.wrapping_add(self.current_address_rel);

      if (self.current_address_abs & 0xFF00) != (self.pc & 0xFF00) {
        // Crossed page boundary, add an additional clock cycle
        self.cycles += 1;
      }

      self.pc = self.current_address_abs;
    }
  }

  /// Branch if negative flag is clear
  fn bpl(&mut self, bus: &mut dyn BusLike, mode: AddressingMode, initial_cycle_count: usize) {
    self.cycles += initial_cycle_count;
    self.fetch(bus, mode, false, false);

    if !self.flags.negative {
      self.cycles += 1;
      self.current_address_abs = self.pc.wrapping_add(self.current_address_rel);

      if (self.current_address_abs & 0xFF00) != (self.pc & 0xFF00) {
        // Crossed page boundary, add an additional clock cycle
        self.cycles += 1;
      }

      self.pc = self.current_address_abs;
    }
  }

  /// Forces the generation of an interrupt request
  fn brk(&mut self, bus: &mut dyn BusLike, mode: AddressingMode, initial_cycle_count: usize) {
    self.cycles += initial_cycle_count;
    self.fetch(bus, mode, false, false);

    self.pc += 1;

    // Push the program counter onto the stack
    self.write(bus, 0x0100 + self.sp as u16, (self.pc >> 8) as u8 & 0x00FF);
    self.sp = self.sp.wrapping_sub(1);
    self.write(bus, 0x0100 + self.sp as u16, (self.pc & 0x00FF) as u8);
    self.sp = self.sp.wrapping_sub(1);

    // Write the status flags onto the stack
    self.flags.break_command = true;
    self.write(bus, 0x0100 + self.sp as u16, self.flags.to_u8());
    self.sp = self.sp.wrapping_sub(1);
    self.flags.break_command = false;

    self.flags.interrupt_disable = true;

    self.pc = self.read(bus, 0xFFFE) as u16 | ((self.read(bus, 0xFFFF) as u16) << 8) as u16;
  }

  /// Branch if overflow flag is clear
  fn bvc(&mut self, bus: &mut dyn BusLike, mode: AddressingMode, initial_cycle_count: usize) {
    self.cycles += initial_cycle_count;
    self.fetch(bus, mode, false, false);

    if !self.flags.overflow {
      self.cycles += 1;
      self.current_address_abs = self.pc.wrapping_add(self.current_address_rel);

      if (self.current_address_abs & 0xFF00) != (self.pc & 0xFF00) {
        // Crossed page boundary, add an additional clock cycle
        self.cycles += 1;
      }

      self.pc = self.current_address_abs;
    }
  }

  /// Branch if overflow flag is set
  fn bvs(&mut self, bus: &mut dyn BusLike, mode: AddressingMode, initial_cycle_count: usize) {
    self.cycles += initial_cycle_count;
    self.fetch(bus, mode, false, false);

    if self.flags.overflow {
      self.cycles += 1;
      self.current_address_abs = self.pc.wrapping_add(self.current_address_rel);

      if (self.current_address_abs & 0xFF00) != (self.pc & 0xFF00) {
        // Crossed page boundary, add an additional clock cycle
        self.cycles += 1;
      }

      self.pc = self.current_address_abs;
    }
  }

  /// Clear carry flag
  fn clc(&mut self, bus: &mut dyn BusLike, mode: AddressingMode, initial_cycle_count: usize) {
    self.cycles += initial_cycle_count;
    self.fetch(bus, mode, false, false);

    self.flags.carry = false;
  }

  /// Clear decimal mode
  fn cld(&mut self, bus: &mut dyn BusLike, mode: AddressingMode, initial_cycle_count: usize) {
    self.cycles += initial_cycle_count;
    self.fetch(bus, mode, false, false);

    self.flags.decimal_mode = false;
  }

  /// Clear interrupt disable flag
  fn cli(&mut self, bus: &mut dyn BusLike, mode: AddressingMode, initial_cycle_count: usize) {
    self.cycles += initial_cycle_count;
    self.fetch(bus, mode, false, false);

    self.flags.interrupt_disable = false;
  }

  /// Clear overflow flag
  fn clv(&mut self, bus: &mut dyn BusLike, mode: AddressingMode, initial_cycle_count: usize) {
    self.cycles += initial_cycle_count;
    self.fetch(bus, mode, false, false);

    self.flags.overflow = false;
  }

  /// Compare the contents of the accumulator with another value in memory
  fn cmp(&mut self, bus: &mut dyn BusLike, mode: AddressingMode, initial_cycle_count: usize) {
    self.cycles += initial_cycle_count;
    self.fetch(bus, mode, true, true);

    let temp = self.a.wrapping_sub(self.fetched_data);

    self.flags.carry = self.a >= self.fetched_data;
    self.flags.zero = (temp & 0x00FF) == 0;
    self.flags.negative = temp & 0x80 != 0;
  }

  /// Compare the contents of the X register with another value in memory
  fn cpx(&mut self, bus: &mut dyn BusLike, mode: AddressingMode, initial_cycle_count: usize) {
    self.cycles += initial_cycle_count;
    self.fetch(bus, mode, true, true);

    let temp = self.x.wrapping_sub(self.fetched_data);

    self.flags.carry = self.x >= self.fetched_data;
    self.flags.zero = (temp & 0x00FF) == 0;
    self.flags.negative = temp & 0x80 != 0;
  }

  /// Compare the contents of the Y register with another value in memory
  fn cpy(&mut self, bus: &mut dyn BusLike, mode: AddressingMode, initial_cycle_count: usize) {
    self.cycles += initial_cycle_count;
    self.fetch(bus, mode, true, true);

    let temp = self.y.wrapping_sub(self.fetched_data);

    self.flags.carry = self.y >= self.fetched_data;
    self.flags.zero = (temp & 0x00FF) == 0;
    self.flags.negative = temp & 0x80 != 0;
  }

  /// Decrement value stored at memory address by 1
  fn dec(&mut self, bus: &mut dyn BusLike, mode: AddressingMode, initial_cycle_count: usize) {
    self.cycles += initial_cycle_count;
    self.fetch(bus, mode, false, false);

    // Make this better later
    let mut value = self.read(bus, self.current_address_abs);
    self.write(bus, self.current_address_abs, value.wrapping_sub(1) & 0x00FF);
    value = self.read(bus, self.current_address_abs);

    self.flags.zero = (value & 0x00FF) == 0;
    self.flags.negative = (value & 0x80) != 0;
  }

  /// Decrement X register by 1
  fn dex(&mut self, bus: &mut dyn BusLike, mode: AddressingMode, initial_cycle_count: usize) {
    self.cycles += initial_cycle_count;
    self.fetch(bus, mode, false, false);

    self.x = self.x.wrapping_sub(1);

    self.flags.zero = self.x == 0;
    self.flags.negative = (self.x & 0x80) != 0;
  }

  /// Decrement Y register by 1
  fn dey(&mut self, bus: &mut dyn BusLike, mode: AddressingMode, initial_cycle_count: usize) {
    self.cycles += initial_cycle_count;
    self.fetch(bus, mode, false, false);

    self.y = self.y.wrapping_sub(1);

    self.flags.zero = self.y == 0;
    self.flags.negative = (self.y & 0x80) != 0;
  }

  /// Logical XOR accummulator with given value
  fn eor(&mut self, bus: &mut dyn BusLike, mode: AddressingMode, initial_cycle_count: usize) {
    self.cycles += initial_cycle_count;
    self.fetch(bus, mode, true, true);

    self.a ^= self.fetched_data;

    self.flags.zero = self.a == 0;
    self.flags.negative = (self.a & 0x80) != 0;
  }

  /// Increment value stored at memory address by 1
  fn inc(&mut self, bus: &mut dyn BusLike, mode: AddressingMode, initial_cycle_count: usize) {
    self.cycles += initial_cycle_count;
    self.fetch(bus, mode, false, false);

    // Make this better later
    let mut value = self.read(bus, self.current_address_abs);
    self.write(bus, self.current_address_abs, value.wrapping_add(1));
    value = self.read(bus, self.current_address_abs);

    self.flags.zero = value == 0;
    self.flags.negative = (value & 0x80) != 0;
  }

  /// Increment X register by 1
  fn inx(&mut self, bus: &mut dyn BusLike, mode: AddressingMode, initial_cycle_count: usize) {
    self.cycles += initial_cycle_count;
    self.fetch(bus, mode, false, false);

    self.x = self.x.wrapping_add(1);

    self.flags.zero = self.x == 0;
    self.flags.negative = (self.x & 0x80) != 0;
  }

  /// Increment Y register by 1
  fn iny(&mut self, bus: &mut dyn BusLike, mode: AddressingMode, initial_cycle_count: usize) {
    self.cycles += initial_cycle_count;
    self.fetch(bus, mode, false, false);

    self.y = self.y.wrapping_add(1);

    self.flags.zero = self.y == 0;
    self.flags.negative = (self.y & 0x80) != 0;
  }

  /// Set the program counter to the given address
  fn jmp(&mut self, bus: &mut dyn BusLike, mode: AddressingMode, initial_cycle_count: usize) {
    self.cycles += initial_cycle_count;
    self.fetch(bus, mode, false, false);

    self.pc = self.current_address_abs;
  }

  // Push the current program counter to the stack, then jump to the given address
  fn jsr(&mut self, bus: &mut dyn BusLike, mode: AddressingMode, initial_cycle_count: usize) {
    self.cycles += initial_cycle_count;
    self.fetch(bus, mode, false, false);

    self.pc = self.pc.wrapping_sub(1);

    self.write(bus, 0x0100 + self.sp as u16, (self.pc >> 8) as u8 & 0x00FF);
    self.sp = self.sp.wrapping_sub(1);
    self.write(bus, 0x0100 + self.sp as u16, self.pc as u8 & 0x00FF);
    self.sp = self.sp.wrapping_sub(1);

    self.pc = self.current_address_abs;
  }

  /// Load a byte of memory into the accumulator
  fn lda(&mut self, bus: &mut dyn BusLike, mode: AddressingMode, initial_cycle_count: usize) {
    self.cycles += initial_cycle_count;
    self.fetch(bus, mode, true, true);

    self.a = self.fetched_data;

    self.flags.zero = self.a == 0;
    self.flags.negative = self.a & 0x80 != 0;
  }

  /// Load a byte of memory into the X register
  fn ldx(&mut self, bus: &mut dyn BusLike, mode: AddressingMode, initial_cycle_count: usize) {
    self.cycles += initial_cycle_count;
    self.fetch(bus, mode, true, true);

    self.x = self.fetched_data;

    self.flags.zero = self.x == 0;
    self.flags.negative = self.x & 0x80 != 0;
  }

  /// Load a byte of memory into the Y register
  fn ldy(&mut self, bus: &mut dyn BusLike, mode: AddressingMode, initial_cycle_count: usize) {
    self.cycles += initial_cycle_count;
    self.fetch(bus, mode, true, true);

    self.y = self.fetched_data;

    self.flags.zero = self.y == 0;
    self.flags.negative = self.y & 0x80 != 0;
  }

  /// Logical shift right
  fn lsr(&mut self, bus: &mut dyn BusLike, mode: AddressingMode, initial_cycle_count: usize) {
    self.cycles += initial_cycle_count;
    self.fetch(bus, mode, true, false);

    let original_value = self.fetched_data as u16;
    let value = (original_value >> 1) as u8;

    self.flags.carry = (original_value & 0x01) != 0;
    self.flags.zero = (value & 0x00FF) == 0;
    self.flags.negative = (value & 0x80) != 0;

    if mode == AddressingMode::Implied {
      self.a = (value & 0x00FF) as u8;
    } else {
      self.write(bus, self.current_address_abs, (value & 0x00FF) as u8);
    }
  }

  /// No op
  fn nop(&mut self, bus: &mut dyn BusLike, mode: AddressingMode, initial_cycle_count: usize) {
    self.cycles += initial_cycle_count;
    self.fetch(bus, mode, false, false);
  }

  /// Logical OR the accumulator with a byte of memory
  fn ora(&mut self, bus: &mut dyn BusLike, mode: AddressingMode, initial_cycle_count: usize) {
    self.cycles += initial_cycle_count;
    self.fetch(bus, mode, true, true);

    self.a |= self.fetched_data;

    self.flags.zero = self.a == 0;
    self.flags.negative = self.a & 0x80 != 0;
  }

  /// Pushes a copy of the accumulator on to the stack.
  fn pha(&mut self, bus: &mut dyn BusLike, mode: AddressingMode, initial_cycle_count: usize) {
    self.cycles += initial_cycle_count;
    self.fetch(bus, mode, false, false);

    self.write(bus, 0x0100 + self.sp as u16, self.a);
    self.sp = self.sp.wrapping_sub(1);
  }

  /// Pushes a copy of the status flags on to the stack.
  fn php(&mut self, bus: &mut dyn BusLike, mode: AddressingMode, initial_cycle_count: usize) {
    self.cycles += initial_cycle_count;
    self.fetch(bus, mode, false, false);

    self.flags.break_command = true;
    self.write(bus, 0x0100 + self.sp as u16, self.flags.to_u8());
    self.flags.break_command = false;
    self.sp = self.sp.wrapping_sub(1);
  }

  /// Pulls an 8 bit value from the stack and into the accumulator.
  fn pla(&mut self, bus: &mut dyn BusLike, mode: AddressingMode, initial_cycle_count: usize) {
    self.cycles += initial_cycle_count;
    self.fetch(bus, mode, false, false);

    self.sp = self.sp.wrapping_add(1);
    self.a = self.read(bus, 0x0100 + self.sp as u16);

    self.flags.zero = self.a == 0;
    self.flags.negative = self.a & 0x80 != 0;
  }

  /// Pulls an 8 bit value from the stack and into the processor flags.
  fn plp(&mut self, bus: &mut dyn BusLike, mode: AddressingMode, initial_cycle_count: usize) {
    self.cycles += initial_cycle_count;
    self.fetch(bus, mode, false, false);

    self.sp = self.sp.wrapping_add(1);
    self.flags = Flags::from_u8(self.read(bus, 0x0100 + self.sp as u16));
    self.flags.break_command = false;
  }

  /// Move each of the bits in either A or M one place to the left.
  fn rol(&mut self, bus: &mut dyn BusLike, mode: AddressingMode, initial_cycle_count: usize) {
    self.cycles += initial_cycle_count;
    self.fetch(bus, mode, true, false);

    let value = (((self.fetched_data as u16) << 1) | self.flags.carry as u16) as u16;

    self.flags.carry = (value & 0xFF00) != 0;
    self.flags.zero = (value & 0x00FF) == 0;
    self.flags.negative = (value & 0x80) != 0;

    if mode == AddressingMode::Implied {
      self.a = (value & 0x00FF) as u8;
    } else {
      self.write(bus, self.current_address_abs, (value & 0x00FF) as u8);
    }
  }

  /// Move each of the bits in either A or M one place to the right.
  fn ror(&mut self, bus: &mut dyn BusLike, mode: AddressingMode, initial_cycle_count: usize) {
    self.cycles += initial_cycle_count;
    self.fetch(bus, mode, true, false);

    let value = ((self.flags.carry as u16) << 7) as u16 | (self.fetched_data >> 1) as u16;

    self.flags.carry = (self.fetched_data & 0x01) != 0;
    self.flags.zero = (value & 0x00FF) == 0;
    self.flags.negative = (value & 0x80) != 0;

    if mode == AddressingMode::Implied {
      self.a = (value & 0x00FF) as u8;
    } else {
      self.write(bus, self.current_address_abs, (value & 0x00FF) as u8);
    }
  }

  /// Return from interrupt
  fn rti(&mut self, bus: &mut dyn BusLike, mode: AddressingMode, initial_cycle_count: usize) {
    self.cycles += initial_cycle_count;
    self.fetch(bus, mode, false, false);

    // Pull status flags
    self.sp = self.sp.wrapping_add(1);
    let break_prev = self.flags.break_command;
    self.flags = Flags::from_u8(self.read(bus, 0x0100 + self.sp as u16));
    self.flags.break_command = break_prev;

    // Pull program counter
    self.sp = self.sp.wrapping_add(1);
    self.pc = self.read(bus, 0x0100 + self.sp as u16) as u16;
    self.sp = self.sp.wrapping_add(1);
    self.pc |= (self.read(bus, 0x0100 + self.sp as u16) as u16) << 8;
  }

  /// Pull the program counter from the stack (minus one) and jump to it
  fn rts(&mut self, bus: &mut dyn BusLike, mode: AddressingMode, initial_cycle_count: usize) {
    self.cycles += initial_cycle_count;
    self.fetch(bus, mode, false, false);

    self.sp = self.sp.wrapping_add(1);
    self.pc = self.read(bus, 0x0100 + self.sp as u16) as u16;
    self.sp = self.sp.wrapping_add(1);
    self.pc |= (self.read(bus, 0x0100 + self.sp as u16) as u16) << 8;

    self.pc = self.pc.wrapping_add(1);
  }

  /// Subtraction with carry
  fn sbc(&mut self, bus: &mut dyn BusLike, mode: AddressingMode, initial_cycle_count: usize) {
    self.cycles += initial_cycle_count;
    self.fetch(bus, mode, true, true);

    let value = self.fetched_data as u16 ^ 0x00FF;
    let temp = self.a as u16 + value + self.flags.carry as u16;
    self.flags.carry = temp & 0xFF00 != 0;
    self.flags.zero = (temp & 0x00FF) == 0;
    self.flags.negative = temp & 0x80 != 0;
    self.flags.overflow = (((temp ^ self.a as u16) & (temp ^ value)) & 0x0080) != 0;

    self.a = (temp & 0x00FF) as u8;
  }

  /// Set carry
  fn sec(&mut self, bus: &mut dyn BusLike, mode: AddressingMode, initial_cycle_count: usize) {
    self.cycles += initial_cycle_count;
    self.fetch(bus, mode, false, false);

    self.flags.carry = true;
  }

  /// Set decimal mode
  fn sed(&mut self, bus: &mut dyn BusLike, mode: AddressingMode, initial_cycle_count: usize) {
    self.cycles += initial_cycle_count;
    self.fetch(bus, mode, false, false);

    self.flags.decimal_mode = true;
  }

  /// Set the interrupt disable flag
  fn sei(&mut self, bus: &mut dyn BusLike, mode: AddressingMode, initial_cycle_count: usize) {
    self.cycles += initial_cycle_count;
    self.fetch(bus, mode, false, false);

    self.flags.interrupt_disable = true;
  }

  /// Store the contents of A in memory
  fn sta(&mut self, bus: &mut dyn BusLike, mode: AddressingMode, initial_cycle_count: usize) {
    self.cycles += initial_cycle_count;
    self.fetch(bus, mode, false, false);

    self.write(bus, self.current_address_abs, self.a);
  }

  /// Store the contents of register X in memory
  fn stx(&mut self, bus: &mut dyn BusLike, mode: AddressingMode, initial_cycle_count: usize) {
    self.cycles += initial_cycle_count;
    self.fetch(bus, mode, false, false);

    self.write(bus, self.current_address_abs, self.x);
  }

  /// Store the contents of register Y in memory
  fn sty(&mut self, bus: &mut dyn BusLike, mode: AddressingMode, initial_cycle_count: usize) {
    self.cycles += initial_cycle_count;
    self.fetch(bus, mode, false, false);

    self.write(bus, self.current_address_abs, self.y);
  }

  /// Transfer the contents of A to register X
  fn tax(&mut self, bus: &mut dyn BusLike, mode: AddressingMode, initial_cycle_count: usize) {
    self.cycles += initial_cycle_count;
    self.fetch(bus, mode, false, false);

    self.x = self.a;

    self.flags.zero = self.x == 0;
    self.flags.negative = self.x & 0x80 != 0;
  }

  /// Transfer the contents of A to register Y
  fn tay(&mut self, bus: &mut dyn BusLike, mode: AddressingMode, initial_cycle_count: usize) {
    self.cycles += initial_cycle_count;
    self.fetch(bus, mode, false, false);

    self.y = self.a;

    self.flags.zero = self.y == 0;
    self.flags.negative = self.y & 0x80 != 0;
  }

  /// Transfer the contents of the stack register to register X
  fn tsx(&mut self, bus: &mut dyn BusLike, mode: AddressingMode, initial_cycle_count: usize) {
    self.cycles += initial_cycle_count;
    self.fetch(bus, mode, false, false);

    self.x = self.sp;

    self.flags.zero = self.x == 0;
    self.flags.negative = self.x & 0x80 != 0;
  }

  /// Transfer the contents of register X to A
  fn txa(&mut self, bus: &mut dyn BusLike, mode: AddressingMode, initial_cycle_count: usize) {
    self.cycles += initial_cycle_count;
    self.fetch(bus, mode, false, false);

    self.a = self.x;

    self.flags.zero = self.a == 0;
    self.flags.negative = self.a & 0x80 != 0;
  }

  /// Transfer the contents of register X to the stack register
  fn txs(&mut self, bus: &mut dyn BusLike, mode: AddressingMode, initial_cycle_count: usize) {
    self.cycles += initial_cycle_count;
    self.fetch(bus, mode, false, false);

    self.sp = self.x;
  }

  /// Transfer the contents of register Y to A
  fn tya(&mut self, bus: &mut dyn BusLike, mode: AddressingMode, initial_cycle_count: usize) {
    self.cycles += initial_cycle_count;
    self.fetch(bus, mode, false, false);

    self.a = self.y;

    self.flags.zero = self.a == 0;
    self.flags.negative = self.a & 0x80 != 0;
  }

  // endregion: Instructions

  pub fn reset(&mut self, bus: &mut dyn BusLike) {
    self.current_address_abs = 0xFFFC;
    let low = self.read(bus, self.current_address_abs) as u16;
    let high = self.read(bus, self.current_address_abs + 1) as u16;
    self.pc = (high << 8) | low;

    self.a = 0;
    self.x = 0;
    self.y = 0;
    self.sp = 0xFD;
    self.flags = Default::default();

    self.current_address_abs = 0x0000;
    self.current_address_rel = 0x0000;
    self.fetched_data = 0x00;

    self.cycles = 8;
  }

  pub fn irq(&mut self, bus: &mut dyn BusLike) {
    if !self.flags.interrupt_disable {
      self.write(bus, 0x0100 + self.sp as u16, (self.pc >> 8) as u8);
      self.sp -= 1;
      self.write(bus, 0x0100 + self.sp as u16, (self.pc & 0x00FF) as u8);
      self.sp -= 1;

      self.flags.break_command = false;

      self.write(bus, 0x0100 + self.sp as u16, self.flags.to_u8());
      self.sp -= 1;

      self.flags.interrupt_disable = true;

      self.current_address_abs = 0xFFFE;
      let low = self.read(bus, self.current_address_abs) as u16;
      let high = self.read(bus, self.current_address_abs + 1) as u16;
      self.pc = (high << 8) | low;

      self.cycles = 7;
    }
  }

  pub fn nmi(&mut self, bus: &mut dyn BusLike) {
    self.write(bus, 0x0100 + self.sp as u16, (self.pc >> 8) as u8);
    self.sp = self.sp.wrapping_sub(1);
    self.write(bus, 0x0100 + self.sp as u16, (self.pc & 0x00FF) as u8);
    self.sp = self.sp.wrapping_sub(1);

    self.flags.break_command = false;

    self.write(bus, 0x0100 + self.sp as u16, self.flags.to_u8());
    self.sp = self.sp.wrapping_sub(1);

    self.flags.interrupt_disable = true;

    self.current_address_abs = 0xFFFA;
    let low = self.read(bus, self.current_address_abs) as u16;
    let high = self.read(bus, self.current_address_abs + 1) as u16;
    self.pc = (high << 8) | low;

    self.cycles = 8;
  }
}