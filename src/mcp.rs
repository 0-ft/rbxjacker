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
    time::{self, Duration, SystemTime, UNIX_EPOCH},
};

pub struct MCPOutput {
    // handle: Handle,
    bus: BusManager<NullMutex<mcp2221::Handle>>,
    // pcas: Vec<Pca9685<I2cProxy<'a, NullMutex<mcp2221::Handle>>>>,
    addresses: Vec<u8>,
}

impl MCPOutput {
    pub fn new(addresses: Vec<u8>) -> Result<Self, Box<dyn Error>> {
        let mut config = mcp2221::Config::default();
        config.i2c_speed_hz = 115200;
        // For talking to a peripheral we might want a higher timeout, but for
        // scanning the bus, a short timeout is good since it allows us to scan all
        // addresses more quickly.
        config.timeout = Duration::from_millis(10);
        let mut handle = mcp2221::Handle::open_first(&config)?;

        // Set GPIO pin 0 high. This is useful if your I2C bus goes through a level
        // shifter and you need to enable that level shifter in order to use the I2C
        // bus. It also serves as an example of using GPIO.
        // let mut gpio_config = mcp2221::GpioConfig::default();
        // gpio_config.set_direction(0, mcp2221::Direction::Output);
        // gpio_config.set_value(0, true);
        // handle.configure_gpio(&gpio_config)?;

        // Before we start, SDA and SCL should be high. If they're not, then either
        // the pull-up resistors are missing, the bus isn't properly connected or
        // something on the bus is holding them low. In any case, we won't be able
        // to operate.
        handle.check_bus()?;
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
        let bus = shared_bus::BusManagerSimple::new(handle);
        Ok(Self {
            // handle,
            bus,
            addresses,
        })

        // println!("Connected to MCP2221: {}", handle.get_device_info()?);
        // let bus = shared_bus::BusManagerSimple::new(handle);
        // // let pcas = Vec::new();
        // let pcas = addresses.into_iter().map(|address| {
        //     let mut pca = Pca9685::new(bus.acquire_i2c(), address).unwrap();
        //     pca.set_prescale(100).unwrap();
        //     pca.enable().unwrap();
        //     pca
        // }).collect();
        // Ok(Self {
        //     // bus: &bus,
        //     pcas,
        // })
    }

    // pub fn add_devices(&'a mut self, addresses: Vec<u8>) -> Result<(), Box<dyn Error>> {

    //     Ok(())
    // }

    pub fn set_single(
        &mut self,
        device: usize,
        channel: Channel,
        value: u16,
    ) -> Result<(), pwm_pca9685::Error<mcp2221::Error>> {
        let i2c = self.bus.acquire_i2c();
        println!("got i2c");
        let mut pca = Pca9685::new(i2c, *self.addresses.get(device).unwrap())?;
        pca.set_prescale(100).unwrap();
        pca.enable().unwrap();
        // let pca = Pca9685::new(self.handle.acquire_i2c(), self.addresses[device]).unwrap();
        println!("setting {} to {}", device, value);
        pca.set_channel_on_off(channel, 0, value)?;
        Ok(())
    }

    // fn write_quad_register(
    //     &mut self,
    //     address: u8,
    //     value0: u16,
    //     value1: u16,
    // ) -> Result<(), Error<E>> {
    //     // if self.config.is_low(BitFlagMode1::AutoInc) {
    //     //     let config = self.config;
    //     //     self.write_mode1(config.with_high(BitFlagMode1::AutoInc))?;
    //     // }
    //     self.i2c
    //         .write(
    //             self.address,
    //             &[
    //                 address,
    //                 value0 as u8,
    //                 (value0 >> 8) as u8,
    //                 value1 as u8,
    //                 (value1 >> 8) as u8,
    //             ],
    //         )
    //         .map_err(Error::I2C)
    // }

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
        values.chunks(16).enumerate().for_each(|(i, values)| {
            let mut pca = Pca9685::new(self.bus.acquire_i2c(), *self.addresses.get(i).unwrap()).unwrap();
            pca.set_prescale(100).unwrap();
            pca.enable().unwrap();
            let mut vals16 = [0u16; 16];
            for (i, &value) in values.iter().enumerate() {
                vals16[i] = value;
            }
            pca.set_all_on_off(&[0u16; 16], &vals16).unwrap();
            // values.iter().enumerate().for_each(|(j, value)| {
            //     pca.set_channel_on_off((j % 16).try_into().unwrap(), 0, *value)
            //         .unwrap();
            // });
        });
        // values.iter().enumerate().for_each(|(i, value)| {
        //     // let pca = pcas.get(i / 16)
        //     //     .expect(
        //     //         format!(
        //     //             "Channel index {} is out of bounds ({} devices)",
        //     //             i,
        //     //             pcas.len()
        //     //         )
        //     //         .as_str(),
        //     //     );
        //     let address: Address = (*self
        //         .addresses
        //         .get(i / 16)
        //         .expect(
        //             format!(
        //                 "Channel index {} is out of bounds ({} devices)",
        //                 i,
        //                 self.addresses.len()
        //             )
        //             .as_str(),
        //         ))
        //         .try_into()
        //         .unwrap();
        //     println!("setting {:?} to {}", address, value);
        //     let mut pca = Pca9685::new(self.bus.acquire_i2c(), address).unwrap();
        //     pca.set_prescale(200).unwrap();
        //     pca.enable().unwrap();
        //     pca.set_channel_on_off((i % 16).try_into().unwrap(), 0, *value)
        //         .unwrap();
        // });
        Ok(())
    }
}

pub fn run() -> mcp2221::Result<()> {
    let mut mcp = MCPOutput::new(vec![0x40, 0x41]).unwrap();
    // mcp.add_devices(vec![0x50, 0x51, 0x52, 0x53]).unwrap();
    let mut t = 0;
    let mut tt = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    loop {
        // mcp.set(0, Channel::C0, i).unwrap();
        let values = (0..32).map(|i| ((((i + t) as f32 / 100.0).sin().powi(2)) * 4096.0) as u16).collect::<Vec<u16>>();
        // let values = (0..16).collect::<Vec<u16>>();
        // println!("{:?}", values);
        mcp.set(values);
        t = (t + 1) % 4096;
        let elapsed = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() - tt;
        let fps = 1_000_000_000.0 / elapsed as f64;
        if(t % 100 == 0) {
            println!("fps: {:.1}", fps);
        }
        tt = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        // thread::sleep(time::Duration::from_millis(1));
    }

    Ok(())
}
