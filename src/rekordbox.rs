use byteorder::*;
use process_list::for_each_module;
use read_process_memory::*;
use std::convert::TryInto;
use sysinfo::{PidExt, ProcessExt, SystemExt};

const TRACK_1_OFFSET: [u32; 6] = [0x03FB2B08, 0x0, 0x240, 0x78, 0x108, 0x148];
const TRACK_2_OFFSET: [u32; 4] = [0x03FB2B08, 0x8, 0x230, 0x148];

const TRACK_1_TITLE: [u32; 5] = [0x03FA6B10, 0x780, 0x170, 0x0, 0x0];
const TRACK_2_TITLE: [u32; 3] = [0x03F4D188, 0x318, 0x0];

const CROSSFADER: [u32; 6] = [0x03FA6B10, 0x200, 0x40, 0x30, 0xE0, 0xB4];

// // #[inline]
// fn vec_to_arr<T, const N: usize>(v: Vec<T>) -> [T; N] {
//     v.try_into()
//         .unwrap_or_else(|v: Vec<T>| panic!("Expected a Vec of length {} but it was {}", N, v.len()))
// }

#[inline]
fn le_double(bytes: Vec<u8>) -> f64 {
    return byteorder::LittleEndian::read_f64(bytes.as_slice());
}

#[inline]
fn le_float(bytes: Vec<u8>) -> f32 {
    return byteorder::LittleEndian::read_f32(bytes.as_slice());
}

fn modules_by_name(pid: sysinfo::Pid) -> Option<Vec<(String, usize)>> {
    let mut modules = Vec::new();
    for_each_module(pid.as_u32(), |(module_base, _size), name| {
        let name = name
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .trim_matches(char::from(0));
        modules.push((String::from(name), module_base));
    }).ok()?;
    return Some(modules);
}

fn open_module(name: &str, module_name: &str) -> Option<ModuleHandle> {
    let mut system = sysinfo::System::new_all();
    system.refresh_all();

    let mut processes = system.processes_by_exact_name(name);
    let proc = processes.next()?;

    let pid = proc.pid();
    // println!("found process {:?}", pid);
    let modules = modules_by_name(pid)?;
    let module = modules.into_iter().find(|m| m.0.eq(module_name))?;

    // println!("found module at {:#x?}", module.1);
    let handle = pid.as_u32().try_into().ok()?;
    return Some(ModuleHandle {
        process_handle: handle,
        module_base: module.1,
    });
}

// fn open_module_2(name: &str, module_name: &str) -> Option<(ProcessHandle, usize)> {
//     let process = Process::from_name("rekordbox.exe").ok()?;
//     let module = process.module("rekordbox.exe").ok()?;
//     return (process.handle(), module.handle() as usize);
// }

struct ModuleHandle {
    process_handle: ProcessHandle,
    module_base: usize,
}

impl ModuleHandle {}

pub struct RekordboxUpdate {
    pub track_1_title: String,
    pub track_2_title: String,
    pub track_1_offset: f64,
    pub track_2_offset: f64,
    pub crossfader: f32,
}

pub struct RekordboxAccess {
    handle: Option<ModuleHandle>,
    track_1_title_address: CachedPointerChain,
    track_2_title_address: CachedPointerChain,
    track_1_offset_address: CachedPointerChain,
    track_2_offset_address: CachedPointerChain,
    crossfader_address: CachedPointerChain,
}

impl RekordboxAccess {
    pub fn make() -> RekordboxAccess {
        let rekordbox_access = RekordboxAccess {
            handle: None,
            track_1_title_address: CachedPointerChain::make(TRACK_1_TITLE.to_vec()),
            track_2_title_address: CachedPointerChain::make(TRACK_2_TITLE.to_vec()),
            track_1_offset_address: CachedPointerChain::make(TRACK_1_OFFSET.to_vec()),
            track_2_offset_address: CachedPointerChain::make(TRACK_2_OFFSET.to_vec()),
            crossfader_address: CachedPointerChain::make(CROSSFADER.to_vec()),
        };
        return rekordbox_access;
    }

    pub fn attach(&mut self) -> Result<(), &'static str> {
        self.handle = open_module("rekordbox.exe", "rekordbox.exe");
        if self.handle.is_some() {
            println!("attached to rekordbox");
            return Ok(());
        } else {
            return Err("could not attach to rekordbox");
        }
    }

    pub fn is_attached(&self) -> bool {
        return self.handle.is_some();
    }

    fn read_values(&mut self) -> Option<RekordboxUpdate> {
        let ref mut handle= self.handle.as_ref()?;
        let track_1_offset = self.track_1_offset_address.get_double(&handle, true)?;
        let track_2_offset = self.track_2_offset_address.get_double(&handle, true)?;

        let track_1_title = self.track_1_title_address.get_string(&handle, false)?;
        let track_2_title = self.track_2_title_address.get_string(&handle, false)?;

        // let crossfader = self
        //     .crossfader_address
        //     .get_bytes(&handle, 4, true)
        //     .and_then(|bytes| Some((le_float(bytes) + 2.5625) / 5.125))?;
        let crossfader = 0.5;
        // println!("read values");
        return Some(RekordboxUpdate {
            track_1_title,
            track_2_title,
            track_1_offset,
            track_2_offset,
            crossfader,
        });
    }

    pub fn get_update(&mut self) -> Option<RekordboxUpdate>{
        return self.read_values().or_else(|| {
            self.handle = None;
            self.attach();
            return None;
        });
    }
}

struct CachedPointerChain {
    chain: Vec<u32>,
    cached_addr: Option<usize>,
}

impl CachedPointerChain {
    fn make(chain: Vec<u32>) -> CachedPointerChain {
        return CachedPointerChain {
            chain: chain,
            cached_addr: None,
        };
    }

    fn follow_chain(&mut self, handle: &ModuleHandle) -> Option<usize> {
        let mut pos: usize = handle.module_base + self.chain[0] as usize;
        for offset in self.chain[1..].iter() {
            let bytes = copy_address(pos, 8, &handle.process_handle).ok()?;
            let pointer = byteorder::LittleEndian::read_i64(bytes.as_slice());
            pos = (pointer as usize) + (*offset as usize);
        }
        self.cached_addr = Some(pos);
        return Some(pos as usize);
    }
    fn get_bytes(
        &mut self,
        handle: &ModuleHandle,
        num_bytes: usize,
        try_without_cache: bool,
    ) -> Option<Vec<u8>> {
        if try_without_cache && self.cached_addr.is_some() {
            let result = self
                .cached_addr
                .and_then(|addr| copy_address(addr, num_bytes, &handle.process_handle).ok());
            if result.is_some() {
                return result;
            }
        }
        self.follow_chain(handle);

        let result = self
            .cached_addr
            .and_then(|addr| copy_address(addr, num_bytes, &handle.process_handle).ok());
        return result;
    }

    fn get_string(&mut self, handle: &ModuleHandle, try_without_cache: bool) -> Option<String> {
        let bytes = self.get_bytes(handle, 128, try_without_cache)?;
        let zero_index = bytes.iter().position(|x| *x == 0)?;
        return String::from_utf8(bytes[..zero_index].to_vec()).ok();
    }

    fn get_double(&mut self, handle: &ModuleHandle, try_without_cache: bool) -> Option<f64> {
        return self
            .get_bytes(handle, 8, try_without_cache)
            .and_then(|bytes| Some(le_double(bytes)));
    }
}
