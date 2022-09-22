use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::fs::read_to_string;
use std::time::UNIX_EPOCH;
use std::{collections::HashMap, time::SystemTime};
// mod rekordbox;
// use image::GenericImageView;
use crate::rekordbox::{FadersState, RekordboxUpdate, TrackState};
use image::{GenericImageView, Pixel};
// use std::time::{SystemTime, UNIX_EPOCH};
use itertools::{EitherOrBoth::*, Itertools};
use std::fmt;

const GRAPH_CHARS: [char; 9] = [' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

#[derive(Serialize, Deserialize)]
#[allow(non_snake_case)]
struct ShowJson {
    showName: String,
    path: String,
    frameRate: usize,
    numLights: usize,
}

#[derive(Serialize, Deserialize)]
struct ShowsJson {
    shows: Vec<ShowJson>,
}

#[derive(Clone, Debug)]
struct LightState {
    brightness: u8,
    // strobe_rate: u8,
    // strobe_fraction: u8,
}

impl fmt::Display for LightState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "(b: {})", self.brightness)
    }
}

struct Show {
    frames: Vec<Vec<LightState>>,
    length: i32,
    num_lights: usize,
    frame_rate: usize,
}

pub struct ShowsManager {
    json_path: String,
    shows: HashMap<String, Show>,
}

fn transpose<T>(v: &Vec<Vec<T>>) -> Vec<Vec<T>>
where
    T: Clone,
{
    assert!(!v.is_empty());
    (0..v[0].len())
        .map(|i| v.iter().map(|inner| inner[i].clone()).collect::<Vec<T>>())
        .collect()
}

pub struct FrameInfo {
    // pub track_1_title: String,
    // pub track_2_title: String,
    pub frame: Vec<u8>,
    pub has_track_1_show: bool,
    pub has_track_2_show: bool,
    // pub track_1_index: usize,
    // pub track_2_index: usize,
}

impl ShowsManager {
    fn load_show_file(path: &str, frame_rate: usize, num_lights: usize) -> Option<Show> {
        let img = image::open(path).ok()?;
        let pixels: Vec<LightState> = img
            .pixels()
            .map(|p| (p.2.to_rgb().0))
            .map(|rgb| LightState {
                brightness: rgb[0],
                // strobe_rate: rgb[1],
                // strobe_fraction: rgb[2],
            })
            .collect();
        let pixel_rows: Vec<Vec<LightState>> = pixels
            .chunks_exact(img.width() as usize)
            .map(|c| c.to_vec())
            .collect();
        // println!("pixel_rows {}x{}", pixel_rows.len(), pixel_rows[0].len());
        let frames: Vec<Vec<LightState>> = pixel_rows
            .chunks_exact(num_lights + 1) // vec of vecs of pixel row vecs
            .map(|c| c[0..num_lights].to_vec()) // remove empty pixel rows
            .flat_map(|row_rows| transpose(&row_rows))
            .collect();
        // println!("loaded show at '{}': {} lights, {} frames", path, frames[0].len(), frames.len());

        let length = frames.len() as i32;
        return Some(Show {
            // data: rows,
            frames,
            length,
            num_lights,
            frame_rate,
        });
    }

    // fn get_show_frame(show: &Show, index: i32) -> Vec<u8> {
    //     if 0 <= index && index < show.length {
    //         let time = SystemTime::now()
    //             .duration_since(UNIX_EPOCH)
    //             .unwrap()
    //             .as_micros();
    //         return show.frames[index as usize]
    //             .iter()
    //             .map(|v| {
    //                 if v.strobe_rate == 0 {
    //                     return v.brightness;
    //                 } else {
    //                     let strobe_multiplier = ((time % (v.strobe_rate as u128)) as u8)
    //                         < (v.strobe_fraction * v.strobe_rate / 255);
    //                     return strobe_multiplier as u8 * v.brightness;
    //                 }
    //             })
    //             .collect();
    //     }
    //     return vec![0; show.lights];
    // }

    fn get_show_frame_no_strobe(show: &Show, index: i32) -> Vec<u8> {
        if 0 <= index && index < show.length {
            return show.frames[index as usize]
                .iter()
                .map(|v| v.brightness)
                .collect();
        }
        return vec![0; show.num_lights];
    }

