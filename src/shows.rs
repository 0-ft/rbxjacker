use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::read_to_string;
// mod rekordbox;
// use image::GenericImageView;
use crate::rekordbox::RekordboxUpdate;
use image::{GenericImageView, Pixel};
use std::time::{SystemTime, UNIX_EPOCH};

use std::fmt;

const GRAPH_CHARS: [char; 9] = [' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

#[derive(Serialize, Deserialize)]
struct ShowJson {
    title: String,
    path: String,
    frameRate: usize,
}

#[derive(Serialize, Deserialize)]
struct ShowsJson {
    shows: Vec<ShowJson>,
}

#[derive(Clone, Debug)]
struct LightState {
    brightness: u8,
    strobe_rate: u8,
    strobe_fraction: u8,
}

impl fmt::Display for LightState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "(b: {})", self.brightness)
    }
}

impl LightState {
    pub fn to_string(&self) -> String {
        // We don't want to disclose the secret
        format!("(b: {})", &self.brightness)
    }
}

struct Show {
    // data: Vec<Vec<u8>>,
    frames: Vec<Vec<LightState>>,
    length: i32,
    lights: usize,
    frame_rate: usize,
}

pub struct ShowsManager {
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
    pub track_1_title: String,
    pub track_2_title: String,
    pub frame: Vec<u8>,
    // pub track_1_index: usize,
    // pub track_2_index: usize,
}

impl ShowsManager {
    fn load_show_file(path: &str, frame_rate: usize, num_lights: usize) -> Option<Show> {
        println!("{}", path);
        let img = image::open(path).ok()?;
        let pixels: Vec<LightState> = img
            .pixels()
            .map(|p| (p.2.to_rgb().0))
            .map(|rgb| LightState {
                brightness: rgb[0],
                strobe_rate: rgb[1],
                strobe_fraction: rgb[2],
            })
            .collect();
        let pixel_rows: Vec<Vec<LightState>> = pixels
            .chunks_exact(img.width() as usize)
            .map(|c| c.to_vec())
            .collect();
        println!("pixel_rows {}x{}", pixel_rows.len(), pixel_rows[0].len());
        let frames: Vec<Vec<LightState>> = pixel_rows
            .chunks_exact(num_lights+1) // vec of vecs of pixel row vecs
            .map(|c| c[0..num_lights].to_vec()) // remove empty pixel rows
            .flat_map(|row_rows| transpose(&row_rows))
            .collect();
            // .collect::<Vec<Vec<Vec<LightState>>>>()
            // .concat();
        // println!("t1ransp {}, h {}", frame_rows.len(), frame_rows[0].len());
        // let frame_rows_ext = frame_rows.concat();
        // let frames = transpose(&frame_rows);
        println!("show {}: {} lights, {} frames", path, frames[0].len(), frames.len());
        // println!("transp {}, h {}", frames.len(), frames[0].len());

        let length = frames.len() as i32;
        return Some(Show {
            // data: rows,
            frames: frames,
            length: length,
            lights: num_lights,
            frame_rate: frame_rate,
        });
    }

    fn get_show_frame(show: &Show, index: i32) -> Vec<u8> {
        if 0 <= index && index < show.length {
            let time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_micros();
            return show.frames[index as usize]
                .iter()
                .map(|v| {
                    if v.strobe_rate == 0 {
                        return v.brightness;
                    } else {
                        let strobe_multiplier = ((time % (v.strobe_rate as u128)) as u8)
                            < (v.strobe_fraction * v.strobe_rate / 255);
                        return strobe_multiplier as u8 * v.brightness;
                    }
                })
                .collect();
            // .data
            // .iter()
            // .map(|row| row[index as usize])
            // .collect();
            // return frame;
        }
        return vec![0; show.lights];
    }

    fn get_show_frame_no_strobe(show: &Show, index: i32) -> Vec<u8> {
        if 0 <= index && index < show.length {
            return show.frames[index as usize]
                .iter()
                .map(|v| v.brightness)
                .collect();
            // .data
            // .iter()
            // .map(|row| row[index as usize])
            // .collect();
            // return frame;
        }
        return vec![0; show.lights];
    }

    pub fn from_json(shows_json_path: &str) -> ShowsManager {
        let json_content: String =
            read_to_string(shows_json_path).expect("Could not read shows JSON");
        let json: ShowsJson =
            serde_json::from_str(json_content.as_str()).expect("JSON was not well-formatted");
        let shows: HashMap<String, Show> = json
            .shows
            .into_iter()
            .map(|s| {
                (
                    s.title,
                    ShowsManager::load_show_file(s.path.as_str(), s.frameRate, 13),
                )
            })
            .filter(|(_title, show)| show.is_some())
            .map(|(title, show)| (title, show.unwrap()))
            .collect();
        println!("loaded {} shows", shows.len());
        return ShowsManager { shows: shows };
    }

    pub fn get_frame_for_title(&self, title: &String, offset: f64) -> Option<Vec<u8>> {
        // println!("'{}'", title);
        let show = self.shows.get(title)?;
        let frame_index = (offset * show.frame_rate as f64).floor() as i32 % show.length;
        let frame = ShowsManager::get_show_frame_no_strobe(show, frame_index);
        return Some(frame);
    }

    pub fn combine_frames(
        track_1_frame: Option<Vec<u8>>,
        track_2_frame: Option<Vec<u8>>,
        crossfader: f32,
        lights: usize,
    ) -> Vec<u8> {
        let left_frame = track_1_frame.unwrap_or(vec![0; lights]);
        let right_frame = track_2_frame.unwrap_or(vec![0; lights]);
        let out_frame = left_frame
            .iter()
            .zip(right_frame)
            .map(|(a, b)| *a as f32 * crossfader + b as f32 * (1.0 - crossfader))
            .map(|sum| if sum > 255.0 { 255 } else { sum as u8 })
            .collect();
        return out_frame;
    }

    pub fn get_frame_from_rekordbox_update(&self, rekordbox_update: RekordboxUpdate) -> FrameInfo {
        let track_1_frame = self.get_frame_for_title(
            &rekordbox_update.track_1_title,
            rekordbox_update.track_1_offset,
        );
        let track_2_frame = self.get_frame_for_title(
            &rekordbox_update.track_2_title,
            rekordbox_update.track_2_offset,
        );

        let out_frame = ShowsManager::combine_frames(
            track_1_frame,
            track_2_frame,
            rekordbox_update.crossfader,
            16,
        );

        return FrameInfo {
            track_1_title: String::from(rekordbox_update.track_1_title),
            track_2_title: String::from(rekordbox_update.track_2_title),
            frame: out_frame,
        };
    }
}

pub fn levels_to_graph(levels: &Vec<u8>) -> String {
    return levels
        .iter()
        .map(|l| GRAPH_CHARS[(*l / 32) as usize] as char)
        .collect();
}
