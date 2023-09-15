use byteorder::*;
use minidom::Element;
use process_list::for_each_module;
use read_process_memory::*;
use std::{convert::TryInto, fmt};
use sysinfo::{PidExt, ProcessExt, SystemExt};
use roxmltree::Document;

use crate::shows::GRAPH_CHARS;

// const TRACK_1_OFFSET: [u32; 6] = [0x03FB2B08, 0x0, 0x240, 0x78, 0x108, 0x148];
const TRACK_1_OFFSET: [u32; 4] = [0x03FB2B08, 0x0, 0x230, 0x148];
const TRACK_2_OFFSET: [u32; 4] = [0x03FB2B08, 0x8, 0x230, 0x148];

const TRACK_1_TITLE: [u32; 5] = [0x03FA6B10, 0x780, 0x170, 0x0, 0x0];
const TRACK_2_TITLE: [u32; 3] = [0x03F4D188, 0x318, 0x0];

const TRACK_1_ARTIST: [u32; 4] = [0x03FB1A50, 0xB0, 0x140, 0x0];
const TRACK_2_ARTIST: [u32; 5] = [0x03FA6B10, 0x788, 0xF8, 0x118, 0x0];

const TRACK_1_ID: [u32; 4] = [0x03F71650, 0x158, 0x0, 0x34];
const TRACK_2_ID: [u32; 2] = [0x03F93898, 0x200];

// const CROSSFADER: [u32; 7] = [0x03F4C1A0, 0x208, 0x20, 0x150, 0x0, 0x468, 0x28];
const CROSSFADER: [u32; 8] = [0x03FA7740, 0x8, 0x180, 0x28, 0x150, 0x0, 0x468, 0x28];

const TRACK_1_FADER: [u32; 8] = [0x03FA7740, 0x8, 0x180, 0x28, 0x150, 0x0, 0x410, 0x28];
// const TRACK_1_FADER: [u32; 1] = [0x03F7E3CC];
// const TRACK_2_FADER: [u32; 1] = [0x03F7E3D4];
const TRACK_2_FADER: [u32; 8] = [0x03FA7740, 0x8, 0x180, 0x28, 0x150, 0x8, 0x410, 0x28];

// // #[inline]
// fn vec_to_arr<T, const N: usize>(v: Vec<T>) -> [T; N] {
//     v.try_into()
//         .unwrap_or_else(|v: Vec<T>| panic!("Expected a Vec of length {} but it was {}", N, v.len()))
// }

#[inline]
fn le_f64(bytes: Vec<u8>) -> f64 {
    return byteorder::LittleEndian::read_f64(bytes.as_slice());
}

fn le_u64(bytes: Vec<u8>) -> u64 {
    return byteorder::LittleEndian::read_u64(bytes.as_slice());
}

fn le_u32(bytes: Vec<u8>) -> u32 {
    return byteorder::LittleEndian::read_u32(bytes.as_slice());
}

#[inline]
fn le_f32(bytes: Vec<u8>) -> f32 {
    return byteorder::LittleEndian::read_f32(bytes.as_slice());
}

fn modules_by_name(pid: sysinfo::Pid) -> Option<Vec<(String, usize)>> {
    let mut modules = Vec::new();
    for_each_module(pid.as_u32(), |(module_base, _size), name| {
        let name = name
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .trim_matches(char::from(0));
        modules.push((String::from(name), module_base));
    })
    .ok()?;
    return Some(modules);
}

fn open_module(name: &str, module_name: &str) -> Option<ModuleHandle> {
    let mut system = sysinfo::System::new_all();
    system.refresh_all();

    let mut processes = system.processes_by_exact_name(name);
    let proc = processes.next()?;

    let pid = proc.pid();
    // println!("found process {:?}", pid);
    let modules = modules_by_name(pid)?;
    let module = modules.into_iter().find(|m| m.0.eq(module_name))?;

    // println!("found module at {:#x?}", module.1);
    let handle = pid.as_u32().try_into().ok()?;
    return Some(ModuleHandle {
        process_handle: handle,
        module_base: module.1,
    });
}

// fn open_module_2(name: &str, module_name: &str) -> Option<(ProcessHandle, usize)> {
//     let process = Process::from_name("rekordbox.exe").ok()?;
//     let module = process.module("rekordbox.exe").ok()?;
//     return (process.handle(), module.handle() as usize);
// }

struct ModuleHandle {
    process_handle: ProcessHandle,
    module_base: usize,
}

impl ModuleHandle {}

#[derive(Debug, Clone)]
pub struct TrackState {
    pub title: String,
    pub artist: String,
    pub id: u32,
    pub beat_offset: f64,
    pub last_cue: Option<XmlCueInfo>,
}

