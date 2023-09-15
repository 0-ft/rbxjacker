use flate2::read::GzDecoder;
use itertools::Itertools;
use roxmltree::Document;
use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::prelude::*;

use flo_curves::*;
use plotters::prelude::*;

#[derive(Debug, Clone, Copy)]
struct AutomationPoint {
    time: f64,
    value: f64,
    curve_control_1x: Option<f64>,
    curve_control_1y: Option<f64>,
    curve_control_2x: Option<f64>,
    curve_control_2y: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct LightingPattern {
    length: f64,
    envelopes: HashMap<String, Vec<bezier::Curve<Coord2>>>,
}

impl LightingPattern {
    pub fn at_time(&self, time: f64) -> HashMap<String, f64> {
        return self.envelopes.iter().map(
            |(name, envelope)| {
                (
                    name.to_string(),
                    sample_segments(time, envelope),
                )
            },
        ).collect();
    }
}

fn find_segment(time: f64, segments: &Vec<bezier::Curve<Coord2>>) -> Option<bezier::Curve<Coord2>> {
    // find the latest curve that starts at or before time
    let mut latest_curve: Option<bezier::Curve<Coord2>> = None;
    for curve in segments {
        if curve.start_point().0 <= time {
            latest_curve = Some(*curve);
        } else {
            break;
        }
    }
    return latest_curve;
}

fn sample_segments(time: f64, segments: &Vec<bezier::Curve<Coord2>>) -> f64 {
    let curve = find_segment(time, segments).unwrap();
    let t = (time - curve.start_point().0) / (curve.end_point().0 - curve.start_point().0);
    let point = curve.point_at_pos(t);
    return point.1;
}

fn multisample_segments(times: &[f64], segments: &Vec<bezier::Curve<Coord2>>) -> Vec<Coord2> {
    return times
        .iter()
        .map(|t| Coord2(*t, sample_segments(*t, segments)))
        .collect();
}

fn linspace(start: f64, end: f64, num_points: usize) -> Vec<f64> {
    assert!(
        num_points >= 2,
        "num_points must be greater than or equal to 2"
    );

    let step = (end - start) / (num_points as f64 - 1.0);
    (0..num_points).map(|i| start + step * (i as f64)).collect()
}

fn plot_segments(
    times: &[f64],
    segments: &Vec<bezier::Curve<Coord2>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let samples: Vec<Coord2> = multisample_segments(times, segments);

    let root = BitMapBackend::new("plot.png", (1600, 400)).into_drawing_area();
    root.fill(&WHITE)?;
    let mut chart = ChartBuilder::on(&root)
        .margin(5)
        .x_label_area_size(30)
        .y_label_area_size(30)
        .build_ranged(times[0]..*times.last().unwrap(), 0f32..1f32)?;

    chart.configure_mesh().draw()?;

    chart.draw_series(LineSeries::new(
        samples.iter().map(|p| (p.0 as f64, p.1 as f32)),
        &RED,
    ))?;

    chart
        .configure_series_labels()
        .background_style(&WHITE.mix(0.8))
        .border_style(&BLACK)
        .draw()?;

    Ok(())
}

fn find_envelopes(root: roxmltree::Node) -> HashMap<String, Vec<AutomationPoint>> {
    let mut map: HashMap<String, Vec<AutomationPoint>> = HashMap::new();

    for envelope in root
        .descendants()
        .filter(|n| n.has_tag_name("AutomationEnvelope"))
    {
        map.extend(process_envelope(envelope));
    }

    map
}

fn process_envelope(envelope: roxmltree::Node) -> HashMap<String, Vec<AutomationPoint>> {
    let mut map = HashMap::new();
    if let Some(pointee_node) = envelope.descendants().find(|n| n.has_tag_name("PointeeId")) {
        if let Some(pointee_id) = pointee_node.attribute("Value") {
            let mut events = Vec::new();

            for event in envelope
                .descendants()
                .filter(|n| n.has_tag_name("FloatEvent"))
            {
                // let id: u32 = event.attribute("Id").unwrap().parse().unwrap();
                let time: f64 = event.attribute("Time").unwrap().parse().unwrap();
                let value: f64 = event.attribute("Value").unwrap().parse().unwrap();
                let curve_control_1x: Option<f64> = event
                    .attribute("CurveControl1X")
                    .map(|s| s.parse().unwrap());
                let curve_control_1y: Option<f64> = event
                    .attribute("CurveControl1Y")
                    .map(|s| s.parse().unwrap());
                let curve_control_2x: Option<f64> = event
                    .attribute("CurveControl2X")
                    .map(|s| s.parse().unwrap());
                let curve_control_2y: Option<f64> = event
                    .attribute("CurveControl2Y")
                    .map(|s| s.parse().unwrap());

                events.push(AutomationPoint {
                    // id,
                    time,
                    value,
                    curve_control_1x,
                    curve_control_1y,
                    curve_control_2x,
                    curve_control_2y,
                });
            }

            map.insert(pointee_id.to_string(), events);
        }
    }
    map
}

fn construct_segments(points: &Vec<AutomationPoint>) -> Vec<bezier::Curve<Coord2>> {
    // iterate points pairwise and construct curves between them using control points
    let mut segments = Vec::new();
    for (a, b) in points.iter().tuple_windows() {
        let start = Coord2(a.time, a.value);
        let end = Coord2(b.time, b.value);
        let curve = match (
            a.curve_control_1x,
            a.curve_control_1y,
            a.curve_control_2x,
            a.curve_control_2y,
        ) {
            (None, None, None, None) => {
                let distance = end - start;
                let c1 = start + distance * 0.33;
                let c2 = start + distance * 0.66;
                bezier::Curve::from_points(start, (c1, c2), end)
            }
            (Some(c1x), Some(c1y), Some(c2x), Some(c2y)) => {
                let dx = end.0 - start.0;
                let dy = end.1 - start.1;
                let c1 = Coord2(start.0 + c1x * dx, start.1 + c1y * dy);
                let c2 = Coord2(start.0 + c2x * dx, start.1 + c2y * dy);
                bezier::Curve::from_points(start, (c1, c2), end)
            }
            _ => panic!("Invalid curve control points"),
        };
        segments.push(curve);
    }
    return segments;
}
// let builder: BezierPathBuilder<SimpleBezierPath> =
//     BezierPathBuilder::<SimpleBezierPath>::start(Coord2(points[0].time, points[0].value));
// points.into_iter().fold(builder, |builder, point| {
//     // add line if no control points, otherwise add curve. use matching to check.
//     builder.points.
//     match (
//         point.curve_control_1x,
//         point.curve_control_1y,
//         point.curve_control_2x,
//         point.curve_control_2y,
//     ) {
//         (None, None, None, None) => builder.line_to(Coord2(point.time, point.value)),
//         (Some(c1x), Some(c1y), Some(c2x), Some(c2y)) => builder.curve_to(
//             (Coord2(c1x, c1y), Coord2(c2x, c2y)),
//             Coord2(point.time, point.value),
//         ),
//         _ => panic!("Invalid curve control points"),
//     }
// });

// path_to_curves(builder.build()).collect()
// builder.build().to_curves()

fn read_gzip(filepath: &String) -> String {
    println!("Reading file: {}", filepath);
    let file = File::open(filepath).unwrap();
    let mut decoder = GzDecoder::new(file);
    let mut s = String::new();
    decoder.read_to_string(&mut s).unwrap();
    s
}

fn find_parameters(root: roxmltree::Node) -> HashMap<String, String> {
    let mut map: HashMap<String, String> = HashMap::new();

    for param in root
        .descendants()
        .filter(|n| n.has_tag_name("MxDFloatParameter"))
    {
        if let Some(name_node) = param.descendants().find(|n| n.has_tag_name("Name")) {
            if let Some(name_value) = name_node.attribute("Value") {
                for target in param
                    .descendants()
                    .filter(|n| n.has_tag_name("AutomationTarget"))
                {
                    if let Some(id_value) = target.attribute("Id") {
                        map.insert(id_value.to_string(), name_value.to_string());
                    }
                }
            }
        }
    }

    map
}

fn find_locators(root: roxmltree::Node) -> Vec<(f64, String)> {
    let mut locators: Vec<(f64, String)> = Vec::new();

    for locator in root.descendants().filter(|n| n.has_tag_name("Locator")) {
        if let Some(name_node) = locator.descendants().find(|n| n.has_tag_name("Name")) {
            if let Some(name_value) = name_node.attribute("Value") {
                if let Some(time_node) = locator.descendants().find(|n| n.has_tag_name("Time")) {
                    if let Some(time_value) = time_node.attribute("Value") {
                        locators.push((time_value.parse().unwrap(), name_value.to_string()));
                        // map.insert(name_value.to_string(), time_value.parse().unwrap());
                    }
                }
            }
        }
    }

    locators.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

    locators
}

fn envelope_between_times(
    start: f64,
    end: f64,
    envelope: &Vec<bezier::Curve<Coord2>>,
) -> Vec<bezier::Curve<Coord2>> {
    let mut curves = Vec::new();
    for curve in envelope.iter() {
        if (start <= curve.start_point().0 && curve.start_point().0 <= end)
            || (start <= curve.end_point().0 && curve.end_point().0 <= end)
        {
            let shifted = bezier::Curve::from_points(
                Coord2(curve.start_point.0 - start, curve.start_point.1),
                (
                    Coord2(curve.control_points.0 .0 - start, curve.control_points.0 .1),
                    Coord2(curve.control_points.1 .0 - start, curve.control_points.1 .1),
                ),
                Coord2(curve.end_point.0 - start, curve.end_point.1),
            );
            curves.push(shifted);
        }
    }
    curves
}

fn split_envelopes_by_locators(
    envelopes: &HashMap<String, Vec<bezier::Curve<Coord2>>>,
    locators: &Vec<(f64, String)>,
) -> HashMap<String, LightingPattern> {
    let mut map: HashMap<String, LightingPattern> = HashMap::new();

    for (start, end) in locators.iter().tuple_windows() {
        let envelope_cut: HashMap<String, Vec<bezier::Curve<Coord2>>> = envelopes.iter().map(|(name, envelope)| {
            (
                name.to_string(),
                envelope_between_times(start.0, end.0, envelope),
            )
        }).collect();
        map.insert(start.1.to_string(), LightingPattern {
            length: end.0 - start.0,
            envelopes: envelope_cut,
        });
    }
    map
}

fn merge_maps<T: Clone>(
    id_to_events: &HashMap<String, Vec<T>>,
    id_to_name: &HashMap<String, String>,
) -> HashMap<String, Vec<T>> {
    let mut name_to_events: HashMap<String, Vec<T>> = HashMap::new();

    for (id, name) in id_to_name {
        if let Some(events) = id_to_events.get(id) {
            name_to_events.insert(name.clone(), events.clone());
        }
    }

    name_to_events
}

pub fn load_patterns_from_als(filepath: &String) -> HashMap<String, LightingPattern> {
    let xml_str = read_gzip(filepath);

    let doc = Document::parse(&xml_str).unwrap();

    let envelopes = find_envelopes(doc.root_element());

    let parameters = find_parameters(doc.root_element());

    let envelope_map = merge_maps(&envelopes, &parameters);

    let curves_map = envelope_map.iter().map(|(name, points)| {
        (
            name.to_string(),
            construct_segments(points),
        )
    }).collect();

    let locators = find_locators(doc.root_element());

    let patterns = split_envelopes_by_locators(&curves_map, &locators);

    println!("{}: Found {} parameters, {} envelopes, {} locators. Loaded {} patterns.", filepath, parameters.len(), envelopes.len(), locators.len(), patterns.len());
    patterns
}

// fn main() {
//     let patterns = load_patterns_from_als(env::args().nth(1).unwrap());
//     let t1 = patterns.get("TEST1").unwrap();
//     let t1c1l0 = t1.envelopes.get("C1L0").unwrap();
//     plot_segments(linspace(0.0, t1.length, 400).as_slice(), t1c1l0);
//     println!("{:#?}", t1c1l0);
//     // let xml_str = read_gzip(env::args().nth(1).unwrap());

//     // let doc = Document::parse(&xml_str).unwrap();

//     // let envelopes = find_envelopes(doc.root_element());

//     // // println!("{:#?}", envelopes);

//     // let parameters = find_parameters(doc.root_element());
//     // // println!("{:#?}", parameters);

//     // let merged = merge_maps(envelopes, parameters);
//     // // println!("{:#?}", merged);

//     // let a = merged.get("C1L0").expect("Auto not found");
//     // // let seg = find_segment(61.0, a);
//     // // println!("{:#?}", seg);
//     // // println!("{:#?}", a);
//     // let segments: Vec<bezier::Curve<Coord2>> = construct_segments(a);
//     // // println!("{:#?}", segments);
//     // // println!("{:#?}", sample_segments(61.0, &segments));
//     // plot_segments(linspace(60.0, 62.0, 1000).as_slice(), &segments);

//     // let locators = find_locators(doc.root_element());
//     // println!("{:#?}", locators);
// }
