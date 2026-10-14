mod config;
mod metadata;

use crate::backends::chip8::config::CHIP8Config;
use crate::backends::chip8::metadata::{CHIP8Metadata, Platform, Quirks};
use crate::core::{Backend, Config, ConfigValue, InputDescriptor, InputEvent, InputKind};
use rand::RngExt;
use sha1::{Digest, Sha1};

use std::sync::{Arc, Mutex};

const CHIP8_INPUTS: [InputDescriptor; 16] = [
    InputDescriptor::new(0x1, "1", InputKind::Digital),
    InputDescriptor::new(0x2, "2", InputKind::Digital),
    InputDescriptor::new(0x3, "3", InputKind::Digital),
    InputDescriptor::new(0xC, "4", InputKind::Digital),
    InputDescriptor::new(0x4, "Q", InputKind::Digital),
    InputDescriptor::new(0x5, "W", InputKind::Digital),
    InputDescriptor::new(0x6, "E", InputKind::Digital),
    InputDescriptor::new(0xD, "R", InputKind::Digital),
    InputDescriptor::new(0x7, "A", InputKind::Digital),
    InputDescriptor::new(0x8, "S", InputKind::Digital),
    InputDescriptor::new(0x9, "D", InputKind::Digital),
    InputDescriptor::new(0xE, "F", InputKind::Digital),
    InputDescriptor::new(0xA, "Z", InputKind::Digital),
    InputDescriptor::new(0x0, "X", InputKind::Digital),
    InputDescriptor::new(0xB, "C", InputKind::Digital),
    InputDescriptor::new(0xF, "V", InputKind::Digital),
];

pub struct CHIP8 {
    config: Arc<Mutex<Config>>,
    metadata: CHIP8Metadata,
    platform: Platform,
    quirks: Quirks,
    /// Pseudo-stack for storing subroutine return addresses
    stack: Vec<u16>,
    /// Memory (4KB of RAM)
    memory: [u8; 65536],
    /// Data registers V0 - VF
    v: [u8; 16],
    /// Address register
    i: u16,
    /// Program counter
    pc: u16,
    /// Current value of the delay timer
    delay_timer: u8,
    /// Current value of the sound timer
    sound_timer: u8,
    /// Current input state represented as u16 bitfield
    input: [bool; 16],
    hires_mode: bool,
    /// Framebuffer (RGB)
    framebuffer: [u8; 128 * 64 * 3],
    /// Buffer for audio samples
    audio_buffer: Vec<f32>,
    /// Flag for execution to be paused, such as when waiting for inut in FX0A
    halted: bool,
    audio_phase: f32,
}

impl CHIP8 {
    pub fn new() -> Self {
        let platform = Platform::originalChip8;
        let mut quirks = Quirks::default();
        quirks.set_platform(platform.clone());

        let framebuffer = [0; 128 * 64 * 3];

        Self {
            config: Arc::new(Mutex::new(CHIP8Config::new().entries)),
            metadata: CHIP8Metadata::new(),
            platform,
            quirks,
            stack: Vec::new(),
            memory: [0; 65536],
            v: [0; 16],
            i: 0,
            pc: 0x200,
            delay_timer: 0,
            sound_timer: 0,
            input: [false; 16],
            hires_mode: false,
            framebuffer,
            audio_buffer: Vec::new(),
            halted: false,
            audio_phase: 0.0,
        }
    }

    fn fetch_opcode(&mut self) -> u16 {
        // Read next instruction and increment
        let idx = self.pc as usize;
        let op = u16::from_be_bytes([self.memory[idx], self.memory[idx + 1]]);
        self.pc += 2;
        //println!("{:04X}", op);

        op
    }

