use std::{collections::HashMap, fs::File, path::Path};

use serde::Deserialize;

#[allow(unused, non_camel_case_types)]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub enum Platform {
    originalChip8,
    hybridVIP,
    modernChip8,
    chip8x,
    chip48,
    superchip1,
    superchip,
    megachip8,
    xochip,
}

impl Platform {
    pub fn as_vec() -> Vec<String> {
        vec![
            "originalChip8".to_owned(),
            "hybridVIP".to_owned(),
            "modernChip8".to_owned(),
            "chip8x".to_owned(),
            "chip48".to_owned(),
            "superchip1".to_owned(),
            "superchip".to_owned(),
            "megachip8".to_owned(),
            "xochip".to_owned(),
        ]
    }

    pub fn from_str(str: &str) -> Self {
        match str {
            "originalChip8" => Platform::originalChip8,
            "hybridVIP" => Platform::hybridVIP,
            "modernChip8" => Platform::modernChip8,
            "chip8x" => Platform::chip8x,
            "chip48" => Platform::chip48,
            "superchip1" => Platform::superchip1,
            "superchip" => Platform::superchip,
            "megachip8" => Platform::megachip8,
            "xochip" => Platform::xochip,
            _ => Platform::originalChip8,
        }
    }
}

#[allow(non_snake_case)]
#[derive(Debug, Default)]
pub struct Quirks {
    pub shift: bool,
    pub memoryIncrementByX: bool,
    pub memoryLeaveIUnchanged: bool,
    pub wrap: bool,
    pub jump: bool,
    pub vblank: bool,
    pub logic: bool,
}

impl Quirks {
    pub fn set_platform(&mut self, platform: Platform) {
        match platform {
            Platform::originalChip8 => {
                self.shift = false;
                self.memoryIncrementByX = false;
                self.memoryLeaveIUnchanged = false;
                self.wrap = false;
                self.jump = false;
                self.vblank = true;
                self.logic = true;
            }
            Platform::hybridVIP => {
                self.shift = false;
                self.memoryIncrementByX = false;
                self.memoryLeaveIUnchanged = false;
                self.wrap = false;
                self.jump = false;
                self.vblank = true;
                self.logic = true;
            }
            Platform::modernChip8 => {
                self.shift = false;
                self.memoryIncrementByX = false;
                self.memoryLeaveIUnchanged = false;
                self.wrap = false;
                self.jump = false;
                self.vblank = false;
                self.logic = false;
            }
            Platform::chip8x => {
                self.shift = false;
                self.memoryIncrementByX = false;
                self.memoryLeaveIUnchanged = false;
                self.wrap = false;
                self.jump = false;
                self.vblank = true;
                self.logic = true;
            }
            Platform::chip48 => {
                self.shift = true;
                self.memoryIncrementByX = true;
                self.memoryLeaveIUnchanged = false;
                self.wrap = false;
                self.jump = true;
                self.vblank = false;
                self.logic = false;
            }
            Platform::superchip1 => {
                self.shift = true;
                self.memoryIncrementByX = true;
                self.memoryLeaveIUnchanged = false;
                self.wrap = false;
                self.jump = true;
                self.vblank = false;
                self.logic = false;
            }
            Platform::superchip => {
                self.shift = true;
                self.memoryIncrementByX = false;
                self.memoryLeaveIUnchanged = true;
                self.wrap = false;
                self.jump = true;
                self.vblank = false;
                self.logic = false;
            }
            Platform::megachip8 => {
                self.shift = true;
                self.memoryIncrementByX = false;
                self.memoryLeaveIUnchanged = true;
                self.wrap = false;
                self.jump = true;
                self.vblank = false;
                self.logic = false;
            }
            Platform::xochip => {
                self.shift = false;
                self.memoryIncrementByX = false;
                self.memoryLeaveIUnchanged = false;
                self.wrap = true;
                self.jump = false;
                self.vblank = false;
                self.logic = false;
            }
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct ProgramRoms {
    pub file: String,
    pub platforms: Vec<Platform>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Program {
    pub title: String,
    pub description: Option<String>,
    pub release: Option<String>,
    pub authors: Option<Vec<String>>,
    pub roms: HashMap<String, ProgramRoms>,
}

#[derive(Debug, Deserialize)]
pub struct CHIP8Metadata {
    programs: Vec<Program>,
    sha1hashes: HashMap<String, u32>,
}

impl CHIP8Metadata {
    pub fn new() -> Self {
        let path = Path::new("metadata/chip8/sha1-hashes.json");
        let sha1hashes_file = File::open(path).expect("Failed to load CHIP8 SHA1 hashes file!");
        let sha1hashes: HashMap<String, u32> =
            serde_json::from_reader(sha1hashes_file).expect("Failed to parse CHIP8 SHA1 hashes!");

        let programs_file = File::open("metadata/chip8/programs.json")
            .expect("Failed to load CHIP8 programs file!");
        let programs: Vec<Program> =
            serde_json::from_reader(programs_file).expect("Failed to parse CHIP8 programs!");

        Self {
            programs,
            sha1hashes,
        }
    }

    pub fn get_program_info(&self, hash: &str) -> Program {
        let (_, program_number) = self
            .sha1hashes
            .get_key_value(hash)
            .expect("Failed to locate program ROM!");
        self.programs
            .get(*program_number as usize)
            .expect("Failed to locate program ROM!")
            .clone()
    }
}
