// Copyright 2022 The mcp2221-rs Authors.
// This project is dual-licensed under Apache 2.0 and MIT terms.
// See LICENSE-APACHE and LICENSE-MIT for details.

use embedded_hal::blocking::{
    self,
    i2c::{Read, Write},
};
use mcp2221::Handle;
use pwm_pca9685::{Address, Channel, Pca9685};
use shared_bus::{BusManager, I2cProxy, NullMutex};
use std::{
    cell::RefCell,
    error::Error,
    sync::Mutex,
    thread,
    time::{self, Duration, SystemTime, UNIX_EPOCH}, collections::HashMap,
};

use crate::shows::LightingOutput;

pub struct MCPOutput {
    handle: Handle,
    // bus: BusManager<NullMutex<mcp2221::Handle>>,
    // pcas: Vec<Pca9685<I2cProxy<'a, NullMutex<mcp2221::Handle>>>>,
    addresses: Vec<u8>,
    map: HashMap<String, u32>
}

impl MCPOutput {

    pub fn show_bus_addresses(handle: &mut Handle) {
        for base_address in (0..=127).step_by(16) {
            for offset in 0..=15 {
                let address = base_address + offset;
                match handle.read(address, &mut [0u8]) {
                    Ok(_) => print!("0x{:02x}", address),
                    Err(_) => print!(" -- "),
                }
            }
            println!();
        }
    }

    pub fn new(addresses: Vec<u8>, map: HashMap<String, u32>) -> Result<Self, Box<dyn Error>> {
        let mut config = mcp2221::Config::default();
        config.i2c_speed_hz = 400000;
        config.timeout = Duration::from_millis(20);
        let mut handle = mcp2221::Handle::open_first(&config)?;

        handle.check_bus()?;

        handle = addresses.iter().fold(handle, |h, address| {
            let mut pca = Pca9685::new(h, *address).unwrap();
            pca.set_prescale(100).unwrap();
            pca.enable().unwrap();
            pca.set_channel_on_off(Channel::C1, 0, 0);
            pca.destroy()
        });

        Ok(Self { handle, addresses, map })
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

        values.chunks(16).enumerate().for_each(|(device, values)| {
            let mut vals16 = [0u16; 16];
            for (i, &value) in values.iter().enumerate() {
                vals16[i] = value;
            }
            let message = Self::set_values_message(&vals16);
            self.handle
                .write(
                    *self.addresses.get(device).expect(
                        format!("Too many values for device count {}", values.len()).as_str(),
                    ),
                    &message,
                )
                .expect("Failed to write message");
        });
        Ok(())
    }
}

pub fn run_test() -> mcp2221::Result<()> {
    let mut mcp = MCPOutput::new(vec![0x44, 0x41, 0x43, 0x42, 0x40], HashMap::new()).unwrap();
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
        self.set(values.iter().map(|v| ((*v * 4095.) as u16).clamp(0, 4095)).collect()).unwrap_or_else(|e| {
            println!("error writing frame: {:?}", e);
        });
    }
    fn output_map(&self) -> &HashMap<String, u32> {
        return &self.map;
    }
}