    fn decode_instruction(&mut self, opcode: u16) {
        let op1 = ((opcode & 0xF000) >> 12) as u8;
        let op2 = ((opcode & 0x0F00) >> 8) as u8;
        let op3 = ((opcode & 0x00F0) >> 4) as u8;
        let op4 = (opcode & 0x000F) as u8;

        match (op1, op2, op3, op4) {
            (0x0, 0x0, 0xC, _) => self.scroll_down_00cn(op4),
            (0x0, 0x0, 0xD, _) => self.scroll_up_00dn(op4),
            (0x0, 0x0, 0xE, 0x0) => self.clear_screen_00e0(),
            (0x0, 0x0, 0xE, 0xE) => self.return_from_subroutine_00ee(),
            (0x0, 0x0, 0xF, 0xB) => self.scroll_right_00fb(),
            (0x0, 0x0, 0xF, 0xC) => self.scroll_left_00fc(),
            (0x0, 0x0, 0xF, 0xE) => self.enable_lores_mode_00fe(),
            (0x0, 0x0, 0xF, 0xF) => self.enable_hires_mode_00ff(),
            (0x0, _, _, _) => {} // Execute machine language subroutine at NNN,
            (0x1, _, _, _) => self.jump_to_address_1nnn(opcode & 0x0FFF),
            (0x2, _, _, _) => self.execute_subroutine_2nnn(opcode & 0x0FFF),
            (0x3, _, _, _) => self.skip_vx_nn_3xnn(op2 as usize, (op3 << 4) | op4),
            (0x4, _, _, _) => self.skip_vx_not_nn_4xnn(op2 as usize, (op3 << 4) | op4),
            (0x5, _, _, 0x0) => self.skip_vx_vy_5xy0(op2 as usize, op3 as usize),
            (0x6, _, _, _) => self.store_nn_vx_6xnn(op2 as usize, (op3 << 4) | op4),
            (0x7, _, _, _) => self.add_nn_vx_7xnn(op2 as usize, (op3 << 4) | op4),
            (0x8, _, _, 0x0) => self.store_vy_vx_8xy0(op2 as usize, op3 as usize),
            (0x8, _, _, 0x1) => self.set_vx_vy_or_8xy1(op2 as usize, op3 as usize),
            (0x8, _, _, 0x2) => self.set_vx_vy_and_8xy2(op2 as usize, op3 as usize),
            (0x8, _, _, 0x3) => self.set_vx_vy_xor_8xy3(op2 as usize, op3 as usize),
            (0x8, _, _, 0x4) => self.add_vx_vy_8xy4(op2 as usize, op3 as usize),
            (0x8, _, _, 0x5) => self.sub_vx_vy_8xy5(op2 as usize, op3 as usize),
            (0x8, _, _, 0x6) => self.set_vx_vy_rshift_8xy6(op2 as usize, op3 as usize),
            (0x8, _, _, 0x7) => self.set_vx_vy_sub_8xy7(op2 as usize, op3 as usize),
            (0x8, _, _, 0xE) => self.set_vx_vy_lshift_8xye(op2 as usize, op3 as usize),
            (0x9, _, _, 0x0) => self.skip_vx_not_vy_9xy0(op2 as usize, op3 as usize),
            (0xA, _, _, _) => self.store_memory_address_annn(opcode & 0x0FFF),
            (0xB, _, _, _) => self.jump_to_address_offset_bnnn(op2 as usize, opcode & 0x0FFF),
            (0xC, _, _, _) => self.set_vx_rand_cxnn(op2 as usize, (op3 << 4) | op4),
            (0xD, _, _, _) => self.draw_sprite_dxyn(op2 as usize, op3 as usize, op4),
            (0xE, _, 0x9, 0xE) => self.skip_vx_pressed_ex9e(op2 as usize),
            (0xE, _, 0xA, 0x1) => self.skip_vx_not_pressed_exa1(op2 as usize),
            (0xF, _, 0x0, 0x7) => self.store_delay_vx_fx07(op2 as usize),
            (0xF, _, 0x0, 0xA) => self.wait_for_keypress_fx0a(op2 as usize),
            (0xF, _, 0x1, 0x5) => self.set_delay_vx_fx15(op2 as usize),
            (0xF, _, 0x1, 0x8) => self.set_sound_vx_fx18(op2 as usize),
            (0xF, _, 0x1, 0xE) => self.add_vx_i_fx1e(op2 as usize),
            (0xF, _, 0x2, 0x9) => self.set_sprite_data_fx29(op2 as usize),
            (0xF, _, 0x3, 0x3) => self.store_bcd_vx_fx33(op2 as usize),
            (0xF, _, 0x5, 0x5) => self.store_v0_vx_fx55(op2 as usize),
            (0xF, _, 0x6, 0x5) => self.load_v0_vx_fx65(op2 as usize),
            (_, _, _, _) => panic!("Unhandled opcode: {:4X}", opcode),
        }
    }

