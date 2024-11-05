use std::{
    collections::HashMap,
    error::Error,
    io::{self, Stdout},
    time::{self, Duration},
};

use colored::Colorize;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    prelude::*,
    widgets::{block::title, *},
};

type Frame<'a> = ratatui::Frame<'a, CrosstermBackend<Stdout>>;

pub struct Tuber {
    // ...
    rekordbox_access: RekordboxAccess,
    shows_manager: ShowsManager,
    output: Box<dyn LightingOutput>,
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl Tuber {
    pub fn create(
        shows_manager: ShowsManager,
        rekordbox_access: RekordboxAccess,
        output: Box<dyn LightingOutput>,
    ) -> Result<Tuber, Box<dyn Error>> {
        let mut terminal = Tuber::setup_terminal()?;
        Ok(Tuber {
            shows_manager,
            rekordbox_access,
            output,
            terminal,
        })
    }

    fn ui_track(f: &mut Frame, area: Rect, track: TrackState, title: &str) {
        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .padding(Padding::uniform(1));
        // f.render_widget(block, area);
        f.render_widget(
            Paragraph::new(format!(
                "Track {} @ {:.3}
Current Cue: {:?}",
                track.id, track.beat_offset, track.last_cue
            ))
            .block(block),
            area,
        );
    }

    fn draw_frame(f: &mut Frame, area: Rect, frame: &HashMap<String, f64>) {
        let block = Block::default()
            .title("Frame")
            .borders(Borders::ALL)
            .padding(Padding::uniform(1));
        // f.render_widget(block, area);
        // let mut text = String::new();
        let text = frame
            .iter()
            .map(|(name, val)| format!("{}: {:.3}", name, val))
            .sorted()
            .chunks(8)
            .into_iter()
            .map(|row| {
                // let vals = row.map(|(name, val)| (val * 255.) as u8).collect();
                // levels_to_graph(&vals)
                row.into_iter().join(" | ")
            })
            .join("\n");
        f.render_widget(Paragraph::new(text).block(block), area);
    }

    fn ui(
        &mut self,
        rekordbox_update: RekordboxUpdate,
        frame: &HashMap<String, f64>,
    ) -> Result<(), Box<dyn Error>> {
        self.terminal.draw(|f| {
            let rows = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
                .split(f.size());

            let cols = Layout::default()
                .direction(Direction::Horizontal)
                .margin(1)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
                .split(rows[0]);

            let left_track = cols[0];
            let right_track = cols[1];
            let both = rows[1];

            Self::ui_track(f, left_track, rekordbox_update.track_1, "LEFT TRACK");
            Self::ui_track(f, right_track, rekordbox_update.track_2, "RIGHT TRACK");
            Self::draw_frame(f, both, frame);
        })?;
        Ok(())
    }

    fn get_frame(&mut self, rekordbox_update: &RekordboxUpdate) -> HashMap<String, f64> {
        let mut states: Vec<(String, f64, f64)> = Vec::new();
        for track in vec![&rekordbox_update.track_1, &rekordbox_update.track_2] {
            if let Some(last_cue) = &track.last_cue {
                states.push((
                    last_cue.comment.clone().unwrap()[2..].to_string(),
                    track.beat_offset - last_cue.beat_offset,
                    1.,
                ));
            }
        }
        let frame = self.shows_manager.get_combined_frame(states);
        frame
    }

    // fn output(&mut self, frame: HashMap<String, f64>) -> Result<(), Box<dyn Error>> {

    //     // println!("{:?}", frame.get("C1L0").unwrap());
    //     self.output.write_frame_mapped(&frame);
    //     Ok(())
    // }

    pub fn tick(
        &mut self, // terminal: &mut Terminal<CrosstermBackend<Stdout>>,
        refresh_ui: bool,
    ) -> Result<(), Box<dyn Error>> {
        // let delay = time::Duration::from_micros(2);
        if let Some(rekordbox_update) = self.rekordbox_access.get_update() {
            // println!(
            //     "cueLeft: {:?} {:?} {}",
            //     trackLeft.last_cue, trackLeft.id, trackLeft.beat_offset
            // );
            // thread::sleep(time::Duration::from_millis(20));
            // println!("{:?} {:?}", rekordbox_update.track_1, rekordbox_update.track_2);
            let frame = self.get_frame(&rekordbox_update);
            
            self.output.write_frame_mapped(&frame);
            if (refresh_ui) {
                self.ui(rekordbox_update, &frame)?;
            }

            //     let frame = shows_manager.get_frame_from_rekordbox_update(&rekordbox_update);
            //     let frame_written = serial_output.write_frame(&adjust_levels(&frame.frame));
            //     // let tracks_display = format!(
            //     //     "{} {}",
            //     //     rekordbox_update.track_1,
            //     //     rekordbox_update.track_2,
            //     // );
            //     i += 1;
            //     if i % 1000 == 0 {
            //         // let frame_chars: String = out_frame.map_or(String::from("none"), |frame| levels_to_graph(&frame));
            //         let frame_chars = levels_to_graph(&frame.frame);
            //         println!(
            //             "{} {} {} ┃ {} {} ┃ {} ({} frames written) ┃ {}",
            //             frame_chars,
            //             rekordbox_update.track_1,
            //             ["❌", "✔️"][frame.has_track_2_show as usize],
            //             rekordbox_update.track_2,
            //             ["❌", "✔️"][frame.has_track_2_show as usize],
            //             ["connected", "not connected"][serial_output.is_connected() as usize],
            //             serial_output.frames_written,
            //             rekordbox_update.faders.to_string() // rekordbox_access.is_attached(),
            //                                                 // (serial_output.frames_written - last_fw) as f64 / (start.elapsed().as_micros() / 1000_000) as f64,
            //         );
            //         start = time::Instant::now();
            //         last_fw = serial_output.frames_written;
            //         // if i % 500000 == 0 {
            //         //     println!("reloading shows");
            //         //     shows_manager.load_shows();
            //         // }
            //     }
            // }
            // let maybe_frame = maybe_rekordbox_update.map(|rekordbox_update| {
            //     shows_manager.get_frame_from_rekordbox_update(rekordbox_update)
            // });
            // let maybe_frame = rekordbox_access.get_update().and_then(|rekordbox_update| {
            //     // println!("got update");
            //     Some(shows_manager.get_frame_from_rekordbox_update(rekordbox_update))
            // });
            // let frame_chars = maybe_frame.map_or(String::from("none"), |frame| {
            //     serial_output.write_frame(&frame.frame);
            //     return levels_to_graph(&frame.frame);
            // });
        }
        Ok(())
    }

    pub fn tick_loop(&mut self) -> Result<(), Box<dyn Error>> {
        let mut i: i64 = 0;
        let mut last_fw = 0;
        let mut start = time::Instant::now();
        // self.restore_terminal()?;
        loop {
            self.tick(i % 10 == 0)?;
            if event::poll(Duration::from_millis(1))? {
                if let Event::Key(key) = event::read()? {
                    if KeyCode::Char('q') == key.code {
                        break;
                    }
                }
            }
            i += 1;
        }
        self.restore_terminal()?;
        Ok(())
    }

    fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>, Box<dyn Error>> {
        let mut stdout = io::stdout();
        enable_raw_mode()?;
        execute!(stdout, EnterAlternateScreen)?;
        Ok(Terminal::new(CrosstermBackend::new(stdout))?)
    }

    fn restore_terminal(&mut self) -> Result<(), Box<dyn Error>> {
        disable_raw_mode()?;
        execute!(self.terminal.backend_mut(), LeaveAlternateScreen,)?;
        Ok(self.terminal.show_cursor()?)
    }
}