fn truncate(s: &str, max_chars: usize) -> String {
    match s.char_indices().nth(max_chars) {
        None => s.to_string(),
        Some((idx, _)) => format!("{}...", &s[..idx - 3]),
    }
}

impl fmt::Display for TrackState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let time_info = match &self.last_cue {
            Some(cue) => format!(
                " @ {}+{:.1}",
                cue.comment.as_ref().unwrap_or(&"no comment".to_string()),
                self.beat_offset - cue.beat_offset
            ),
            None => format!(" @ {:.1}", self.beat_offset),
        };
        // self.last_cue.as_ref().map_or("❌".to_string(), |cue| {
        //     format!(
        //         "✔️ {}+{:.1}",
        //         cue.comment.as_ref().unwrap_or(&"no comment".to_string()),
        //         self.beat_offset - cue.beat_offset
        //     )
        //     .to_string()
        // });
        write!(
            f,
            "{} '{}'{}",
            self.id,
            // rekordbox_update.track_1.artist,
            truncate(self.title.as_str(), 16),
            time_info
        )
    }
}

#[derive(Debug, Clone)]
pub struct FadersState {
    pub track_1_fader: f32,
    pub track_2_fader: f32,
    pub crossfader: f32,
}

impl ToString for FadersState {
    fn to_string(&self) -> String {
        return format!(
            "{} {}{}{} {}",
            GRAPH_CHARS[(self.track_1_fader * 8.) as usize],
            "─".repeat((self.crossfader * 8.).round() as usize),
            "■",
            "─".repeat(8 - (self.crossfader * 8.).round() as usize),
            GRAPH_CHARS[(self.track_2_fader * 8.) as usize]
        );
    }
}

#[derive(Debug, Clone)]
pub struct RekordboxUpdate {
    pub track_1: TrackState,
    pub track_2: TrackState,
    pub faders: FadersState,
}

pub struct RekordboxAccess {
    handle: Option<ModuleHandle>,
    track_1_title_address: CachedPointerChain,
    track_1_artist_address: CachedPointerChain,
    track_1_id_address: CachedPointerChain,
    track_1_offset_address: CachedPointerChain,
    track_2_title_address: CachedPointerChain,
    track_2_artist_address: CachedPointerChain,
    track_2_id_address: CachedPointerChain,
    track_2_offset_address: CachedPointerChain,
    track_1_fader_address: CachedPointerChain,
    track_2_fader_address: CachedPointerChain,
    crossfader_address: CachedPointerChain,
    xml_tracks: Vec<XmlTrackInfo>,
}

impl RekordboxAccess {
    pub fn make(collection_xml_path: &String) -> RekordboxAccess {
        let rekordbox_access = RekordboxAccess {
            handle: None,
            track_1_title_address: CachedPointerChain::make(TRACK_2_TITLE.to_vec()),
            track_1_artist_address: CachedPointerChain::make(TRACK_1_ARTIST.to_vec()),
            track_1_id_address: CachedPointerChain::make(TRACK_1_ID.to_vec()),
            track_1_offset_address: CachedPointerChain::make(TRACK_1_OFFSET.to_vec()),
            track_2_title_address: CachedPointerChain::make(TRACK_2_TITLE.to_vec()),
            track_2_artist_address: CachedPointerChain::make(TRACK_2_ARTIST.to_vec()),
            track_2_id_address: CachedPointerChain::make(TRACK_2_ID.to_vec()),
            track_2_offset_address: CachedPointerChain::make(TRACK_2_OFFSET.to_vec()),
            track_1_fader_address: CachedPointerChain::make(TRACK_1_FADER.to_vec()),
            track_2_fader_address: CachedPointerChain::make(TRACK_2_FADER.to_vec()),
            crossfader_address: CachedPointerChain::make(CROSSFADER.to_vec()),
            xml_tracks: parse_rekordbox_xml(collection_xml_path).unwrap_or(Vec::new()),
        };
        return rekordbox_access;
    }