    // Opcode instructions
    fn scroll_down_00cn(&mut self, n: u8) {
        let amount = self.vertical_scroll_amount(n);
        self.scroll_vertical(amount, true);
    }

    fn scroll_up_00dn(&mut self, n: u8) {
        let amount = self.vertical_scroll_amount(n);
        self.scroll_vertical(amount, false);
    }

    fn clear_screen_00e0(&mut self) {
        self.clear_framebuffer();
    }

    fn return_from_subroutine_00ee(&mut self) {
        if let Some(address) = self.stack.pop() {
            self.pc = address;
        }
    }

    fn scroll_right_00fb(&mut self) {
        self.scroll_horizontal(self.horizontal_scroll_amount(), true);
    }

    fn scroll_left_00fc(&mut self) {
        self.scroll_horizontal(self.horizontal_scroll_amount(), false);
    }

    fn enable_lores_mode_00fe(&mut self) {
        self.hires_mode = false;
    }

    fn enable_hires_mode_00ff(&mut self) {
        self.hires_mode = true;
    }

    fn jump_to_address_1nnn(&mut self, nnn: u16) {
        self.pc = nnn;
    }

    fn execute_subroutine_2nnn(&mut self, nnn: u16) {
        self.stack.push(self.pc);
        self.pc = nnn;
    }

    fn skip_vx_nn_3xnn(&mut self, x: usize, nn: u8) {
        if self.v[x] == nn {
            self.pc += 2;
        }
    }

    fn skip_vx_not_nn_4xnn(&mut self, x: usize, nn: u8) {
        if self.v[x] != nn {
            self.pc += 2;
        }
    }

    fn skip_vx_vy_5xy0(&mut self, x: usize, y: usize) {
        if self.v[x] == self.v[y] {
            self.pc += 2;
        }
    }

    fn store_nn_vx_6xnn(&mut self, x: usize, nn: u8) {
        self.v[x] = nn;
    }

    fn add_nn_vx_7xnn(&mut self, x: usize, nn: u8) {
        self.v[x] = self.v[x].wrapping_add(nn);
    }

    fn store_vy_vx_8xy0(&mut self, x: usize, y: usize) {
        self.v[x] = self.v[y];
    }

    fn set_vx_vy_or_8xy1(&mut self, x: usize, y: usize) {
        self.v[x] |= self.v[y];
        if self.quirks.logic {
            self.v[0xF] = 0;
        }
    }

    fn set_vx_vy_and_8xy2(&mut self, x: usize, y: usize) {
        self.v[x] &= self.v[y];
        if self.quirks.logic {
            self.v[0xF] = 0;
        }
    }

    fn set_vx_vy_xor_8xy3(&mut self, x: usize, y: usize) {
        self.v[x] ^= self.v[y];
        if self.quirks.logic {
            self.v[0xF] = 0;
        }
    }

    fn add_vx_vy_8xy4(&mut self, x: usize, y: usize) {
        let (result, overflow) = self.v[x].overflowing_add(self.v[y]);
        self.v[x] = result;
        self.v[0xF] = if overflow { 1 } else { 0 };
    }

    fn sub_vx_vy_8xy5(&mut self, x: usize, y: usize) {
        let (result, overflow) = self.v[x].overflowing_sub(self.v[y]);
        self.v[x] = result;
        self.v[0xF] = if overflow { 0 } else { 1 };
    }

