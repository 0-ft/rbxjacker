use crate::ableton::{load_patterns_from_als, LightingPattern};
use crate::rekordbox::RekordboxUpdate;
use colored::Colorize;
use itertools::Itertools;
use std::collections::HashMap;
use walkdir::WalkDir;

pub struct ShowsManager {
    shows: HashMap<String, LightingPattern>,
}

impl ShowsManager {
    pub fn from_directory(dir: &str) -> ShowsManager {
        let als_paths: Vec<String> = WalkDir::new(dir)
            .into_iter()
            .map(|e| e.unwrap())
            .filter(|e| {
                e.file_type().is_file() && e.file_name().to_str().unwrap().ends_with(".als")
            })
            .map(|e| e.path().to_str().unwrap().to_string())
            .collect();
        
        println!("found {} shows: {:?}", als_paths.len(), als_paths);

        let als_patterns: HashMap<String, HashMap<String, LightingPattern>> = als_paths
            .iter()
            .map(|path| (path.to_string(), load_patterns_from_als(path)))
            .collect();

        let mut all_patterns: HashMap<String, LightingPattern> = HashMap::new();
        for (path, map) in als_patterns.iter() {
            for (name, pattern) in map {
                if (all_patterns.insert(name.clone(), pattern.clone()).is_some()) {
                    panic!(
                        "Error: duplicate clip tag {}, found in the following files:\n{}",
                        name,
                        als_patterns
                            .iter()
                            .filter(|(path, map)| map.contains_key(name))
                            .map(|(path, _)| path)
                            .join("\n")
                    );
                }
            }
        }

        println!("Initialized ShowsManager, loaded {} patterns total", all_patterns.len());

        return ShowsManager {
            shows: all_patterns,
        };
    }

    pub fn get_frame(&self, show_name: String, time: f64) -> HashMap<String, f64> {
        let show = self.shows.get(&show_name);
        if let Some(show) = show {
            return show.at_time(time);
        } else {
            return HashMap::new();
        }
    }
}

pub const GRAPH_CHARS: [char; 9] = [' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

pub fn fader_to_char(fader: f32) -> char {
    return GRAPH_CHARS[(fader * 8.) as usize] as char;
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
