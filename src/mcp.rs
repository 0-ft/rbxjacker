// Copyright 2022 The mcp2221-rs Authors.
// This project is dual-licensed under Apache 2.0 and MIT terms.
// See LICENSE-APACHE and LICENSE-MIT for details.

use colored::Colorize;
use embedded_hal::blocking::{
    self,
    i2c::{Read, Write},
};
use mcp2221::Handle;
use pwm_pca9685::{Address, Channel, Pca9685};
use shared_bus::{BusManager, I2cProxy, NullMutex};
use std::{
    cell::RefCell,
    collections::HashMap,
    error::Error,
    sync::Mutex,
    thread,
    time::{self, Duration, SystemTime, UNIX_EPOCH},
};

use crate::{ew3::EW3_DEVICE_MAP, shows::LightingOutput};

pub struct MCPOutput {
    handle: Handle,
    // bus: BusManager<NullMutex<mcp2221::Handle>>,
    // pcas: Vec<Pca9685<I2cProxy<'a, NullMutex<mcp2221::Handle>>>>,
    addresses: HashMap<u8, u8>,
    map: HashMap<String, u32>,
}

impl MCPOutput {
    pub fn get_bus_addresses(handle: &mut Handle, to_check: Option<Vec<u8>>) -> Vec<u8> {
        let mut addresses: Vec<u8> = Vec::new();
        if let Some(to_check) = to_check {
            println!("Checking addresses: {:?}", to_check);
            return to_check
                .iter()
                .filter(|address| handle.read(**address, &mut [0u8]).is_ok())
                .map(|address| *address)
                .collect();
        } else {
            for base_address in (0..=127).step_by(16) {
                for offset in 0..=15 {
                    let address = base_address + offset;
                    match handle.read(address, &mut [0u8]) {
                        Ok(_) => {
                            addresses.push(address);
                            print!("0x{:02x}", address);
                        }
                        Err(_) => print!(" -- "),
                    }
                }
                println!();
            }
            addresses
        }
    }

    pub fn new(
        addresses: HashMap<u8, u8>,
        map: HashMap<String, u32>,
    ) -> Result<Self, Box<dyn Error>> {
        let mut config = mcp2221::Config::default();
        config.i2c_speed_hz = 400000;
        config.timeout = Duration::from_millis(20);
        let mut handle = mcp2221::Handle::open_first(&config)?;

        handle.check_bus()?;
        let bus_addresses =
            Self::get_bus_addresses(&mut handle, Some(addresses.values().cloned().collect()));
        let addresses_used = addresses
            .iter()
            .filter(|(_, address)| bus_addresses.contains(address))
            .map(|(offset, address)| (*offset, *address))
            .collect::<HashMap<u8, u8>>();
        if (addresses_used.len() != addresses.len()) {
            println!(
                "{}",
                format!(
                    "Not all requested addresses ({:?}) are available. Using {:?}",
                    addresses, addresses_used
                )
                .color("orange")
                .bold()
            );
        }
        handle = addresses_used.iter().fold(handle, |h, (offset, address)| {
            let mut pca = Pca9685::new(h, *address).unwrap();
            pca.set_prescale(100).unwrap();
            pca.enable().unwrap();
            pca.set_channel_on_off(Channel::C1, 0, 0);
            pca.destroy()
        });
        Ok(Self {
            handle,
            addresses: addresses_used,
            map,
        })
    }

    fn set_values_message(values: &[u16; 16]) -> [u8; 65] {
        let mut data = [0; 65];
        data[0] = 0x06;
        for (i, value) in values.iter().enumerate() {
            data[i * 4 + 1] = 0;
            data[i * 4 + 2] = 0;
            data[i * 4 + 3] = *value as u8;
            data[i * 4 + 4] = (*value >> 8) as u8;
        }
        return data;
    }

    pub fn set(&mut self, values: Vec<u16>) -> Result<(), pwm_pca9685::Error<mcp2221::Error>> {
        // let pcas = self
        //     .addresses
        //     .iter()
        //     .map(|address| {
        //         let mut pca = Pca9685::new(self.bus.acquire_i2c(), *address).unwrap();
        //         pca.set_prescale(100).unwrap();
        //         pca.enable().unwrap();
        //         pca
        //     })
        //     .collect::<Vec<Pca9685<I2cProxy<'_, NullMutex<mcp2221::Handle>>>>>();

        self.addresses.iter().for_each(|(offset, address)| {
            let mut device_vals = values
                .iter()
                .skip(*offset as usize)
                .take(16)
                .map(|v| *v)
                .collect::<Vec<u16>>();
            device_vals.resize(16, 0);
            // println!("offset {} @device {}: {:?}", offset, address, device_vals);
            let message = Self::set_values_message(&device_vals.try_into().unwrap());
            self.handle
                .write(*address, &message)
                .unwrap_or_else(|err| println!("Failed to write message: {}", err));
        });

        // values.chunks(16).enumerate().for_each(|(device, values)| {
        //     let mut vals16 = [0u16; 16];
        //     for (i, &value) in values.iter().enumerate() {
        //         vals16[i] = value;
        //     }
        //     let message = Self::set_values_message(&vals16);
        //     let device_address = *self
        //         .addresses
        //         .get(&((device * 16) as u8))
        //         .expect(format!("Too many values for device count {}", values.len()).as_str());
        //     self.handle
        //         .write(
        //             *self.addresses.get(device).expect(
        //                 format!("Too many values for device count {}", values.len()).as_str(),
        //             ),
        //             &message,
        //         )
        //         .expect("Failed to write message");
        // });
        Ok(())
    }
}

pub fn run_test() -> mcp2221::Result<()> {
    let mut mcp = MCPOutput::new(EW3_DEVICE_MAP(), HashMap::new()).unwrap();
    // mcp.add_devices(vec![0x50, 0x51, 0x52, 0x53]).unwrap();
    let mut t = 0;
    let mut tt = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    loop {
        let values = (0..64)
            .map(|i| ((((i * 2 + t) as f32 / 100.0).sin().powi(2)) * 4096.0) as u16)
            .collect::<Vec<u16>>();
        mcp.set(values);
        t = (t + 1) % 4096;
        if (t % 100 == 0) {
            let elapsed = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
                - tt;
            let fps = 100_000_000_000.0 / elapsed as f64;
            println!("fps: {:.1}", fps);
            tt = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
        }
    }

    Ok(())
}

impl LightingOutput for MCPOutput {
    fn write_frame(&mut self, values: &Vec<f64>) {
        self.set(
            values
                .iter()
                .map(|v| ((*v * 4095.) as u16).clamp(0, 4095))
                .collect(),
        )
        .unwrap_or_else(|e| {
            println!("error writing frame: {:?}", e);
        });
    }
    fn output_map(&self) -> &HashMap<String, u32> {
        return &self.map;
    }
}