    fn set_vx_vy_rshift_8xy6(&mut self, x: usize, y: usize) {
        let lsb = self.v[x] & 0x01;
        if self.quirks.shift {
            self.v[x] >>= 1;
        } else {
            self.v[x] = self.v[y] >> 1;
        }
        self.v[0xF] = lsb;
    }

    fn set_vx_vy_sub_8xy7(&mut self, x: usize, y: usize) {
        let (result, overflow) = self.v[y].overflowing_sub(self.v[x]);
        self.v[x] = result;
        self.v[0xF] = if overflow { 0 } else { 1 };
    }

    fn set_vx_vy_lshift_8xye(&mut self, x: usize, y: usize) {
        let msb = (self.v[x] & 0x80) >> 7;
        if self.quirks.shift {
            self.v[x] <<= 1;
        } else {
            self.v[x] = self.v[y] << 1;
        }
        self.v[0xF] = msb;
    }

    fn skip_vx_not_vy_9xy0(&mut self, x: usize, y: usize) {
        if self.v[x] != self.v[y] {
            self.pc += 2;
        }
    }

    fn store_memory_address_annn(&mut self, nnn: u16) {
        self.i = nnn;
    }

    fn jump_to_address_offset_bnnn(&mut self, x: usize, nnn: u16) {
        if self.quirks.jump {
            self.pc = nnn + self.v[x] as u16;
        } else {
            self.pc = nnn + self.v[0] as u16;
        }
    }

    fn set_vx_rand_cxnn(&mut self, x: usize, nn: u8) {
        let mut rng = rand::rng();
        let value: u8 = rng.random();
        self.v[x] = value & nn;
    }

    fn draw_sprite_dxyn(&mut self, x: usize, y: usize, n: u8) {
        // TODO: vblank quirk, wrap quirk
        let x_pos = self.v[x] as isize;
        let y_pos = self.v[y] as isize;

        let (w, h) = self.framebuffer_size();
        let (width, height) = (w as isize, h as isize);

        self.v[0xF] = 0;

        // Determine whether initial coordinates are out of bounds
        let wrap_x = !(0..width).contains(&x_pos);
        let wrap_y = !(0..height).contains(&y_pos);

        for row in 0..n as isize {
            let mut py = y_pos + row;

            if wrap_y {
                py = py.rem_euclid(height);
            } else if py >= height {
                break; // clip vertically
            }

            let sprite = self.memory[self.i as usize + row as usize];

            for col in 0..8isize {
                if (sprite & (0x80 >> col)) == 0 {
                    continue;
                }

                let mut px = x_pos + col;

                if wrap_x {
                    px = px.rem_euclid(width);
                } else if px >= width {
                    break; // clip horizontally
                }

                let idx = ((py * width + px) * 3) as usize;

                let pixel_on = self.framebuffer[idx] == 255;

                if pixel_on {
                    self.v[0xF] = 1;

                    // XOR OFF
                    self.framebuffer[idx] = 0;
                    self.framebuffer[idx + 1] = 0;
                    self.framebuffer[idx + 2] = 0;
                } else {
                    // XOR ON
                    self.framebuffer[idx] = 255;
                    self.framebuffer[idx + 1] = 255;
                    self.framebuffer[idx + 2] = 255;
                }
            }
        }
    }

    fn skip_vx_pressed_ex9e(&mut self, x: usize) {
        if self.input[self.v[x] as usize] {
            self.pc += 2;
        }
    }

    fn skip_vx_not_pressed_exa1(&mut self, x: usize) {
        let idx = self.v[x] & 0x0F;
        if !self.input[idx as usize] {
            self.pc += 2;
        }
    }

    fn store_delay_vx_fx07(&mut self, x: usize) {
        self.v[x] = self.delay_timer;
    }

    fn wait_for_keypress_fx0a(&mut self, x: usize) {
        let mut key_idx = None;
        for (idx, key) in self.input.iter().enumerate() {
            if *key {
                key_idx.replace(idx);
                break;
            }
        }
        match key_idx {
            Some(idx) => {
                self.halted = false;
                self.v[x] = idx as u8;
            }
            None => {
                self.halted = true;
                self.pc = self.pc.wrapping_sub(2);
            }
        }
    }

