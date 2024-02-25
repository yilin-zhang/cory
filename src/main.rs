use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::channel,
    Arc,
};

use cpal::traits::{HostTrait, StreamTrait};
use eyre::Result;
use ratatui::{backend::CrosstermBackend, Terminal};

use crate::config::CoryConfig;
use crate::playback::init_stream;
use crate::sampler::{Sampler, SamplerParam};
use crate::tui::{App, Tui, UIEventCapturer};
use crate::utils::AtomicF64;

mod config;
mod playback;
mod sampler;
mod tui;
mod utils;

fn main() -> Result<()> {
    // Load config
    let mut config = CoryConfig::load()?;

    // Initialize channel
    let (sampler_event_sender, sampler_event_receiver) = channel();

    // Initialize sampler
    let param = Arc::new(SamplerParam {
        bpm: AtomicF64::new(config.bpm),
        playing: AtomicBool::new(true),
        volume: AtomicF64::new(config.volume),
    });
    let sampler = Sampler::new(param.clone(), Some(sampler_event_sender.clone()))?;

    // Initialize audio device
    let host = cpal::default_host();
    let device = host.default_output_device().unwrap();
    let stream = init_stream(&device, sampler);

    // Initialize TUI
    let backend = CrosstermBackend::new(std::io::stderr());
    let terminal = Terminal::new(backend)?;
    let ui_event_capturer = UIEventCapturer::new(20);
    let mut tui = Tui::new(terminal, ui_event_capturer);
    let mut app = App::new(param.clone());

    tui.enter()?;
    stream.play()?;
    while !app.should_quit {
        // Render the user interface.
        tui.draw(&mut app)?;

        // Audio event (try not to block)
        match sampler_event_receiver.try_recv() {
            Ok(ref e) => app.update_by_sampler_event(e),
            _ => (),
        }

        // UI event
        let input_event = tui.ui_event_capturer.next()?;
        if let Some(ui_event) = app.map_input_event(&input_event) {
            app.update_by_ui_event(&ui_event);
        }
    }
    stream.pause()?;
    tui.exit()?;

    // update config and write
    config.bpm = param.bpm.load(Ordering::Relaxed);
    config.volume = param.volume.load(Ordering::Relaxed);
    config.write()?;

    Ok(())
}
