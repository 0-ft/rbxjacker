use serialport;
use std::time::Duration;

pub struct SerialLightOutput {
    port_name: String,
    port: Option<Box<dyn serialport::SerialPort>>,
    last_frame: Vec<u8>,
    pub frames_written: u64,
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
        self.port = serialport::new(self.port_name.as_str(), 921600)
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
        return SerialLightOutput {
            port: None,
            port_name: serial_port.clone(),
            frames_written: 0,
            last_frame: vec![0, 0],
        };
    }

    pub fn is_connected(&self) -> bool {
        return self.port.is_some();
    }

    pub fn write_frame(&mut self, frame: &Vec<u8>) -> bool {
        // if *frame == self.last_frame {
        //     return true;
        // }
        if let Some(ref mut port) = self.port {
            let checksum: u8 = frame.iter().sum();
            let info = ['\n' as u8, frame.len() as u8, checksum];
            let msg = [&info[..], &frame[..]].concat();
            let success = port.write_all(msg.as_slice()).is_ok();
            if !success {
                self.port = None;
            }
            self.frames_written += success as u64;
            // self.last_frame = frame.clone();
            return success;
        } else {
            self.connect();
            return false;
        }
    }
}
