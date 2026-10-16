use crate::backends::nes::Cartridge;
use crate::backends::nes::PPU;
use crate::backends::nes::APU;

pub trait BusLike {
  fn cpu_read(&mut self, address: u16) -> u8;
  fn cpu_write(&mut self, address: u16, data: u8);
  fn reset(&mut self);
  #[allow(unused)]
  fn dump_ram(&self) -> Vec<u8>;
  fn get_global_cycles(&self) -> u32;
  fn set_global_cycles(&mut self, cycles: u32);
  fn update_controller(&mut self, controller_index: usize, value: u8);
  fn dma_queued(&self) -> bool;
  fn set_dma_queued(&mut self, queued: bool);
  fn dma_running(&self) -> bool;
  fn set_dma_running(&mut self, running: bool);
  fn dma_page(&self) -> u8;
  fn dma_address(&self) -> u8;
  fn set_dma_address(&mut self, address: u8);
  fn dma_data(&self) -> u8;
  fn set_dma_data(&mut self, data: u8);
  fn scanline(&mut self);
}

pub struct Bus {
  // Devices
  cpu_ram: Vec<u8>,
  pub ppu: PPU,
  pub cartridge: Option<Cartridge>,
  controllers: [u8; 2],
  controllers_state: [u8; 2],
  pub apu: APU,
  // Global cycle count
  global_cycles: u32,
  // DMA vars
  dma_page: u8,
  dma_address: u8,
  dma_data: u8,
  dma_queued: bool,
  dma_running: bool,
}

impl Bus {
  pub fn new() -> Self {
    Self {
      cpu_ram: vec![0; 2048],
      ppu: PPU::new(),
      apu: APU::new(),
      cartridge: None,
      controllers: [0, 0],
      controllers_state: [0, 0],
      global_cycles: 0,
      dma_page: 0,
      dma_address: 0,
      dma_data: 0,
      dma_queued: false,
      dma_running: false,
    }
  }
}

impl BusLike for Bus {
  fn cpu_read(&mut self, address: u16) -> u8 {
    match address {
      0x0000..=0x1FFF => {
        self.cpu_ram[(address & 0x07FF) as usize]
      },
      0x2000..=0x3FFF => {
        self.ppu.cpu_read(&mut self.cartridge, address & 0x0007)
      },
      0x4015 => {
        self.apu.cpu_read(address)
      },
      0x4016 | 0x4017 => {
        let index = (address & 0x1) as usize;
        let value = (self.controllers_state[index] & 0x80) > 0;
        self.controllers_state[index] <<= 1;
        value as u8
      },
      0x6000..=0x7FFF => {
        if let Some(cartridge) = &self.cartridge {
          if cartridge.has_ram {
            cartridge.cpu_read(address)
          } else {
            0
          }
        } else {
          panic!("Cartridge is not connected!");
        }
      }
      0x8000..=0xFFFF => {
        if let Some(cartridge) = &self.cartridge {
          cartridge.cpu_read(address)
        } else {
          panic!("Cartridge is not connected!");
        }
      },
      _ => 0
    }
  }

  fn cpu_write(&mut self, address: u16, value: u8) {
    match address {
      0x0000..=0x1FFF => {
        self.cpu_ram[(address & 0x07FF) as usize] = value;
      },
      0x2000..=0x3FFF => {
        self.ppu.cpu_write(&mut self.cartridge, address & 0x0007, value);
      },
      0x4000..=0x4013 => {
        self.apu.cpu_write(address, value);
      }
      0x4014 => {
        self.dma_page = value;
        self.dma_address = 0;
        self.dma_queued = true;
      },
      0x4015 => {
        self.apu.cpu_write(address, value);
      }
      0x4016 => {
        // NOTE: This seems to be inaccurate from the OLC video, fix later
        // https://www.nesdev.org/wiki/Standard_controller#Input_.28.244016_write.29
        let index = (address & 0x1) as usize;
        self.controllers_state[index] = self.controllers[index];
      },
      0x4017 => {
        self.apu.cpu_write(address, value);
      },
      0x6000..=0x7FFF => {
        if let Some(cartridge) = &mut self.cartridge {
          if cartridge.has_ram {
            cartridge.cpu_write(address, value);
          }
        } else {
          panic!("Cartridge is not connected!");
        }
      }
      0x8000..=0xFFFF => {
        if let Some(cartridge) = &mut self.cartridge {
          cartridge.cpu_write(address, value);
        } else {
          panic!("Cartridge is not connected!");
        }
      },
      _ => {}
    }
  }

  fn reset(&mut self) {
    self.ppu.reset();
  }

  fn dump_ram(&self) -> Vec<u8> {
    println!("{:X?}", self.cpu_ram);
    vec![]
  }

  fn get_global_cycles(&self) -> u32 {
    self.global_cycles
  }

  fn set_global_cycles(&mut self, cycles: u32) {
    self.global_cycles = cycles;
  }

  fn update_controller(&mut self, controller_index: usize, value: u8) {
    self.controllers[controller_index] = value;
  }

  fn dma_queued(&self) -> bool {
    self.dma_queued
  }

  fn set_dma_queued(&mut self, queued: bool) {
    self.dma_queued = queued;
  }

  fn dma_running(&self) -> bool {
    self.dma_running
  }

  fn set_dma_running(&mut self, running: bool) {
    self.dma_running = running;
  }

  fn dma_page(&self) -> u8 {
    self.dma_page
  }

  fn dma_address(&self) -> u8 {
    self.dma_address
  }

  fn set_dma_address(&mut self, address: u8) {
    self.dma_address = address;
  }

  fn dma_data(&self) -> u8 {
    self.dma_data
  }

  fn set_dma_data(&mut self, data: u8) {
    self.dma_data = data;
  }

  fn scanline(&mut self) {
    if let Some(cartridge) = &mut self.cartridge {
      cartridge.mapper.scanline();
    } else {
      panic!("Cartridge is not connected!");
    }
  }
}

#[cfg(test)]
pub struct MockBus {
  pub cpu_ram: Vec<u8>,
}

#[cfg(test)]
impl MockBus {
  pub fn new() -> Self {
    Self {
      cpu_ram: vec![0; 0x10000],
    }
  }
}

#[cfg(test)]
impl BusLike for MockBus {
  fn cpu_read(&mut self, address: u16) -> u8 {
    self.cpu_ram[address as usize]
  }

  fn cpu_write(&mut self, address: u16, value: u8) {
    self.cpu_ram[address as usize] = value;
  }

  fn reset(&mut self) {}

  fn dump_ram(&self) -> Vec<u8> {
    self.cpu_ram.clone()
  }

  fn get_global_cycles(&self) -> u32 {
    0
  }

  fn set_global_cycles(&mut self, _cycles: u32) {}

  fn update_controller(&mut self, _controller_index: usize, _value: u8) {}

  fn dma_queued(&self) -> bool {
    false
  }

  fn set_dma_queued(&mut self, _queued: bool) {}

  fn dma_running(&self) -> bool {
    false
  }

  fn set_dma_running(&mut self, _running: bool) {}

  fn dma_page(&self) -> u8 {
    0
  }

  fn dma_address(&self) -> u8 {
    0
  }

  fn set_dma_address(&mut self, _address: u8) {}

  fn dma_data(&self) -> u8 {
    0
  }

  fn set_dma_data(&mut self, _data: u8) {}

  fn scanline(&mut self) {}
}