    pub fn attach(&mut self) -> Result<(), &'static str> {
        self.handle = open_module("rekordbox.exe", "rekordbox.exe");
        if self.handle.is_some() {
            println!("attached to rekordbox");
            return Ok(());
        } else {
            return Err("could not attach to rekordbox");
        }
    }

    pub fn is_attached(&self) -> bool {
        return self.handle.is_some();
    }

    fn get_last_cue(&self, track: &TrackState) -> Option<XmlCueInfo> {
        let xml_track = self.xml_tracks.iter().find(|track_info| {
            // let foundtrack = track_info.title == track.title && track_info.artist == track.artist;
            // let foundtrack = track_info.title == track.title;
            // println!("have track for {}, {}: {}", track.title, track.artist, foundtrack);
            let foundtrack = track_info.id == track.id;
            return foundtrack;
        })?;
        // println!("trackinfo {:?}", xml_track);
        return xml_track
            .cues
            .iter()
            .filter(|cue| {
                cue.comment
                    .as_ref()
                    .map_or(false, |comment| comment.starts_with("EW"))
            })
            .filter(|cue| cue.beat_offset < track.beat_offset)
            .last()
            .map(std::clone::Clone::clone);
    }

    // fn map_raw_fader(raw_fader: f32) -> f32 {
    //     return raw_fader / 1023.;
    //     // println!("{:#}", f32::from_bits(raw_fader.to_bits() & 0xFFFFFF));
    //     // return f32::from_bits(raw_fader.to_bits() & 0xFFFFFF);
    //     let ranged = if raw_fader > 0.0 { (raw_fader - 0.875) } else { 0.0 };

    //     return 1. - (1. - ranged).powf(0.3);
    // }

    //TODO: make each track optional
    fn read_values(&mut self) -> Option<RekordboxUpdate> {
        let ref mut handle = self.handle.as_ref()?;

        // println!("gtrack33");
        let mut track_1 = TrackState {
            // title: self.track_1_title_address.get_string(&handle, false)?,
            // artist: self.track_1_artist_address.get_string(&handle, false)?,
            title: "unknown".to_string(),
            artist: "ua".to_string(),
            id: self.track_1_id_address.get_u32(&handle, false)?,
            beat_offset: self.track_1_offset_address.get_f64(&handle, true)?,
            last_cue: None,
        };

        // println!("gtrack1");
        track_1.last_cue = self.get_last_cue(&track_1);
        // let t1cuestring = track_1.last_cue.as_ref().map_or("no cue".to_string(), |cue| cue.comment.as_ref().unwrap_or(&"no comment".to_string()).to_string());
        // println!("track1cue: {:?}", track_1.last_cue);

        let mut track_2 = TrackState {
            // title: self.track_2_title_address.get_string(&handle, false)?,
            // artist: self.track_2_artist_address.get_string(&handle, false)?,
            title: "unknown".to_string(),
            artist: "ua".to_string(),
            id: self.track_2_id_address.get_u32(&handle, false)?,
            beat_offset: self.track_2_offset_address.get_f64(&handle, true)?,
            last_cue: None,
        };
        // println!("gtrack2");

        track_2.last_cue = self.get_last_cue(&track_2);

        let track_1_fader = self.track_1_fader_address.get_f32(handle, false)? / 1023.;
        let track_2_fader = self.track_2_fader_address.get_f32(handle, false)? / 1023.;
        let crossfader = self.crossfader_address.get_f32(handle, false)? / 1023.;
        return Some(RekordboxUpdate {
            track_1: track_1,
            track_2: track_2,
            faders: FadersState {
                track_1_fader,
                track_2_fader,
                crossfader,
            },
        });
    }

    pub fn get_update(&mut self) -> Option<RekordboxUpdate> {
        return self.read_values().or_else(|| {
            self.handle = None;
            println!("failed to read values from rekordbox, reattaching");
            self.attach();
            return None;
        });
    }
}

struct CachedPointerChain {
    chain: Vec<u32>,
    cached_addr: Option<usize>,
}

impl CachedPointerChain {
    fn make(chain: Vec<u32>) -> CachedPointerChain {
        return CachedPointerChain {
            chain: chain,
            cached_addr: None,
        };
    }

    fn follow_chain(&mut self, handle: &ModuleHandle) -> Option<usize> {
        let mut pos: usize = handle.module_base + self.chain[0] as usize;
        for offset in self.chain[1..].iter() {
            let bytes = copy_address(pos, 8, &handle.process_handle).ok()?;
            let pointer = byteorder::LittleEndian::read_i64(bytes.as_slice());
            pos = (pointer as usize) + (*offset as usize);
        }
        self.cached_addr = Some(pos);
        return Some(pos as usize);
    }
    fn get_bytes(
        &mut self,
        handle: &ModuleHandle,
        num_bytes: usize,
        try_without_cache: bool,
    ) -> Option<Vec<u8>> {
        if try_without_cache && self.cached_addr.is_some() {
            let result = self
                .cached_addr
                .and_then(|addr| copy_address(addr, num_bytes, &handle.process_handle).ok());
            if result.is_some() {
                return result;
            }
        }
        self.follow_chain(handle);

        let result = self
            .cached_addr
            .and_then(|addr| copy_address(addr, num_bytes, &handle.process_handle).ok());
        return result;
    }

