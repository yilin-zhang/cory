use anyhow::Result;
use cpal::traits::{DeviceTrait, HostTrait};
use sampler::{Sampler, SamplerParam};
use std::sync::mpsc::channel;
use std::sync::Arc;
use std::thread;

use crate::playback::run_metronome;
use crate::utils::AtomicF64;

mod playback;
mod sampler;
mod utils;

fn main() -> Result<()> {
    let (sender, receiver) = channel();
    let param = Arc::new(SamplerParam { bpm: AtomicF64::new(120.0) });
    let sampler = Sampler::new(param.clone(), Some(sender.clone()))?;

    let host = cpal::default_host();
    let device = host.default_output_device().unwrap();
    let config = device.default_output_config()?;

    let handle = thread::spawn(move || {
        loop {
            let x = receiver.recv().unwrap();
            println!("TICK!");
        }
    });

    match config.sample_format() {
        cpal::SampleFormat::I8 => run_metronome::<i8>(&device, &config.into(), sampler),
        cpal::SampleFormat::I16 => run_metronome::<i16>(&device, &config.into(), sampler),
        // cpal::SampleFormat::I24 => run::<I24>(&device, &config.into()),
        cpal::SampleFormat::I32 => run_metronome::<i32>(&device, &config.into(), sampler),
        // cpal::SampleFormat::I48 => run::<I48>(&device, &config.into()),
        cpal::SampleFormat::I64 => run_metronome::<i64>(&device, &config.into(), sampler),
        cpal::SampleFormat::U8 => run_metronome::<u8>(&device, &config.into(), sampler),
        cpal::SampleFormat::U16 => run_metronome::<u16>(&device, &config.into(), sampler),
        // cpal::SampleFormat::U24 => run::<U24>(&device, &config.into()),
        cpal::SampleFormat::U32 => run_metronome::<u32>(&device, &config.into(), sampler),
        // cpal::SampleFormat::U48 => run::<U48>(&device, &config.into()),
        cpal::SampleFormat::U64 => run_metronome::<u64>(&device, &config.into(), sampler),
        cpal::SampleFormat::F32 => run_metronome::<f32>(&device, &config.into(), sampler),
        cpal::SampleFormat::F64 => run_metronome::<f64>(&device, &config.into(), sampler),
        sample_format => panic!("Unsupported sample format '{sample_format}'"),
    }

    handle.join();

    Ok(())
}
