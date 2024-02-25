use std::sync::atomic::Ordering;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::{io, panic, thread};

use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event as CrosstermEvent, KeyCode,
        KeyModifiers,
    },
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use eyre::Result;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    prelude::{Alignment, Frame, Text},
    style::{Color, Style},
    widgets::{Block, Borders, Gauge, Paragraph},
};

use crate::config::{MAX_BPM, MAX_TOTAL_BEATS, MAX_VOLUME, MIN_BPM, MIN_TOTAL_BEATS, MIN_VOLUME};
use crate::sampler::{SamplerEvent, SamplerParam};

pub type CrosstermTerminal = ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stderr>>;

#[derive(Debug)]
pub struct App {
    pub param: Arc<SamplerParam>,
    pub beat_count: u32,
    pub total_beats: u32,
    pub should_quit: bool,
}

impl App {
    pub fn new(param: Arc<SamplerParam>) -> Self {
        Self {
            param,
            beat_count: 1,
            total_beats: 4,
            should_quit: false,
        }
    }

    pub fn map_input_event(&self, input_event: &InputEvent<CrosstermEvent>) -> Option<Action> {
        match input_event {
            InputEvent::Tick => Some(Action::Tick),
            InputEvent::Input(e) => map_term_event(e),
        }
    }

    pub fn update_by_ui_event(&mut self, ui_event: &Action) {
        match ui_event {
            Action::Tick => {
                // force the UI to refresh
            }
            Action::Quit => {
                self.should_quit = true;
            }
            Action::IncBPM => {
                let bpm = self.param.bpm.load(Ordering::Relaxed);
                self.param
                    .bpm
                    .store((bpm + 1.0).clamp(MIN_BPM, MAX_BPM), Ordering::Relaxed);
            }
            Action::DecBPM => {
                let bpm = self.param.bpm.load(Ordering::Relaxed);
                self.param
                    .bpm
                    .store((bpm - 1.0).clamp(MIN_BPM, MAX_BPM), Ordering::Relaxed);
            }
            Action::IncTotalBeats => {
                if self.total_beats < MAX_TOTAL_BEATS {
                    self.total_beats += 1;
                }
            }
            Action::DecTotalBeats => {
                if self.total_beats > MIN_TOTAL_BEATS {
                    self.total_beats -= 1;
                }
            }
            Action::IncVolume => {
                let volume = self.param.volume.load(Ordering::Relaxed);
                self.param.volume.store(
                    (volume + 0.1).clamp(MIN_VOLUME, MAX_VOLUME),
                    Ordering::Relaxed,
                );
            }
            Action::DecVolume => {
                let volume = self.param.volume.load(Ordering::Relaxed);
                self.param.volume.store(
                    (volume - 0.1).clamp(MIN_VOLUME, MAX_VOLUME),
                    Ordering::Relaxed,
                );
            }
        };
    }

    pub fn update_by_sampler_event(&mut self, _sampler_event: &SamplerEvent) {
        // For now there is only one kind of event (tick), no need to parse
        self.beat_count = self.beat_count % self.total_beats + 1;
    }
}

pub enum Action {
    Tick,
    IncBPM,
    DecBPM,
    IncTotalBeats,
    DecTotalBeats,
    IncVolume,
    DecVolume,
    Quit,
}

pub enum InputEvent<T> {
    Input(T),
    Tick,
}

#[derive(Debug)]
pub struct UIEventCapturer {
    #[allow(dead_code)]
    sender: Sender<InputEvent<CrosstermEvent>>,
    receiver: Receiver<InputEvent<CrosstermEvent>>,
    #[allow(dead_code)]
    handler: thread::JoinHandle<()>,
}

impl UIEventCapturer {
    pub fn new(tick_rate: u64) -> Self {
        let tick_rate = Duration::from_millis(tick_rate);
        let (ui_event_sender, ui_event_receiver) = mpsc::channel();
        let handler = {
            let sender = ui_event_sender.clone();
            thread::spawn(move || {
                let mut last_tick = Instant::now();
                loop {
                    let timeout = tick_rate
                        .checked_sub(last_tick.elapsed())
                        .unwrap_or(tick_rate);

                    if event::poll(timeout).expect("unable to poll for event") {
                        let term_event = event::read().expect("unable to read event");
                        sender
                            .send(InputEvent::Input(term_event))
                            .expect("failed to send event")
                    }

                    if last_tick.elapsed() >= tick_rate {
                        // Send might fail due to a closed receiver
                        // (has something to do with drop order?)
                        sender.send(InputEvent::Tick).ok();
                        // .expect("failed to send tick event");
                        last_tick = Instant::now();
                    }
                }
            })
        };
        Self {
            sender: ui_event_sender,
            receiver: ui_event_receiver,
            handler,
        }
    }

