use cpal::{FromSample, SizedSample};
use eyre::{eyre, Result};
use hound::{SampleFormat, WavReader};
use std::io::{self, BufReader};
use std::sync::atomic::AtomicBool;
use std::sync::{atomic::Ordering, mpsc::Sender, Arc};

use crate::utils::AtomicF64;

const AUDIO_FILE: &[u8] = include_bytes!("../assets/click.wav");

#[derive(Debug)]
pub struct SamplerParam {
    pub bpm: AtomicF64,
    pub playing: AtomicBool,
    pub volume: AtomicF64,
}

#[derive(Debug)]
pub enum SamplerEvent {
    Tick,
}

#[derive(Debug)]
pub struct Sampler {
    // buffer
    samples: Vec<f64>,
    #[allow(dead_code)]
    n_channels: u16,
    sample_rate: u32,
    // parameter
    param: Arc<SamplerParam>,
    // event sender (optional)
    sender: Option<Sender<SamplerEvent>>,
    // internal states
    playhead: f64,
    was_playing: bool,
}

impl Sampler {
    pub fn new(param: Arc<SamplerParam>, sender: Option<Sender<SamplerEvent>>) -> Result<Self> {
        // It must success
        let mut reader = hound::WavReader::new(AUDIO_FILE).unwrap();
        Self::from_reader(&mut reader, param, sender)
    }

    #[allow(dead_code)]
    pub fn from_path(
        file_path: &str,
        param: Arc<SamplerParam>,
        sender: Option<Sender<SamplerEvent>>,
    ) -> Result<Self> {
        let mut reader =
            WavReader::new(BufReader::new(std::fs::File::open(file_path).unwrap())).unwrap();
        Self::from_reader(&mut reader, param, sender)
    }

    pub fn from_reader<R: io::Read>(
        reader: &mut WavReader<R>,
        param: Arc<SamplerParam>,
        sender: Option<Sender<SamplerEvent>>,
    ) -> Result<Self> {
        let spec = reader.spec();
        let bit_depth = spec.bits_per_sample;

        match spec.sample_format {
            SampleFormat::Float => {
                let buffer_in = read_samples_to_buffer::<f32, _>(reader);
                let mut buffer_out = vec![0.0; buffer_in.len()];
                buffer_f32_to_f64(&buffer_in, &mut buffer_out)?;
                Ok(Self {
                    samples: buffer_out,
                    n_channels: spec.channels,
                    sample_rate: spec.sample_rate,
                    playhead: 0.0,
                    param,
                    sender,
                    was_playing: false,
                })
            }
            SampleFormat::Int => {
                let samples: Vec<f64> = match bit_depth {
                    16 => {
                        let buffer_in = read_samples_to_buffer::<i16, _>(reader);
                        let mut buffer_out = vec![0.0; buffer_in.len()];
                        buffer_i16_to_f64(&buffer_in, bit_depth, &mut buffer_out)?;
                        Ok(buffer_out)
                    }
                    24 | 32 => {
                        let buffer_in = read_samples_to_buffer::<i32, _>(reader);
                        let mut buffer_out = vec![0.0; buffer_in.len()];
                        buffer_i32_to_f64(&buffer_in, bit_depth, &mut buffer_out)?;
                        Ok(buffer_out)
                    }
                    _ => Err(eyre!("Unsupported integer sample format bit depth")),
                }?;

                Ok(Self {
                    samples,
                    n_channels: spec.channels,
                    sample_rate: spec.sample_rate,
                    playhead: 0.0,
                    param,
                    sender,
                    was_playing: false,
                })
            }
        }
    }

    pub fn send_tick(&self) -> Result<()> {
        if let Some(ref _sender) = self.sender {
            _sender.send(SamplerEvent::Tick)?;
        }
        Ok(())
    }

    fn cycle_length(&self) -> f64 {
        let bpm = self.param.bpm.load(Ordering::Relaxed);
        self.sample_rate as f64 * 60.0 / bpm
    }

    pub fn write<T>(&mut self, data: &mut [T], sample_rate: u32, n_channels: u16)
    where
        T: SizedSample + FromSample<f64>,
    {
        for frame in data.chunks_mut(n_channels as usize) {
            // update playing state
            let playing = self.param.playing.load(Ordering::Relaxed);
            if !self.was_playing && playing {
                self.was_playing = true;
            } else if self.was_playing && !playing {
                self.was_playing = false;
                self.playhead = 0.0;
            }

            // skip if is not playing
            if !playing {
                continue;
            }

            // BUG: This does not handle stereo samples.
            let volume = self.param.volume.load(Ordering::Relaxed);
            let idx = self.playhead.round() as usize;
            if idx < self.samples.len() {
                let value: T = T::from_sample(self.samples[idx] * volume);
                for sample in frame.iter_mut() {
                    *sample = value;
                }
            }

            let inc = self.sample_rate as f64 / sample_rate as f64;
            let playhead_inc = self.playhead + inc;
            let length = self.cycle_length();

            // move the clock
            if playhead_inc < length {
                self.playhead = playhead_inc;
            } else {
                self.playhead = playhead_inc - length;
                // send a tick whenever the playhead rewinds
                self.send_tick().unwrap();
            }
        }
    }
}

fn read_samples_to_buffer<T, R>(reader: &mut WavReader<R>) -> Vec<T>
where
    R: io::Read,
    T: hound::Sample,
{
    reader.samples::<T>().map(|x| x.unwrap()).collect()
}

fn buffer_i32_to_f64(buffer_in: &[i32], bit_depth: u16, buffer_out: &mut [f64]) -> Result<()> {
    let max_value = match bit_depth {
        24 => Ok(((1 << 23) - 1) as f64),
        32 => Ok(i32::MAX as f64),
        _ => Err(eyre!("Not supported bit depth")),
    }?;
    for (sample_in, sample_out) in buffer_in.iter().zip(buffer_out.iter_mut()) {
        let normalized_sample = *sample_in as f64 / max_value;
        *sample_out = normalized_sample;
    }
    Ok(())
}

fn buffer_i16_to_f64(buffer_in: &[i16], bit_depth: u16, buffer_out: &mut [f64]) -> Result<()> {
    let max_value = match bit_depth {
        16 => Ok(i16::MAX as f64),
        _ => Err(eyre!("Not supported bit depth")),
    }?;
    for (sample_in, sample_out) in buffer_in.iter().zip(buffer_out.iter_mut()) {
        let normalized_sample = *sample_in as f64 / max_value;
        *sample_out = normalized_sample;
    }
    Ok(())
}

fn buffer_f32_to_f64(buffer_in: &[f32], buffer_out: &mut [f64]) -> Result<()> {
    for (sample_in, sample_out) in buffer_in.iter().zip(buffer_out.iter_mut()) {
        *sample_out = *sample_in as f64;
    }
    Ok(())
}
