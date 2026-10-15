mod apu;
mod bus;
mod cartridge;
mod config;
mod cpu;
mod ppu;
mod mapper;
mod mappers;


use apu::APU;
use bus::{Bus, BusLike};
use cartridge::Cartridge;
use cpu::NES6502;
use ppu::PPU;

use std::cell::RefCell;
use std::rc::Rc;
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
    bus: Rc<RefCell<Box<dyn BusLike>>>,
    cpu: Rc<RefCell<NES6502>>,
    ppu: Rc<RefCell<PPU>>,
    apu: Rc<RefCell<APU>>,
    cartridge: Option<Rc<RefCell<Cartridge>>>,
    rom_loaded: bool,
}

impl NES {
    pub fn new() -> Self {
        // Create bus
        let bus = Rc::new(RefCell::new(Box::new(Bus::new()) as Box<dyn BusLike>));

        // Create CPU
        let cpu = Rc::new(RefCell::new(NES6502::new()));

        let ppu = Rc::new(RefCell::new(PPU::new()));

        let apu = Rc::new(RefCell::new(APU::new()));

        // Connect bus to CPU
        {
            let mut bus_ref = bus.borrow_mut();
            let cpu_ref = Rc::clone(&cpu);
            bus_ref.connect_cpu(Rc::clone(&cpu_ref));
        }

        // Connect CPU to bus
        {
            let mut cpu_ref = cpu.borrow_mut();
            let bus_ref = Rc::clone(&bus);
            cpu_ref.connect_to_bus(Rc::clone(&bus_ref));
        }

        // Connect bus to PPU
        {
            let mut bus_ref = bus.borrow_mut();
            let ppu_ref = Rc::clone(&ppu);
            bus_ref.connect_ppu(Rc::clone(&ppu_ref));
        }

        // Connect PPU to bus
        {
            let mut ppu_ref = ppu.borrow_mut();
            let bus_ref = Rc::clone(&bus);
            ppu_ref.connect_to_bus(Rc::clone(&bus_ref));
        }

        // Connect bus to APU
        {
            let mut bus_ref = bus.borrow_mut();
            let apu_ref = Rc::clone(&apu);
            bus_ref.connect_apu(Rc::clone(&apu_ref));
        }

        // Connect APU to bus
        {
            let mut apu_ref = apu.borrow_mut();
            let bus_ref = Rc::clone(&bus);
            apu_ref.connect_to_bus(Rc::clone(&bus_ref));
        }

        let mut display = Vec::new();
        display.resize(184320, 0);

        NES {
            config: Arc::new(Mutex::new(NESConfig::new().entries)),
            bus,
            cpu,
            ppu,
            apu,
            cartridge: None,
            rom_loaded: false,
        }
    }
}

impl Backend for NES {
    fn reset(&mut self) {
        if !self.rom_loaded {
            return;
        }
        self.cpu.borrow_mut().reset();
        self.ppu.borrow_mut().reset();
    }

    fn load_rom(&mut self, data: &[u8]) {
        let cartridge = Rc::new(RefCell::new(Cartridge::from_bytes(data.to_vec())));
        {
            let mut bus_ref = self.bus.borrow_mut();
            let cartridge_ref = Rc::clone(&cartridge);
            bus_ref.insert_cartridge(Rc::clone(&cartridge_ref));
        }
        self.cartridge = Some(cartridge);
        self.rom_loaded = true;

        self.cpu.borrow_mut().reset();
        self.ppu.borrow_mut().reset();
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
            let cycles = self.bus.borrow().get_global_cycles();
            let dma_running = self.bus.borrow().dma_running();
            let mut should_run_dma = false;

            self.ppu.borrow_mut().step();
            if cycles % 3 == 0 {
                if self.bus.borrow().dma_queued() && !dma_running {
                    if cycles % 2 == 1 {
                        should_run_dma = true;
                    }
                } else if dma_running {
                    if cycles % 2 == 0 {
                        let dma_data = {
                            let bus = self.bus.borrow();
                            let dma_page = bus.dma_page() as u16;
                            let dma_address = bus.dma_address() as u16;
                            let dma_data = bus.cpu_read((dma_page << 8) | dma_address);
                            dma_data
                        };
                        self.bus.borrow_mut().set_dma_data(dma_data);
                    } else {
                        let mut dma_address = self.bus.borrow().dma_address();
                        let dma_data = self.bus.borrow().dma_data();
                        let oam_index = (dma_address / 4) as usize;
                        let mut ppu = self.ppu.borrow_mut();
                        match dma_address % 4 {
                            0 => ppu.oam[oam_index].y = dma_data,
                            1 => ppu.oam[oam_index].id = dma_data,
                            2 => ppu.oam[oam_index].attributes.set_from_u8(dma_data),
                            3 => ppu.oam[oam_index].x = dma_data,
                            _ => (),
                        }
                        dma_address = dma_address.wrapping_add(1);
                        self.bus.borrow_mut().set_dma_address(dma_address);

                        if dma_address == 0 {
                            self.bus.borrow_mut().set_dma_running(false);
                            self.bus.borrow_mut().set_dma_queued(false);
                        }
                    }
                } else {
                    self.cpu.borrow_mut().step();
                    self.apu.borrow_mut().step(self.cpu.borrow().total_cycles);
                    if self.apu.borrow().registers.status.dmc_interrupt || self.apu.borrow().registers.status.frame_interrupt || self.cartridge.as_ref().unwrap().borrow().mapper.irq_state() {
                        self.cpu.borrow_mut().irq();
                    }
                }
            }
            let nmi = self.ppu.borrow().nmi;
            if nmi {
                self.ppu.borrow_mut().nmi = false;
                self.cpu.borrow_mut().nmi();
            }
            self.bus.borrow_mut().set_global_cycles(cycles + 1);
            if should_run_dma {
                self.bus.borrow_mut().set_dma_running(true);
            }
            self.apu.borrow_mut().update_output();
        }
    }

    fn framebuffer_rgb(&mut self) -> Vec<u8> {
        self.ppu.borrow().get_screen()
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
        self.bus.borrow_mut().update_controller(0, controller_state as u8);
    }

    fn flush_audio_samples(&mut self, out: &mut Vec<f32>) {
        let buffer = std::mem::take(&mut self.apu.borrow_mut().output_buffer);
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