    pub fn load_shows(&mut self) {
        let json_content: String =
            read_to_string(self.json_path.as_str()).expect("Could not read shows JSON");
        let json: ShowsJson =
            serde_json::from_str(json_content.as_str()).expect("JSON was not well-formatted");
        let shows_num = json.shows.len();
        let shows: HashMap<String, Show> = json
            .shows
            .into_iter()
            .map(|s| {
                (
                    s.showName,
                    ShowsManager::load_show_file(s.path.as_str(), s.frameRate, s.numLights),
                )
            })
            .filter(|(_title, show)| show.is_some())
            .map(|(title, show)| (title, show.unwrap()))
            .collect();
        println!(
            "loaded {} shows, failed to load {}",
            shows.len(),
            shows_num - shows.len()
        );
        self.shows = shows;
    }

    pub fn from_json(shows_json_path: &str) -> ShowsManager {
        let mut result = ShowsManager {
            shows: HashMap::new(),
            json_path: shows_json_path.to_string(),
        };
        result.load_shows();
        return result;
    }

    pub fn get_frame_for_state(&self, track: &TrackState) -> Option<Vec<u8>> {
        // println!("'{}'", title);

        // let track_slug = format!("{} - {}", &track.artist, &track.title);
        if let Some(track_show) = self
            .shows
            .get(&format!("{} - {}", &track.artist, &track.title))
        {
            let frame_index = (track.beat_offset * track_show.frame_rate as f64).floor() as i32
                % track_show.length;
            // println!("tt {} {}", track.title, frame_index);
            return Some(ShowsManager::get_show_frame_no_strobe(
                track_show,
                frame_index,
            ));
        } else if let Some(last_cue) = &track.last_cue {
            if let Some(last_cue_comment) = &last_cue.comment {
                if let Some(cue_show) = self.shows.get(last_cue_comment) {
                    let frame_index = ((track.beat_offset - last_cue.beat_offset)
                        * cue_show.frame_rate as f64)
                        .floor() as i32
                        % cue_show.length;
                    return Some(ShowsManager::get_show_frame_no_strobe(
                        cue_show,
                        frame_index,
                    ));
                }
            }
        }
        return None;
        // return Some(frame);
    }

    pub fn combine_frames(
        track_1_frame: Option<Vec<u8>>,
        track_2_frame: Option<Vec<u8>>,
        faders: &FadersState,
        // lights: usize,
    ) -> Vec<u8> {
        let track_1_multiplier = 1_f32.min(2. - 2. * faders.crossfader) * faders.track_1_fader;
        let track_2_multiplier = 1_f32.min(2. * faders.crossfader) * faders.track_2_fader;
        if track_1_frame.is_some() && track_2_frame.is_some() {
            let track_1_frame = track_1_frame.unwrap();
            let track_2_frame = track_2_frame.unwrap();
            return track_1_frame
                .iter()
                .map(|x| ((*x as f32) * track_1_multiplier) as u8)
                .zip_longest(
                    track_2_frame
                        .iter()
                        .map(|x| ((*x as f32) * track_2_multiplier) as u8),
                )
                .map(|pair| match pair {
                    Both(l, r) => std::cmp::min(255, l as u16 + r as u16) as u8,
                    Left(l) => l,
                    Right(r) => r,
                })
                .collect();
        } else if track_1_frame.is_some() {
            return track_1_frame
                .unwrap()
                .iter()
                .map(|x| ((*x as f32) * track_1_multiplier) as u8)
                .collect();
        } else if track_2_frame.is_some() {
            return track_2_frame
                .unwrap()
                .iter()
                .map(|x| ((*x as f32) * track_2_multiplier) as u8)
                .collect();
        }
        return vec![0; 0];
        // let out_frame = vec!([0; std::cmp::max(track_1_frame.e)])
    }

    pub fn get_frame_from_rekordbox_update(&self, rekordbox_update: &RekordboxUpdate) -> FrameInfo {
        let track_1_frame = self.get_frame_for_state(&rekordbox_update.track_1);
        let track_2_frame = self.get_frame_for_state(&rekordbox_update.track_2);
        let has_track_1_show = track_1_frame.is_some();
        let has_track_2_show = track_2_frame.is_some();
        let out_frame =
            ShowsManager::combine_frames(track_1_frame, track_2_frame, &rekordbox_update.faders);

        return FrameInfo {
            frame: out_frame,
            has_track_1_show,
            has_track_2_show,
        };
    }
}

pub fn levels_to_graph(levels: &Vec<u8>) -> String {
    // let d = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs_f32();
    return levels
        .iter()
        .map(|l| "█".truecolor((*l / 16) * (*l / 16), *l / 3, *l))
        .join("");

    // return levels
    //     .iter()
    //     .map(|l| GRAPH_CHARS[(*l / 32) as usize] as char)
    //     .collect();
}
