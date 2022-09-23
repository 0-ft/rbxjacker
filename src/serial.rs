use ratelimit::Limiter;
use serialport;
use std::time::Duration;

const SERIAL_BAUD_RATE: u32 = 115200; //921600;

pub struct SerialLightOutput {
    port_name: String,
    port: Option<Box<dyn serialport::SerialPort>>,
    last_frame: Vec<u8>,
    pub frames_written: u64,
    rate_limit: Limiter,
}

impl SerialLightOutput {
    pub fn prompt_port() -> Option<String> {
        let ports = serialport::available_ports().expect("No ports found!");
        let choices: Vec<String> = ports
            .iter()
            .enumerate()
            .map(|(i, p)| format!("{}: {}", i, p.port_name))
            .collect();
        if choices.len() < 1 {
            return None;
        }
        println!("choose a serial port:\n{}", choices.join("\n"));
        let mut choice = String::new();
        std::io::stdin().read_line(&mut choice).ok()?;
        let choice_index = choice.trim_end().parse::<usize>().ok()?;
        return Some(ports[choice_index].port_name.clone());
    }

    pub fn connect(&mut self) -> bool {
        // println!("opening port {}", self.port_name);
        self.port = serialport::new(self.port_name.as_str(), SERIAL_BAUD_RATE)
            .timeout(Duration::from_millis(100))
            .open()
            .ok();
        let success = self.port.is_some();
        if success {
            println!("successfully opened {}", self.port_name);
            // } else {
            //     println!("failed to open {}", self.port_name);
        }
        return success;
    }

    pub fn make(serial_port: &String) -> SerialLightOutput {
        let rate_limit = ratelimit::Builder::new()
            .capacity(1) //number of tokens the bucket will hold
            .quantum(1) //add one token per interval
            .interval(Duration::new(0, 2000000)) //add quantum tokens every 1 second
            .build();

        return SerialLightOutput {
            port: None,
            port_name: serial_port.clone(),
            frames_written: 0,
            last_frame: vec![0, 0],
            rate_limit
        };
    }

    pub fn is_connected(&self) -> bool {
        return self.port.is_some();
    }

    // pub fn write_frame_human(&mut self, frame: &Vec<u8>) -> bool {
    //     if *frame == self.last_frame {
    //         // println!("skipping frame");
    //         return false;
    //     } else {
    //         // println!("{:?} | {:?}", *frame, self.last_frame);
    //         self.last_frame = frame.clone();
    //     }
    //     if let Some(ref mut port) = self.port {
    //         // let info = ['\n' as u8, frame.len() as u8, checksum];
    //         // let msg = [&info[..], &frame[..]].concat();
    //         // let frame_adj: Vec<u8> = frame
    //         //     .iter()
    //         //     .map(|x| if *x == 255 { 255 } else { *x + 1 })
    //         //     .collect();
    //         // let checksum: u8 = frame_adj.iter().sum();
    //         // let msg = [&frame_adj[..], &[0 as u8]].concat();
    //         // println!("{:?}", msg);
    //         let msg = frame.iter().map(|x| x.to_string()).collect::<Vec<String>>().join(",") + "\n";
    //         let success = port.write_all(msg.as_bytes()).is_ok();
    //         if !success {
    //             println!("failed to write to serial, reconnecting");
    //             self.port = None;
    //         } else {
    //             self.frames_written += 1;
    //         }
    //     } else {
    //         self.connect();
    //     }
    //     return true;

    // }

    // returns whether new frame was written
    pub fn write_frame(&mut self, frame: &Vec<u8>) -> bool {
        if *frame == self.last_frame {
            // println!("skipping frame");
            return false;
        } else {
            // println!("{:?} | {:?}", *frame, self.last_frame);
            self.last_frame = frame.clone();
        }
        if let Some(ref mut port) = self.port {
            let frame_adj: Vec<u8> = frame
                .iter()
                .map(|x| if *x == 255 { 255 } else { *x + 1 })
                .collect();
            let msg = [&frame_adj[..], &[0 as u8]].concat();
            let success = port.write_all(msg.as_slice()).is_ok();
            if !success {
                println!("failed to write to serial, reconnecting");
                self.port = None;
            } else {
                self.frames_written += 1;
            }
        } else {
            self.connect();
        }
        self.rate_limit.wait();
        return true;
    }
}