    fn get_string(&mut self, handle: &ModuleHandle, try_without_cache: bool) -> Option<String> {
        let bytes = self.get_bytes(handle, 128, try_without_cache)?;
        let zero_index = bytes.iter().position(|x| *x == 0)?;
        return String::from_utf8(bytes[..zero_index].to_vec()).ok();
    }

    fn get_f64(&mut self, handle: &ModuleHandle, try_without_cache: bool) -> Option<f64> {
        return self
            .get_bytes(handle, 8, try_without_cache)
            .map(|bytes| le_f64(bytes));
    }

    fn get_f32(&mut self, handle: &ModuleHandle, try_without_cache: bool) -> Option<f32> {
        return self
            .get_bytes(handle, 4, try_without_cache)
            .map(|bytes| le_f32(bytes));
    }

    fn get_u64(&mut self, handle: &ModuleHandle, try_without_cache: bool) -> Option<u64> {
        return self.get_bytes(handle, 8, try_without_cache).map(|bytes| {
            return le_u64(bytes);
        });
    }

    fn get_u32(&mut self, handle: &ModuleHandle, try_without_cache: bool) -> Option<u32> {
        return self
            .get_bytes(handle, 4, try_without_cache)
            .map(|bytes| le_u32(bytes));
    }
}

#[derive(Debug, Clone)]
pub struct XmlCueInfo {
    pub beat_offset: f64,
    pub comment: Option<String>,
}

#[derive(Debug)]
pub struct XmlTrackInfo {
    pub title: String,
    pub artist: String,
    pub id: u32,
    pub cues: Vec<XmlCueInfo>,
}

fn parse_xml_cues(track_elem: roxmltree::Node) -> Vec<XmlCueInfo> {
    let tempo_points: Vec<(f64, f64)> = track_elem
        .children()
        .filter(|child| child.has_tag_name("TEMPO"))
        .filter_map(|child| {
            Some((
                child.attribute("Inizio")?.parse::<f64>().ok()?,
                child.attribute("Bpm")?.parse::<f64>().ok()?,
            ))
        })
        .collect();
    if let Some((start_seconds, tempo)) = tempo_points.get(0) {
        let beats_per_second = tempo / 60.0;
        let mut cues: Vec<XmlCueInfo> = track_elem
            .children()
            .filter(|child| child.has_tag_name("POSITION_MARK"))
            .filter_map(|child| {
                return Some(XmlCueInfo {
                    comment: child
                        .attribute("Name")
                        .filter(|name| !name.is_empty())
                        .map(str::to_string),
                    beat_offset: (child
                        .attribute("Start")?
                        .parse::<f64>()
                        .ok()
                        .expect("could not parse cue beat offset")
                        - start_seconds)
                        * beats_per_second,
                });
            })
            .collect();
        cues.sort_by(|a, b| a.beat_offset.partial_cmp(&b.beat_offset).unwrap());
        return cues;
    }
    return Vec::new();
}

fn parse_rekordbox_xml(path: &String) -> Option<Vec<XmlTrackInfo>> {
    // let file_contents: String = ;
    println!("loading rekordbox xml");
    let raw_xml = std::fs::read_to_string(path)
        .expect("failed to read xml");
    let doc = Document::parse(&raw_xml)
        .expect("failed to parse xml");
    // println!("rekordbox root: {:?}", root);
    let root = doc.root();
    let collection = root
        .descendants()
        .find(|n| n.has_tag_name("COLLECTION"))
        .expect("could not find collection tag");
    let xml_tracks: Vec<XmlTrackInfo> = collection
        .children()
        .filter(|n| n.has_tag_name("TRACK"))
        .map(|track_elem| {
            // println!("track elem: {:?}", track_elem);
            return XmlTrackInfo {
                title: track_elem
                    .attribute("Name")
                    .expect("could not parse track title")
                    .to_string(),
                artist: track_elem
                    .attribute("Artist")
                    .expect("could not parse track artist")
                    .to_string(),
                id: track_elem
                    .attribute("TrackID")
                    .expect("could not read track ID")
                    .parse::<u32>()
                    .expect("could not parse track ID"),
                cues: parse_xml_cues(track_elem),
            };
        })
        .collect();
    println!("Finished loading XML {}, found {} tracks with {} cue points", path, xml_tracks.len(), xml_tracks.iter().map(|track| track.cues.len()).sum::<usize>());
    return Some(xml_tracks);
}
