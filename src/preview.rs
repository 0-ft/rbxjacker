use nannou::prelude::*;

use crate::rekordbox::{RekordboxAccess, self};
use crate::shows::ShowsManager;

// pub struct FrameProvider {
// }

// impl FrameProvider {
//     pub fn get_frame_now(&self) -> Vec<u8> {
//         let rekordbox_update = self.rekordbox_access.get_update();
//         let out_frame = self.shows_manager.get_frame_from_rekordbox_update(rekordbox_update);
//         return out_frame;
//     }
// }

pub fn show_preview() {
    nannou::app(model).update(update).simple_window(view).run();
}

struct Model {
    rekordbox_access: RekordboxAccess,
    shows_manager: ShowsManager,
    frame: Vec<u8>,
    lights: Vec<(Point3, Point3)>,
}

// impl Model {
//     fn get_frame(&mut self) -> Vec<u8> {
//         return out_frame;
//     }
// }

fn light_from_pos_and_rot(pos: Point3) -> (Point3, Point3) {
    return (
        pt3(pos.x - 0.3, pos.y, pos.z),
        pt3(pos.x + 0.3, pos.y, pos.z),
    );
}

fn point_to_screen(pos: Point3, win: Rect<f32>) -> Point2 {
    return pt2(win.x() + (pos.x / 20.0) * win.w(), win.y() + (pos.z / 20.0) * win.h());
}

fn model(app: &App) -> Model {
    let shows_manager = ShowsManager::from_json("shows/shows.json");
    let rekordbox_access = RekordboxAccess::make();
    // let frame_provider = FrameProvider {
    //     shows_manager,
    //     rekordbox_access
    // };
    let lights = vec![
        light_from_pos_and_rot(pt3(-4.0, 10.0, -1.0)),
        light_from_pos_and_rot(pt3(-3.0, 10.0, -1.0)),
        light_from_pos_and_rot(pt3(-2.0, 10.0, -1.0)),
        light_from_pos_and_rot(pt3(-1.0, 10.0, -1.0)),
        light_from_pos_and_rot(pt3(0.0, 10.0, -1.0)),
        light_from_pos_and_rot(pt3(1.0, 10.0, -1.0)),
        light_from_pos_and_rot(pt3(2.0, 10.0, -1.0)),
        light_from_pos_and_rot(pt3(3.0, 10.0, -1.0)),
        light_from_pos_and_rot(pt3(-4.0, 10.0, 1.0)),
        light_from_pos_and_rot(pt3(-3.0, 10.0, 1.0)),
        light_from_pos_and_rot(pt3(-2.0, 10.0, 1.0)),
        light_from_pos_and_rot(pt3(-1.0, 10.0, 1.0)),
        light_from_pos_and_rot(pt3(0.0, 10.0, 1.0)),
        light_from_pos_and_rot(pt3(1.0, 10.0, 1.0)),
        light_from_pos_and_rot(pt3(2.0, 10.0, 1.0)),
        light_from_pos_and_rot(pt3(3.0, 10.0, 1.0)),
    ];
    return Model {
        rekordbox_access,
        shows_manager,
        frame: vec![0; 16],
        lights: lights,
    };
}

fn update(_app: &App, model: &mut Model, _update: Update) {
    // println!("updating");
    let rekordbox_update = model.rekordbox_access.get_update();
    if let Some(update) = rekordbox_update {
        let out_frame = model
        .shows_manager
        .get_frame_from_rekordbox_update(update);
        model.frame = out_frame;
    }
}

fn view(app: &App, model: &Model, frame: Frame) {
    let draw = app.draw();
    draw.background().color(BLACK);
    let win = app.window_rect();
    let w = win.w() / 16.0;
    // println!("{:?}", model.frame);
    for i in 0..16 {
        let (a, b) = (point_to_screen(model.lights[i].0, win), point_to_screen(model.lights[i].1, win));
            // .wh(a);
        draw.line()
            .weight(10.0)
            .caps_round()
            .color(nannou::color::gray(model.frame[i] as f32 / 255.0))
            .points(a, b);
            draw.text(i.to_string().as_str())
            .color(BLUE)
            .font_size(14)
            .xy(a);
    }
    draw.to_frame(app, &frame).unwrap();
}
