use std::collections::HashMap;

pub fn EW3_LIGHT_MAP() -> HashMap<String, u32> {
    let mut map = HashMap::new();
    for col in 0..4 {
        for l in 0..8 {
            let name = format!("C{}L{}", col+1, l);
            map.insert(name, (col * 8 + l) as u32);
        }
    }
    for tube in 0..20 {
        let name = format!("T{}", tube);
        map.insert(name, 32 + tube);
    }
    // map.insert("STROBE FL".to_string(), 64);
    // map.insert("STROBE FR".to_string(), 65);
    // map.insert("STROBE BL".to_string(), 66);
    // map.insert("STROBE BR".to_string(), 67);

    // map.insert("L TERRAIN".to_string(), 68);
    // map.insert("R TERRAIN".to_string(), 69);
    // map.insert("DJ RIM L".to_string(), 70);
    // map.insert("DJ RIM R".to_string(), 71);

    // map.insert("TREES 1".to_string(), 72);
    // map.insert("TREES 2".to_string(), 73);
    // map.insert("TREES 3".to_string(), 74);
    // map.insert("TREES 4".to_string(), 75);
    map
}