    fn set_delay_vx_fx15(&mut self, x: usize) {
        self.delay_timer = self.v[x];
    }

    fn set_sound_vx_fx18(&mut self, x: usize) {
        self.sound_timer = self.v[x];
    }

    fn add_vx_i_fx1e(&mut self, x: usize) {
        self.i = self.i.wrapping_add(self.v[x] as u16);
    }

    fn set_sprite_data_fx29(&mut self, x: usize) {
        let digit = self.v[x] & 0x0F;
        self.i = 0x0000 + (digit * 5) as u16;
    }

    fn store_bcd_vx_fx33(&mut self, x: usize) {
        self.memory[self.i as usize % self.memory.len()] = self.v[x] / 100;
        self.memory[(self.i as usize + 1) % self.memory.len()] = (self.v[x] / 10) % 10;
        self.memory[(self.i as usize + 2) % self.memory.len()] = (self.v[x] % 100) % 10;
    }

    fn store_v0_vx_fx55(&mut self, x: usize) {
        for i in 0..(x + 1) {
            self.memory[(self.i as usize + i) % self.memory.len()] = self.v[i];
        }

        if self.quirks.memoryLeaveIUnchanged {
            return;
        }

        if self.quirks.memoryIncrementByX {
            self.i = self.i.wrapping_add(x as u16);
        } else {
            self.i = self.i.wrapping_add(x as u16 + 1);
        }
    }

    fn load_v0_vx_fx65(&mut self, x: usize) {
        for i in 0..(x + 1) {
            self.v[i] = self.memory[(self.i as usize + i) % self.memory.len()];
        }

        if self.quirks.memoryLeaveIUnchanged {
            return;
        }

        if self.quirks.memoryIncrementByX {
            self.i = self.i.wrapping_add(x as u16);
        } else {
            self.i = self.i.wrapping_add(x as u16 + 1);
        }
    }

    // Helpers
    fn clear_framebuffer(&mut self) {
        self.framebuffer.fill(0);
    }

    fn load_font(&mut self) {
        let lores_fontset: [u8; 5 * 16] = [
            0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
            0x20, 0x60, 0x20, 0x20, 0x70, // 1
            0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
            0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
            0x90, 0x90, 0xF0, 0x10, 0x10, // 4
            0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
            0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
            0xF0, 0x10, 0x20, 0x40, 0x40, // 7
            0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
            0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
            0xF0, 0x90, 0xF0, 0x90, 0x90, // A
            0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
            0xF0, 0x80, 0x80, 0x80, 0xF0, // C
            0xE0, 0x90, 0x90, 0x90, 0xE0, // D
            0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
            0xF0, 0x80, 0xF0, 0x80, 0x80, // F
        ];
        let hires_fontset: [u8; 10 * 16] = [
            0xff, 0xff, 0xc3, 0xc3, 0xc3, 0xc3, 0xc3, 0xc3, 0xff, 0xff, // 0
            0x18, 0x78, 0x78, 0x18, 0x18, 0x18, 0x18, 0x18, 0xff, 0xff, // 1
            0xff, 0xff, 0x03, 0x03, 0xff, 0xff, 0xc0, 0xc0, 0xff, 0xff, // 2
            0xff, 0xff, 0x03, 0x03, 0xff, 0xff, 0x03, 0x03, 0xff, 0xff, // 3
            0xc3, 0xc3, 0xc3, 0xc3, 0xff, 0xff, 0x03, 0x03, 0x03, 0x03, // 4
            0xff, 0xff, 0xc0, 0xc0, 0xff, 0xff, 0x03, 0x03, 0xff, 0xff, // 5
            0xff, 0xff, 0xc0, 0xc0, 0xff, 0xff, 0xc3, 0xc3, 0xff, 0xff, // 6
            0xff, 0xff, 0x03, 0x03, 0x06, 0x0c, 0x18, 0x18, 0x18, 0x18, // 7
            0xff, 0xff, 0xc3, 0xc3, 0xff, 0xff, 0xc3, 0xc3, 0xff, 0xff, // 8
            0xff, 0xff, 0xc3, 0xc3, 0xff, 0xff, 0x03, 0x03, 0xff, 0xff, // 9
            0x7e, 0xff, 0xc3, 0xc3, 0xc3, 0xff, 0xff, 0xc3, 0xc3, 0xc3, // A
            0xfc, 0xfc, 0xc3, 0xc3, 0xfc, 0xfc, 0xc3, 0xc3, 0xfc, 0xfc, // B
            0x3c, 0xff, 0xc3, 0xc0, 0xc0, 0xc0, 0xc0, 0xc3, 0xff, 0x3c, // C
            0xfc, 0xfe, 0xc3, 0xc3, 0xc3, 0xc3, 0xc3, 0xc3, 0xfe, 0xfc, // D
            0xff, 0xff, 0xc0, 0xc0, 0xff, 0xff, 0xc0, 0xc0, 0xff, 0xff, // E
            0xff, 0xff, 0xc0, 0xc0, 0xff, 0xff, 0xc0, 0xc0, 0xc0, 0xc0, // F
        ];

        let memory_slice = &mut self.memory[0x00..0xf0];
        memory_slice[0..lores_fontset.len()].copy_from_slice(&lores_fontset[..]);
        memory_slice[0x50..].copy_from_slice(&hires_fontset[..]);
    }

