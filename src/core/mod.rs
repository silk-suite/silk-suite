use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

#[allow(unused)]
pub enum InputKind {
    Digital, // On/Off
    Analog,  // -1.0 to 1.0 (or 0.0 to 1.0 for triggers)
}

pub struct InputDescriptor {
    pub id: usize,
    pub name: &'static str,
    pub kind: InputKind,
}

impl InputDescriptor {
    pub const fn new(id: usize, name: &'static str, kind: InputKind) -> Self {
        Self { id, name, kind }
    }
}

#[derive(Debug)]
pub enum InputEvent {
    Digital(usize, bool),
}

pub trait Backend {
    fn reset(&mut self);
    fn load_rom(&mut self, data: &[u8]);
    fn reset_rom(&mut self);
    fn frame_tick(&mut self);
    fn framebuffer_rgb(&mut self) -> Vec<u8>;
    fn framebuffer_size(&self) -> (usize, usize);
    fn set_inputs(&mut self, input: Vec<InputEvent>);
    fn flush_audio_samples(&mut self, out: &mut Vec<f32>);
    /// Returns the list of inputs this backend supports.
    fn input_map(&self) -> &[InputDescriptor];
    fn file_extensions() -> &'static [&'static str]
    where
        Self: Sized;
    fn config(&self) -> Arc<Mutex<Config>>;
}

pub struct AudioEngine {
    queue: Arc<Mutex<VecDeque<f32>>>,
    _stream: cpal::Stream,
}

impl AudioEngine {
    pub fn new(sample_rate: u32) -> Self {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .expect("no output device available");

        let supported_config = device
            .supported_output_configs()
            .expect("failed to query output configurations")
            .filter(|config| {
                config.sample_format() == cpal::SampleFormat::F32
            })
            .find_map(|config| {
                config.try_with_sample_rate(cpal::SAMPLE_RATE_48K)
            })
            .expect("device does not support 48 kHz f32 output");

        let mut config = supported_config.config();
        config.buffer_size = cpal::BufferSize::Fixed(1024);

        let queue: Arc<Mutex<VecDeque<f32>>> =
            Arc::new(Mutex::new(VecDeque::with_capacity(sample_rate as usize)));

        let queue_cb = queue.clone();

        let channels = config.channels as usize;

        let err_fn = |err| eprintln!("audio stream error: {err}");

        let mut last_sample = 0.0_f32;
        let stream = device
            .build_output_stream(
                config,
                move |data: &mut [f32], _| {
                    let mut q = queue_cb.lock().unwrap();

                    for frame in data.chunks_mut(channels) {
                        let sample = match q.pop_front() {
                            Some(sample) => {
                                last_sample = sample;
                                sample
                            }
                            None => last_sample,
                        };

                        for out in frame {
                            *out = sample;
                        }
                    }
                },
                err_fn,
                None,
            )
            .expect("failed to build output stream");

        stream.play().expect("failed to start audio stream");

        Self {
            queue,
            _stream: stream,
        }
    }

    /// Push emulator-generated samples into the audio queue
    pub fn push_samples(&self, samples: &[f32]) {
        let mut q = self.queue.lock().unwrap();

        for &s in samples {
            q.push_back(s);
        }

        // Prevent unlimited growth (simple backpressure)
        const MAX_BUFFER: usize = 48000 * 2; // ~2 seconds

        while q.len() > MAX_BUFFER {
            q.pop_front();
        }
    }

    /// Optional: clear audio buffer (useful on reset / ROM load)
    pub fn clear(&self) {
        let mut q = self.queue.lock().unwrap();
        q.clear();
    }
}

#[allow(unused)]
#[derive(Clone, Debug)]
pub enum ConfigValue {
    Bool(bool),
    Integer { raw: i32, range: (i32, i32) },
    Float { raw: f32, range: (f32, f32) },
    Text(&'static str),
    Choice { selected: u32, options: Vec<String> },
}

#[derive(Clone, Debug)]
pub struct ConfigEntry {
    pub label: String,
    pub value: ConfigValue,
}

impl ConfigEntry {
    pub fn new(label: String, value: ConfigValue) -> Self {
        Self { label, value }
    }
}

pub type Config = Vec<ConfigEntry>;
