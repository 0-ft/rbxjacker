use std::time;
mod shows;
use shows::{levels_to_graph, ShowsManager};
use std::env;

mod rekordbox;
use rekordbox::RekordboxAccess;

// mod preview;
// use preview::show_preview;

// mod ggwin;
// use ggwin::show_in_window;

mod serial;
use serial::SerialLightOutput;

fn display_loop(mut rekordbox_access: RekordboxAccess, shows_manager: ShowsManager) {
    // let delay = time::Duration::from_micros(2);
    let mut i: i64 = 0;
    // let empty_show = vec![vec![0; 1]; 16];
    let mut start = time::Instant::now();
    loop {
        let maybe_frame = rekordbox_access.get_update().and_then(|rekordbox_update| {
            // println!("got update");
            Some(shows_manager.get_frame_from_rekordbox_update(rekordbox_update))
        });

        let frame_chars =
            maybe_frame.map_or(String::from("none"), |frame| levels_to_graph(&frame.frame));
        i += 1;
        if (i % 10 == 0) {
            println!("{}, {:?}", frame_chars, start.elapsed());
            start = time::Instant::now()
        }
    }
}

fn output_loop(
    mut rekordbox_access: RekordboxAccess,
    shows_manager: ShowsManager,
    mut serial_output: SerialLightOutput,
) {
    // let delay = time::Duration::from_micros(2);
    let mut i: i64 = 0;
    let mut start = time::Instant::now();
    loop {
        let maybe_frame = rekordbox_access.get_update().and_then(|rekordbox_update| {
            // println!("got update");
            Some(shows_manager.get_frame_from_rekordbox_update(rekordbox_update))
        });
        let frame_chars = maybe_frame.map_or(String::from("none"), |frame| {
            serial_output.write_frame(&frame.frame);
            // println!(
            //     "left show: {} (f{}), right show: {}, (f{})",
            //     frame.track_1_title, 0, frame.track_2_title, 0
            // );
            return levels_to_graph(&frame.frame);
        });

        i += 1;
        if i % 1000 == 0 {
            // let frame_chars: String = out_frame.map_or(String::from("none"), |frame| levels_to_graph(&frame));
            println!(
                "serial: {}, serial frames written: {}, rekordbox attached: {}, fps: {:?}, frame: {}",
                serial_output.is_connected(),
                serial_output.frames_written,
                rekordbox_access.is_attached(),
                1000_000_000 / start.elapsed().as_micros(),
                frame_chars
            );
            start = time::Instant::now()
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let shows_manager = ShowsManager::from_json("shows/shows.json");
    let collection_xml_path = args.get(1).expect("rekordbox collection xml path required");
    let rekordbox_access = RekordboxAccess::make(collection_xml_path);
    // rekordbox_access
    // .attach()
    // .expect("could not attach to rekordbox");
    let port = args
        .get(2)
        .and_then(|a| Some(String::from(a)))
        .or_else(|| SerialLightOutput::prompt_port());
    // let port = SerialLightOutput::prompt_port().or(Some(String::from(&args[1]))).unwrap();
    // let mut serial_output = SerialLightOutput::make(&port.unwrap());
    // serial_output.connect();
    println!("finished setup");
    // show_in_window();
    // show_preview();
    // output_loop(rekordbox_access, shows_manager, serial_output)
    display_loop(rekordbox_access, shows_manager);
}