    fn vertical_scroll_amount(&self, n: u8) -> usize {
        if !self.hires_mode && self.platform == Platform::superchip1 {
            (n as usize) / 2
        } else {
            n as usize
        }
    }

    fn horizontal_scroll_amount(&self) -> usize {
        if !self.hires_mode && self.platform == Platform::superchip1 {
            2
        } else {
            4
        }
    }

    fn scroll_vertical(&mut self, amount: usize, down: bool) {
        let (w, h) = self.framebuffer_size();
        let row_bytes = w * 4;
        let visible_bytes = h * row_bytes;

        if amount == 0 || amount >= h {
            // clear to opaque black, not transparent
            for px in self.framebuffer[..visible_bytes].chunks_exact_mut(4) {
                px[0] = 0;
                px[1] = 0;
                px[2] = 0;
                px[3] = 255;
            }
            return;
        }

        let byte_amount = amount * row_bytes;

        if down {
            self.framebuffer
                .copy_within(0..visible_bytes - byte_amount, byte_amount);

            // fill top with opaque black
            for px in self.framebuffer[..byte_amount].chunks_exact_mut(4) {
                px[0] = 0;
                px[1] = 0;
                px[2] = 0;
                px[3] = 255;
            }
        } else {
            self.framebuffer.copy_within(byte_amount..visible_bytes, 0);

            // fill bottom with opaque black
            for px in
                self.framebuffer[visible_bytes - byte_amount..visible_bytes].chunks_exact_mut(4)
            {
                px[0] = 0;
                px[1] = 0;
                px[2] = 0;
                px[3] = 255;
            }
        }

        // ensure anything outside visible region stays opaque black too
        for px in self.framebuffer[visible_bytes..].chunks_exact_mut(4) {
            px[0] = 0;
            px[1] = 0;
            px[2] = 0;
            px[3] = 255;
        }
    }

    fn scroll_horizontal(&mut self, amount: usize, right: bool) {
        let (w, h) = self.framebuffer_size();

        if amount >= w {
            self.framebuffer.fill(0);
            return;
        }

        let row_bytes = w * 4;
        let visible_bytes = h * row_bytes;
        let shift = amount * 4;

        for row in 0..h {
            let start = row * row_bytes;
            let end = start + row_bytes;

            let row_slice = &mut self.framebuffer[start..end];

            if right {
                row_slice.copy_within(0..row_bytes - shift, shift);
                row_slice[..shift].fill(0);
            } else {
                row_slice.copy_within(shift..row_bytes, 0);
                row_slice[row_bytes - shift..].fill(0);
            }
        }

        self.framebuffer[visible_bytes..].fill(0);
    }

