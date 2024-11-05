use std::env;
use std::error::Error;

mod rekordbox;
use rekordbox::RekordboxAccess;
mod gui;

use crate::gui::Tuber;

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    let rekordbox_access = RekordboxAccess::make(collection_xml_path);

    let mut tuber = Tuber::create(shows_manager, rekordbox_access, Box::new(mcp))
        .expect("Could not create tuber");
    tuber.tick_loop()
}
