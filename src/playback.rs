use cpal::{traits::DeviceTrait, FromSample, SizedSample};

use std::fmt::Debug;

use crate::sampler::Sampler;

pub fn get_stream<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    mut sampler: Sampler,
) -> cpal::Stream
where
    T: SizedSample + FromSample<f64> + Debug,
{
    let sample_rate: u32 = config.sample_rate.0;
    let channels: u16 = config.channels;

    let stream = device
        .build_output_stream(
            config,
            move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                sampler.write(data, sample_rate, channels);
            },
            |err| {
                eprintln!("an error occurred on stream: {}", err);
            },
            None,
        )
        .unwrap();
    stream
}

pub fn init_stream(device: &cpal::Device, sampler: Sampler) -> cpal::Stream {
    let config = device.default_output_config().unwrap();
    match config.sample_format() {
        cpal::SampleFormat::I8 => get_stream::<i8>(&device, &config.into(), sampler),
        cpal::SampleFormat::I16 => get_stream::<i16>(&device, &config.into(), sampler),
        // cpal::SampleFormat::I24 => run::<I24>(&device, &config.into()),
        cpal::SampleFormat::I32 => get_stream::<i32>(&device, &config.into(), sampler),
        // cpal::SampleFormat::I48 => run::<I48>(&device, &config.into()),
        cpal::SampleFormat::I64 => get_stream::<i64>(&device, &config.into(), sampler),
        cpal::SampleFormat::U8 => get_stream::<u8>(&device, &config.into(), sampler),
        cpal::SampleFormat::U16 => get_stream::<u16>(&device, &config.into(), sampler),
        // cpal::SampleFormat::U24 => run::<U24>(&device, &config.into()),
        cpal::SampleFormat::U32 => get_stream::<u32>(&device, &config.into(), sampler),
        // cpal::SampleFormat::U48 => run::<U48>(&device, &config.into()),
        cpal::SampleFormat::U64 => get_stream::<u64>(&device, &config.into(), sampler),
        cpal::SampleFormat::F32 => get_stream::<f32>(&device, &config.into(), sampler),
        cpal::SampleFormat::F64 => get_stream::<f64>(&device, &config.into(), sampler),
        sample_format => panic!("Unsupported sample format '{sample_format}'"),
    }
}