    // Audio
    fn generate_square_wave(&mut self) {
        const SAMPLE_RATE: f32 = 44100.0;
        const FREQ: f32 = 440.0;
        let samples_per_frame = (SAMPLE_RATE / 60.0) as usize;

        for _ in 0..samples_per_frame {
            // Use stored phase instead of calculating from index
            let sample = if self.audio_phase < 0.5 { 0.2 } else { -0.2 };

            // Advance phase continuously
            self.audio_phase = (self.audio_phase + FREQ / SAMPLE_RATE) % 1.0;

            self.audio_buffer.push(sample);
        }
    }

    fn generate_silence(&mut self) {
        const SAMPLE_RATE: f32 = 44100.0;
        let samples_per_frame = (SAMPLE_RATE / 60.0) as usize;

        for _ in 0..samples_per_frame {
            self.audio_buffer.push(0.0);
        }

        // Reset phase when silence starts to avoid click on next beep
        self.audio_phase = 0.0;
    }
}

impl Backend for CHIP8 {
    fn reset(&mut self) {
        self.memory.fill(0);
        self.v.fill(0);
        self.i = 0x200;
        self.delay_timer = 0;
        self.sound_timer = 0;
        self.input.fill(false);
        self.clear_framebuffer();
        self.pc = 0x200;
        self.hires_mode = false;
    }

    fn load_rom(&mut self, data: &[u8]) {
        // Check ROM hash against known ROMs
        let hash = hex::encode(Sha1::digest(data));
        let program = self.metadata.get_program_info(&hash);
        println!("{:?}", program);
        let platforms = &program
            .roms
            .get(&hash)
            .expect("Failed to retrieve program info!")
            .platforms;

        {
            let ConfigValue::Choice { selected, ref options } = self.config.lock().expect("Lock poisoned!")[0].value else {
                todo!();
            };
            self.platform = if selected != 999 {
                Platform::from_str(&options[selected as usize])
            } else {
                platforms[0].clone()
            };
            self.quirks.set_platform(self.platform.clone());
        }

        // Load font(s)
        self.load_font();
        // Load into memory
        let memory_slice = &mut self.memory[0x200..0x200 + data.len()];
        memory_slice.copy_from_slice(data);
    }

    fn reset_rom(&mut self) {
        self.v.fill(0);
        self.i = 0x200;
        self.delay_timer = 0;
        self.sound_timer = 0;
        self.input.fill(false);
        self.clear_framebuffer();
        self.pc = 0x200;
    }

    fn frame_tick(&mut self) {
        if !self.halted {
            self.delay_timer = self.delay_timer.saturating_sub(1);
            self.sound_timer = self.sound_timer.saturating_sub(1);
        }
        let cycles = if self.quirks.shift { 18 } else { 9 };
        for _ in 1..cycles {
            let op = self.fetch_opcode();
            self.decode_instruction(op);
        }

        if self.sound_timer > 1 {
            self.generate_square_wave();
        } else {
            self.generate_silence();
        }
    }

    fn framebuffer_rgb(&mut self) -> Vec<u8> {
        if self.hires_mode {
            self.framebuffer.to_vec()
        } else {
            self.framebuffer[0..6144].to_vec()
        }
    }

    fn framebuffer_size(&self) -> (usize, usize) {
        if self.hires_mode { (128, 64) } else { (64, 32) }
    }

    fn set_inputs(&mut self, inputs: Vec<InputEvent>) {
        for input in inputs {
            match input {
                InputEvent::Digital(idx, pressed) => {
                    let chip8_key = CHIP8_INPUTS[idx].id;
                    self.input[chip8_key] = pressed;
                }
            }
        }
    }

    fn flush_audio_samples(&mut self, out: &mut Vec<f32>) {
        out.append(&mut self.audio_buffer);
    }

    fn input_map(&self) -> &[InputDescriptor] {
        &CHIP8_INPUTS
    }

    fn file_extensions() -> &'static [&'static str] {
        &["ch8"]
    }

    fn config(&self) -> Arc<Mutex<Config>> {
        Arc::clone(&self.config)
    }
}
