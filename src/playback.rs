use cpal::{
    traits::{DeviceTrait, StreamTrait},
    FromSample, SizedSample,
};

use std::fmt::Debug;

use crate::sampler::Sampler;

pub fn run_metronome<T>(device: &cpal::Device, config: &cpal::StreamConfig, mut sampler: Sampler)
where
    T: SizedSample + FromSample<f64> + Debug,
{
    let sample_rate: u32 = config.sample_rate.0;
    let channels: u16 = config.channels;

    let mut fisrt_tick_has_sent = false;
    let stream = device
        .build_output_stream(
            config,
            move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                if !fisrt_tick_has_sent {
                    sampler.send_tick().unwrap();
                    fisrt_tick_has_sent = true;
                }
                sampler.write(data, sample_rate, channels);
            },
            |err| {
                eprintln!("an error occurred on stream: {}", err);
            },
            None,
        )
        .unwrap();

    stream.play().unwrap();
    std::thread::park();
}
