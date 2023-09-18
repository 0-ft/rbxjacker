use std::error::Error;
use std::io::{self, Stdout};
use std::time;
mod shows;
use ew3::EW3_DEVICE_MAP;
use shows::{levels_to_graph, ShowsManager};
use std::env;
use std::thread;

mod rekordbox;
use rekordbox::RekordboxAccess;

mod serial;
use serial::SerialLightOutput;

mod ableton;

mod gui;
mod mcp;
mod ew3;

use crate::ew3::EW3_LIGHT_MAP;
use crate::gui::Tuber;
use crate::shows::fader_to_char;
use crate::mcp::{run_test, MCPOutput};

fn adjust_levels(frame: &Vec<u8>) -> Vec<u8> {
    return frame.iter().map(|l| *l).collect();
}


fn main() -> Result<(), Box<dyn Error>> {
    env::set_var("RUST_BACKTRACE", "1");
    // run_test();
    // return Ok(());
    // println!("{:?}", ShowsManager::combine_frames(Some(vec![1, 255, 4]), Some(vec![2, 8]), 0.5));
    let args: Vec<String> = env::args().collect();
    let shows_manager =
        ShowsManager::from_directory(args.get(1).expect("shows directory required"));
    let collection_xml_path = args.get(2).expect("rekordbox collection xml path required");
    let rekordbox_access = RekordboxAccess::make(collection_xml_path);
    let port = args
        .get(2)
        .and_then(|a| Some(String::from(a)))
        .or_else(|| SerialLightOutput::prompt_port());
    // let mut serial_output = SerialLightOutput::make(&port.expect("no serial port found"));
    // serial_output.connect();
    let output_map = EW3_LIGHT_MAP();
    let mut mcp = MCPOutput::new(EW3_DEVICE_MAP(), output_map)?;

    // println!("finished setup");
    // show_in_window();
    // show_preview();
    
    let mut tuber = Tuber::create(
        shows_manager,
        rekordbox_access,
        Box::new(mcp),
    ).expect("Could not create tuber");
    tuber.tick_loop()
    // display_loop(rekordbox_access, shows_manager);
}
