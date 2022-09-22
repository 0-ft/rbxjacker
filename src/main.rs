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

// fn display_loop(mut rekordbox_access: RekordboxAccess, shows_manager: ShowsManager) {
//     // let delay = time::Duration::from_micros(2);
//     let mut i: i64 = 0;
//     let mut start = time::Instant::now();
//     loop {
//         let maybe_frame = rekordbox_access.get_update().and_then(|rekordbox_update| {
//             // println!("got update");
//             Some(shows_manager.get_frame_from_rekordbox_update(&rekordbox_update))
//         });

//         let frame_chars =
//             maybe_frame.map_or(String::from("none"), |frame| levels_to_graph(&frame.frame));
//         i += 1;
//         if (i % 10 == 0) {
//             println!("{}, {:?}", frame_chars, start.elapsed());
//             start = time::Instant::now()
//         }
//     }
// }

fn adjust_levels(frame: &Vec<u8>) -> Vec<u8> {
    return frame.iter().map(|l| *l).collect();
}

fn output_loop(
    mut rekordbox_access: RekordboxAccess,
    mut shows_manager: ShowsManager,
    mut serial_output: SerialLightOutput,
) {
    // let delay = time::Duration::from_micros(2);
    let mut i: i64 = 0;
    let mut last_fw = 0;
    let mut start = time::Instant::now();
    loop {
        if let Some(rekordbox_update) = rekordbox_access.get_update() {
            let frame = shows_manager.get_frame_from_rekordbox_update(&rekordbox_update);
            let frame_written = serial_output.write_frame(&adjust_levels(&frame.frame));
            // let tracks_display = format!(
            //     "{} {}",
            //     rekordbox_update.track_1,
            //     rekordbox_update.track_2,
            // );
            i += 1;
            if i % 1000 == 0 {
                // let frame_chars: String = out_frame.map_or(String::from("none"), |frame| levels_to_graph(&frame));
                let frame_chars = levels_to_graph(&frame.frame);
                println!(
                    "{} {} {} || {} {} || serial: {} ({} frames written)",
                    frame_chars,
                    rekordbox_update.track_1,
                    if frame.has_track_1_show { "✔️" } else { "❌" },
                    rekordbox_update.track_2,
                    if frame.has_track_2_show { "✔️" } else { "❌" },
                    serial_output.is_connected(),
                    serial_output.frames_written,
                    // rekordbox_access.is_attached(),
                    // (serial_output.frames_written - last_fw) as f64 / (start.elapsed().as_micros() / 1000_000) as f64,
                );
                start = time::Instant::now();
                last_fw = serial_output.frames_written;
                if i % 500000 == 0 {
                    println!("reloading shows");
                    shows_manager.load_shows();
                }
            }
        }
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
}

fn main() {
    // println!("{:?}", ShowsManager::combine_frames(Some(vec![1, 255, 4]), Some(vec![2, 8]), 0.5));
    let args: Vec<String> = env::args().collect();
    let shows_manager = ShowsManager::from_json("shows/shows.json");
    let collection_xml_path = args.get(1).expect("rekordbox collection xml path required");
    let rekordbox_access = RekordboxAccess::make(collection_xml_path);
    // rekordbox_access
    // .attach()
    // .expect("could not attach to rekordbox");
    // let port = SerialLightOutput::prompt_port().or(Some(String::from(&args[1]))).unwrap();
    let port = args
        .get(2)
        .and_then(|a| Some(String::from(a)))
        .or_else(|| SerialLightOutput::prompt_port());
    let mut serial_output = SerialLightOutput::make(&port.expect("no serial port found"));
    serial_output.connect();

    println!("finished setup");
    // show_in_window();
    // show_preview();
    output_loop(rekordbox_access, shows_manager, serial_output)
    // display_loop(rekordbox_access, shows_manager);
}
