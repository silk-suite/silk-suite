use std::collections::VecDeque;
use std::num::{NonZeroU16, NonZeroU32};
use std::sync::mpsc::Receiver;
use std::time::Duration;

use rodio::source::Source;

/// An infinite source representing the NES APU output.
///
/// Always has a rate of 48kHz and one channel.
pub struct APUOutput {
  apu_messenger: Receiver<Vec<f32>>,
  buffer: VecDeque<f32>,
  last_value: f32,
}

impl APUOutput {
  /// The frequency of the square wave.
  #[inline]
  pub fn new(apu_messenger: Receiver<Vec<f32>>) -> APUOutput {
    APUOutput {
      apu_messenger,
      buffer: vec![].into(),
      last_value: 0.0,
    }
  }
}

impl Iterator for APUOutput {
  type Item = f32;

  #[inline]
  fn next(&mut self) -> Option<f32> {
    match self.apu_messenger.try_recv() {
      Ok(buffer) => {
        self.buffer.extend(buffer)
      },
      Err(_) => {},
    }

    let value = self.buffer.pop_front().unwrap_or(self.last_value);
    self.last_value = value;
    Some(value)
  }
}

impl Source for APUOutput {
  #[inline]
  fn current_span_len(&self) -> Option<usize> {
    None
  }

  #[inline]
  fn channels(&self) -> NonZeroU16 {
    NonZeroU16::new(1).unwrap()
  }

  #[inline]
  fn sample_rate(&self) -> NonZeroU32 {
    NonZeroU32::new(48000).unwrap()
  }

  #[inline]
  fn total_duration(&self) -> Option<Duration> {
    None
  }
}