    pub fn next(&self) -> Result<InputEvent<CrosstermEvent>> {
        Ok(self.receiver.recv()?)
    }
}

fn map_term_event(event: &CrosstermEvent) -> Option<Action> {
    match event {
        CrosstermEvent::Key(e) => {
            if e.kind == event::KeyEventKind::Press {
                match e.code {
                    KeyCode::Right => Some(Action::IncBPM),
                    KeyCode::Left => Some(Action::DecBPM),
                    KeyCode::Up => Some(Action::IncVolume),
                    KeyCode::Down => Some(Action::DecVolume),
                    KeyCode::Char('k') => Some(Action::IncTotalBeats),
                    KeyCode::Char('j') => Some(Action::DecTotalBeats),
                    KeyCode::Esc | KeyCode::Char('q') => Some(Action::Quit),
                    KeyCode::Char('c') => {
                        if e.modifiers == KeyModifiers::CONTROL {
                            Some(Action::Quit)
                        } else {
                            None
                        }
                    }
                    _ => None, // ignore other key presses
                }
            } else {
                None // ignore KeyEventKind::Release on windows
            }
        }
        _ => None,
    }
}

pub struct Tui {
    /// Interface to the Terminal.
    terminal: CrosstermTerminal,
    pub ui_event_capturer: UIEventCapturer,
}

impl Tui {
    /// Constructs a new instance of [`Tui`].
    pub fn new(terminal: CrosstermTerminal, ui_event_capturer: UIEventCapturer) -> Self {
        Self {
            terminal,
            ui_event_capturer,
        }
    }

    /// Initializes the terminal interface.
    ///
    /// It enables the raw mode and sets terminal properties.
    pub fn enter(&mut self) -> Result<()> {
        terminal::enable_raw_mode()?;
        crossterm::execute!(io::stderr(), EnterAlternateScreen, EnableMouseCapture)?;

        // Define a custom panic hook to reset the terminal properties.
        // This way, you won't have your terminal messed up if an unexpected error happens.
        let panic_hook = panic::take_hook();
        panic::set_hook(Box::new(move |panic| {
            Self::reset().expect("failed to reset the terminal");
            panic_hook(panic);
        }));

        self.terminal.hide_cursor()?;
        self.terminal.clear()?;
        Ok(())
    }

    fn reset() -> Result<()> {
        terminal::disable_raw_mode()?;
        crossterm::execute!(io::stderr(), LeaveAlternateScreen, DisableMouseCapture)?;
        Ok(())
    }

    pub fn exit(&mut self) -> Result<()> {
        Self::reset()?;
        self.terminal.show_cursor()?;
        Ok(())
    }

    pub fn draw(&mut self, app: &mut App) -> Result<()> {
        self.terminal.draw(|frame| render(app, frame))?;
        Ok(())
    }
}

pub fn render(app: &App, f: &mut Frame) {
    let bpm = app.param.bpm.load(Ordering::Relaxed);
    let volume = app.param.volume.load(Ordering::Relaxed);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
        ])
        .split(f.size());

    let title = Paragraph::new(Text::styled("Cory Metronome", Style::default()))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default()),
        );

    let bpm_gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title("BPM (←/→)"))
        .gauge_style(Style::default().fg(Color::White).bg(Color::Black))
        .ratio(((bpm - MIN_BPM) / (MAX_BPM - MIN_BPM)).clamp(0.0, 1.0))
        .label(format!("{}/{}", bpm, MAX_BPM));

    let beat_gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title("Beat (j/k)"))
        .gauge_style(Style::default().fg(Color::White).bg(Color::Black))
        .ratio((app.beat_count as f64 / app.total_beats as f64).clamp(0.0, 1.0))
        .label(format!("{}/{}", app.beat_count, app.total_beats));

    let volume_gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title("Volume (↑/↓)"))
        .gauge_style(Style::default().fg(Color::White).bg(Color::Black))
        .ratio(volume);

    let desc = Paragraph::new(Text::styled(
        "Press (q) or (Ctrl-C) to quit",
        Style::default(),
    ))
    .alignment(Alignment::Left)
    .block(Block::default().style(Style::default()));

    f.render_widget(title, chunks[0]);
    f.render_widget(bpm_gauge, chunks[1]);
    f.render_widget(beat_gauge, chunks[2]);
    f.render_widget(volume_gauge, chunks[3]);
    f.render_widget(desc, chunks[4]);
}
