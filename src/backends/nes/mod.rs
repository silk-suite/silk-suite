mod apu;
mod bus;
mod cartridge;
mod config;
mod cpu;
mod ppu;
mod mapper;
mod mappers;
#[cfg(test)]
mod tests;


use apu::APU;
use bus::{Bus, BusLike};
use cartridge::Cartridge;
use cpu::NES6502;
use ppu::PPU;

use std::sync::{Arc, Mutex};

use crate::backends::nes::config::NESConfig;
use crate::core::{Backend, Config, InputDescriptor, InputEvent, InputKind};

const NES_INPUTS: [InputDescriptor; 8] = [
    InputDescriptor::new(0x01, "ArrowRight", InputKind::Digital),
    InputDescriptor::new(0x02, "ArrowLeft", InputKind::Digital),
    InputDescriptor::new(0x04, "ArrowDown", InputKind::Digital),
    InputDescriptor::new(0x08, "ArrowUp", InputKind::Digital),
    InputDescriptor::new(0x10, "Enter", InputKind::Digital),
    InputDescriptor::new(0x20, "Space", InputKind::Digital),
    InputDescriptor::new(0x40, "Z", InputKind::Digital),
    InputDescriptor::new(0x80, "X", InputKind::Digital),
];

pub struct NES {
    config: Arc<Mutex<Config>>,
    bus: Bus,
    cpu: NES6502,
    rom_loaded: bool,
}

impl NES {
    pub fn new() -> Self {
        let bus = Bus::new();
        let cpu = NES6502::new();

        let mut display = Vec::new();
        display.resize(256 * 240 * 3, 0);

        NES {
            config: Arc::new(Mutex::new(NESConfig::new().entries)),
            bus,
            cpu,
            rom_loaded: false,
        }
    }
}

impl Backend for NES {
    fn reset(&mut self) {
        if !self.rom_loaded {
            return;
        }
        self.cpu.reset(&mut self.bus);
        self.bus.reset();
    }

    fn load_rom(&mut self, data: &[u8]) {
        let cartridge = Cartridge::from_bytes(data.to_vec());
        self.bus.cartridge = Some(cartridge);
        self.rom_loaded = true;

        self.reset();
    }

    fn frame_tick(&mut self) {
        if !self.rom_loaded {
            return;
        }
        // Run the emulation
        // It would be nice to just eventually step the bus itself,
        // but the borrow checker is screwing me here so this is fine for now
        for _ in 0..(341*262) {
            // Grab some variables from the bus to use while stepping
            let cycles = self.bus.get_global_cycles();
            let dma_running = self.bus.dma_running();
            let mut should_run_dma = false;

            if self.bus.ppu.step(&mut self.bus.cartridge) { // Returns true if we should scanline
                self.bus.scanline();
            }
            if cycles % 3 == 0 {
                if self.bus.dma_queued() && !dma_running {
                    if cycles % 2 == 1 {
                        should_run_dma = true;
                    }
                } else if dma_running {
                    if cycles % 2 == 0 {
                        let dma_data = {
                            let bus = &mut self.bus;
                            let dma_page = bus.dma_page() as u16;
                            let dma_address = bus.dma_address() as u16;
                            let dma_data = bus.cpu_read((dma_page << 8) | dma_address);
                            dma_data
                        };
                        self.bus.set_dma_data(dma_data);
                    } else {
                        let mut dma_address = self.bus.dma_address();
                        let dma_data = self.bus.dma_data();
                        let oam_index = (dma_address / 4) as usize;
                        let ppu = &mut self.bus.ppu;
                        match dma_address % 4 {
                            0 => ppu.oam[oam_index].y = dma_data,
                            1 => ppu.oam[oam_index].id = dma_data,
                            2 => ppu.oam[oam_index].attributes.set_from_u8(dma_data),
                            3 => ppu.oam[oam_index].x = dma_data,
                            _ => (),
                        }
                        dma_address = dma_address.wrapping_add(1);
                        self.bus.set_dma_address(dma_address);

                        if dma_address == 0 {
                            self.bus.set_dma_running(false);
                            self.bus.set_dma_queued(false);
                        }
                    }
                } else {
                    self.cpu.step(&mut self.bus);
                    self.bus.apu.step(self.cpu.total_cycles);

                    // DMC MEMORY READER
                    // Don't love doing this here but will fix it later
                    if self.bus.apu.registers.dmc.sample_buffer == 0 && self.bus.apu.registers.dmc.bytes_remaining > 0 {
                    self.bus.apu.registers.dmc.sample_buffer = self.bus.cpu_read(self.bus.apu.registers.dmc.sample_address);
                    self.bus.apu.registers.dmc.memory_reader_address = match self.bus.apu.registers.dmc.memory_reader_address.overflowing_add(1) {
                        (_, true) => 0x8000,
                        (address, false) => address,
                    };
                    self.bus.apu.registers.dmc.bytes_remaining -= 1;
                    if self.bus.apu.registers.dmc.bytes_remaining == 0 {
                        if self.bus.apu.registers.dmc.loop_sample {
                            self.bus.apu.registers.dmc.reset();
                            } else if self.bus.apu.registers.dmc.irq_enable {
                            self.bus.apu.registers.status.dmc_interrupt = true;
                            }
                        }
                    }
                    self.bus.apu.registers.dmc.tick_output_unit();

                    if self.bus.apu.registers.status.dmc_interrupt || self.bus.apu.registers.status.frame_interrupt || self.bus.cartridge.as_mut().unwrap().mapper.irq_state() {
                        self.cpu.irq(&mut self.bus);
                    }
                }
            }
            let nmi = self.bus.ppu.nmi;
            if nmi {
                self.bus.ppu.nmi = false;
                self.cpu.nmi(&mut self.bus);
            }
            self.bus.set_global_cycles(cycles + 1);
            if should_run_dma {
                self.bus.set_dma_running(true);
            }
            self.bus.apu.update_output();
        }
    }

    fn framebuffer_rgb(&mut self) -> Vec<u8> {
        self.bus.ppu.get_screen()
    }

    fn framebuffer_size(&self) -> (usize, usize) {
        (256, 240)
    }

    fn set_inputs(&mut self, inputs: Vec<InputEvent>) {
        let mut controller_state = 0x00;
        for input in inputs {
            match input {
                InputEvent::Digital(idx, pressed) => {
                    let nes_button = NES_INPUTS[idx].id;
                    if pressed {
                        controller_state |= nes_button
                    };
                },
            }
        }
        self.bus.update_controller(0, controller_state as u8);
    }

    fn flush_audio_samples(&mut self, out: &mut Vec<f32>) {
        let buffer = std::mem::take(&mut self.bus.apu.output_buffer);
        let mut averaged = buffer
            .chunks(112)
            .fold(Vec::new(), |mut acc, x| {
                let sum: f32 = x.iter().sum();
                acc.push(sum / x.len() as f32);
                acc
            });
        out.append(&mut averaged);
    }

    fn input_map(&self) -> &[InputDescriptor] {
        &NES_INPUTS
    }

    fn file_extensions() -> &'static [&'static str]
    where
        Self: Sized {
        &["ines", "nes"]
    }

    fn config(&self) -> Arc<Mutex<Config>> {
        Arc::clone(&self.config)
    }
}