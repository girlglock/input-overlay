use std::collections::HashSet;
use std::ffi::CString;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use hidapi::HidApi;
use tokio::sync::mpsc::UnboundedSender;

use crate::services::consts::hid_to_vk;
use crate::ws_server::InputEvent;

const READ_TIMEOUT_MS: i32 = 200;

//protocols
#[derive(Clone)]
enum Protocol {
    WootingV1,
    WootingV2,
    RazerV2,
    RazerV3,
    NuPhy,
    DrunkDeer,
    Keychron { layout: &'static [u16] },
    Madlions { layout: &'static [u16] },
    Bytech,
}

struct DeviceEntry {
    path: CString,
    proto: Protocol,
}

//api
pub struct AnalogThread {
    stop: Arc<AtomicBool>,
    handles: Vec<thread::JoinHandle<()>>,
}

impl AnalogThread {
    //filter is the analog_keyboard config value "auto" or a brandname.
    pub fn start(tx: UnboundedSender<InputEvent>, filter: &str) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let mut handles = Vec::new();

        match find_devices(filter) {
            Ok(devices) => {
                for entry in devices {
                    let tx2 = tx.clone();
                    let stop2 = Arc::clone(&stop);
                    let h = thread::Builder::new()
                        .name("AnalogHID".into())
                        .spawn(move || device_thread(tx2, entry, stop2))
                        .expect("failed to spawn analog thread");
                    handles.push(h);
                }
                if handles.is_empty() {
                    tracing::info!("analog: no compatible keyboards found (filter={filter:?})");
                }
            }
            Err(e) => tracing::warn!("analog: HID enumeration failed: {e}"),
        }

        AnalogThread { stop, handles }
    }
}

impl Drop for AnalogThread {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        for h in self.handles.drain(..) {
            let _ = h.join();
        }
    }
}

fn find_devices(filter: &str) -> Result<Vec<DeviceEntry>, hidapi::HidError> {
    let api = HidApi::new()?;
    let mut found = Vec::new();

    for dev in api.device_list() {
        let vid = dev.vendor_id();
        let pid = dev.product_id();
        let (up, u) = (dev.usage_page(), dev.usage());

        let Some((brand, proto)) = detect_protocol(vid, pid, up, u) else {
            continue;
        };

        if filter != "auto" && filter != brand {
            continue;
        }

        if let Ok(path) = CString::new(dev.path().to_bytes()) {
            tracing::info!("analog: found {brand} vid=0x{vid:04X} pid=0x{pid:04X}");
            found.push(DeviceEntry { path, proto });
        }
    }

    Ok(found)
}

fn detect_protocol(vid: u16, pid: u16, up: u16, usage: u16) -> Option<(&'static str, Protocol)> {
    //wooting
    if (vid == 0x31E3 || vid == 0x03EB) && up == 0xFF54 {
        return Some(("wooting", Protocol::WootingV1));
    }
    if vid == 0x31E3 && up == 0xFF53 {
        return Some(("wooting", Protocol::WootingV2));
    }

    //razer
    if vid == 0x1532 {
        match pid {
            0x0266 | 0x0282 => return Some(("razer", Protocol::RazerV2)),
            0x02A6 | 0x02A7 | 0x02B0 => return Some(("razer", Protocol::RazerV3)),
            _ => {}
        }
    }

    //NuPhy
    if vid == 0x19F5 && up == 1 && usage == 0 {
        return Some(("nuphy", Protocol::NuPhy));
    }

    //DrunkDeer
    if vid == 0x352D && up == 0xFF00 {
        return Some(("drunkdeer", Protocol::DrunkDeer));
    }

    //keychron, lemokey
    if (vid == 0x3434 || vid == 0x362D) && up == 0xFF60 && usage == 0x61 {
        let layout = keychron_layout(vid, pid)?;
        return Some(("keychron", Protocol::Keychron { layout }));
    }

    //madlions
    if vid == 0x373B && up == 0xFF60 && usage == 0x61 {
        let layout = madlions_layout(pid)?;
        return Some(("madlions", Protocol::Madlions { layout }));
    }

    //redragon
    if vid == 0x372E && pid == 0x105B && up == 0xFF00 {
        return Some(("bytech", Protocol::Bytech));
    }

    None
}

fn keychron_layout(vid: u16, pid: u16) -> Option<&'static [u16]> {
    match (vid, pid) {
        (0x3434, 0x0B10) | (0x3434, 0x0B11) | (0x3434, 0x0B12) => Some(KC_Q1_HE),
        (0x3434, 0x0B30) => Some(KC_Q3_HE),
        (0x3434, 0x0B50) => Some(KC_Q5_HE),
        (0x3434, 0x0E20) | (0x3434, 0x0E21) | (0x3434, 0x0E22) => Some(KC_K2_HE),
        (0x362D, 0x0610) => Some(KC_LMK_P1),
        _ => None,
    }
}

fn madlions_layout(pid: u16) -> Option<&'static [u16]> {
    match pid {
        0x1053 | 0x1055 | 0x1056 | 0x105D => Some(MAD60_LAYOUT),
        0x1058 | 0x1059 | 0x105A | 0x105C | 0x10A7 => Some(MAD68_LAYOUT),
        _ => None,
    }
}

fn device_thread(tx: UnboundedSender<InputEvent>, entry: DeviceEntry, stop: Arc<AtomicBool>) {
    let mut backoff = Duration::from_secs(1);

    while !stop.load(Ordering::Relaxed) {
        let api = match HidApi::new() {
            Ok(a) => a,
            Err(e) => {
                tracing::error!("analog: HidApi::new failed: {e}");
                thread::sleep(backoff);
                backoff = (backoff * 2).min(Duration::from_secs(16));
                continue;
            }
        };
        if let Ok(dev) = api.open_path(&entry.path) {
            backoff = Duration::from_secs(1);
            tracing::info!("analog: device connected");
            match &entry.proto {
                Protocol::WootingV1
                | Protocol::WootingV2
                | Protocol::RazerV2
                | Protocol::RazerV3
                | Protocol::NuPhy => run_passive(&dev, &tx, &entry.proto, &stop),
                Protocol::DrunkDeer => run_drunkdeer(&dev, &tx, &stop),
                Protocol::Keychron { layout } => run_keychron(&dev, &tx, layout, &stop),
                Protocol::Madlions { layout } => run_madlions(&dev, &tx, layout, &stop),
                Protocol::Bytech => run_bytech(&dev, &tx, &stop),
            }
            if !stop.load(Ordering::Relaxed) {
                tracing::info!("analog: device disconnected, retrying in {}s", backoff.as_secs());
            }
        }
        if !stop.load(Ordering::Relaxed) {
            thread::sleep(backoff);
            backoff = (backoff * 2).min(Duration::from_secs(16));
        }
    }

    tracing::info!("analog: device thread stopped");
}

fn run_passive(
    dev: &hidapi::HidDevice,
    tx: &UnboundedSender<InputEvent>,
    proto: &Protocol,
    stop: &AtomicBool,
) {
    let mut buf = [0u8; 512];
    let mut prev: HashSet<u16> = HashSet::new();
    while !stop.load(Ordering::Relaxed) {
        match dev.read_timeout(&mut buf, READ_TIMEOUT_MS) {
            Ok(0) => {}
            Ok(n) => {
                let current = match proto {
                    Protocol::WootingV1 => parse_wooting_v1(&buf[..n], tx),
                    Protocol::WootingV2 => parse_wooting_v2(&buf[..n], tx),
                    Protocol::RazerV2 => parse_razer(&buf[..n], tx, false),
                    Protocol::RazerV3 => parse_razer(&buf[..n], tx, true),
                    Protocol::NuPhy => {
                        parse_nuphy(&buf[..n], tx);
                        HashSet::new()
                    }
                    _ => unreachable!(),
                };
                for &vk in prev.difference(&current) {
                    let _ = tx.send(InputEvent::AnalogDepth {
                        rawcode: vk,
                        depth: 0.0,
                    });
                }
                prev = current;
            }
            Err(e) => {
                tracing::warn!("analog: read error: {e}");
                break;
            }
        }
    }
}

//3 byte entries: [scancode_hi, scancode_lo, value]. Stop when scancode == 0
fn parse_wooting_v1(report: &[u8], tx: &UnboundedSender<InputEvent>) -> HashSet<u16> {
    let mut active = HashSet::new();
    let mut i = 0;
    while i + 2 < report.len() {
        let scancode = u16::from_be_bytes([report[i], report[i + 1]]);
        if scancode == 0 {
            break;
        }
        let value = report[i + 2];
        i += 3;
        if let Some(vk) = hid_to_vk(scancode) {
            let _ = tx.send(InputEvent::AnalogDepth {
                rawcode: vk,
                depth: value as f32 / 255.0,
            });
            active.insert(vk);
        }
    }
    active
}

//4 byte entries: [matrix_pos, scancode_lo, packed, value_hi]. Stop when scancode_lo == 0
fn parse_wooting_v2(report: &[u8], tx: &UnboundedSender<InputEvent>) -> HashSet<u16> {
    let mut active = HashSet::new();
    let mut i = 0;
    while i + 3 < report.len() {
        let scancode_lo = report[i + 1];
        if scancode_lo == 0 {
            break;
        }
        let packed = report[i + 2];
        let value_hi = report[i + 3];
        i += 4;

        let scancode_hi = (packed >> 2) & 0x0F;
        let value_lo = (packed >> 6) & 0x03;
        let scancode = ((scancode_hi as u16) << 8) | (scancode_lo as u16);
        let value = ((value_hi as u16) << 2) | (value_lo as u16);
        if value == 0 {
            continue;
        }

        if let Some(vk) = hid_to_vk(scancode) {
            let _ = tx.send(InputEvent::AnalogDepth {
                rawcode: vk,
                depth: value as f32 / 1023.0,
            });
            active.insert(vk);
        }
    }
    active
}

//V2: 2-byte pair [razer_scan, value], stop at 0
//V3: 3-byte entries [razer_scan, value, skip], stop at 0
fn parse_razer(report: &[u8], tx: &UnboundedSender<InputEvent>, v3: bool) -> HashSet<u16> {
    let mut active = HashSet::new();
    let stride = if v3 { 3 } else { 2 };
    let mut i = 0;
    while i + stride <= report.len() {
        let scan = report[i];
        if scan == 0 {
            break;
        }
        let value = report[i + 1];
        i += stride;
        if let Some(hid) = razer_to_hid(scan) {
            if let Some(vk) = hid_to_vk(hid) {
                let _ = tx.send(InputEvent::AnalogDepth {
                    rawcode: vk,
                    depth: value as f32 / 255.0,
                });
                active.insert(vk);
            }
        }
    }
    active
}

//buffered: byte 0 == 0xA0, scancode at bytes 2-3 (LE u16), value at byte 7
fn parse_nuphy(report: &[u8], tx: &UnboundedSender<InputEvent>) {
    if report.len() < 8 || report[0] != 0xA0 {
        return;
    }
    let raw = u16::from_le_bytes([report[2], report[3]]);
    let value = report[7];
    let depth = value as f32 / 200.0;
    if let Some(hid) = nuphy_to_hid(raw) {
        emit_hid(tx, hid, depth.min(1.0));
    }
}

fn run_drunkdeer(dev: &hidapi::HidDevice, tx: &UnboundedSender<InputEvent>, stop: &AtomicBool) {
    let mut req = [0u8; 64];
    req[0] = 0x04;
    req[1] = 0xb6;
    req[2] = 0x03;
    req[3] = 0x01;

    let mut cur_vks: HashSet<u16> = HashSet::new();
    let mut prev_vks: HashSet<u16> = HashSet::new();
    let mut last_poll = Instant::now() - Duration::from_secs(1);

    while !stop.load(Ordering::Relaxed) {
        if last_poll.elapsed() >= Duration::from_millis(8) {
            if dev.write(&req).is_err() { break; }
            last_poll = Instant::now();
        }

        let mut buf = [0u8; 128];
        match dev.read_timeout(&mut buf, 5) {
            Ok(n) if n >= 5 => {
                //on bimbows hidapi prepends the report ID for non-zero IDs
                //detect: if buf[0] == 0x04, data starts at 1
                let off = if buf[0] == 0x04 { 1 } else { 0 };
                if n < off + 4 {
                    continue;
                }

                let n_pkt = buf[off + 3];
                if n_pkt == 0 {
                    cur_vks.clear();
                }

                for (pos, &value) in buf[(off + 4)..n].iter().enumerate() {
                    if value != 0 {
                        let idx = n_pkt as usize * 59 + pos;
                        if let Some(&hid) = DRUNKDEER.get(idx) {
                            if hid != 0 {
                                if let Some(vk) = hid_to_vk(hid) {
                                    let _ = tx.send(InputEvent::AnalogDepth {
                                        rawcode: vk,
                                        depth: (value as f32 / 40.0).min(1.0),
                                    });
                                    cur_vks.insert(vk);
                                }
                            }
                        }
                    }
                }

                if n_pkt == 2 {
                    for &vk in prev_vks.difference(&cur_vks) {
                        let _ = tx.send(InputEvent::AnalogDepth {
                            rawcode: vk,
                            depth: 0.0,
                        });
                    }
                    prev_vks.clone_from(&cur_vks);
                }
            }
            Ok(_) => {}
            Err(_) => break,
        }
        thread::sleep(Duration::from_millis(1));
    }
}

fn run_keychron(
    dev: &hidapi::HidDevice,
    tx: &UnboundedSender<InputEvent>,
    layout: &'static [u16],
    stop: &AtomicBool,
) {
    let mut req = [0u8; 33]; //report_id=0 + 32 byte
                             //wakeup
    req[1] = 0xa9;
    req[2] = 0x01;
    let _ = dev.write(&req);

    let mut buf = [0u8; 64];
    //wait a bit after wakeup
    let _ = dev.read_timeout(&mut buf, 500);

    //get keys
    req[2] = 0x31;
    let _ = dev.write(&req);

    let mut chunk = 0usize;
    let mut depths = vec![0.0f32; layout.len()];

    while !stop.load(Ordering::Relaxed) {
        match dev.read_timeout(&mut buf, READ_TIMEOUT_MS) {
            Ok(n) if n >= 32 => {
                //30 travel values at offsets 2 to31
                for i in 0..30usize {
                    let li = chunk * 30 + i;
                    if li >= layout.len() {
                        continue;
                    }
                    let hid = layout[li];
                    if hid == 0 {
                        continue;
                    }
                    let travel = buf[2 + i];
                    depths[li] = if travel >= 5 {
                        (travel as f32 / 235.0).min(1.0)
                    } else {
                        0.0
                    };
                }
                chunk += 1;
                if chunk == 4 {
                    chunk = 0;
                    for (li, &depth) in depths.iter().enumerate() {
                        let hid = layout[li];
                        if hid != 0 {
                            if let Some(vk) = hid_to_vk(hid) {
                                let _ = tx.send(InputEvent::AnalogDepth { rawcode: vk, depth });
                            }
                        }
                    }
                    //next
                    if dev.write(&req).is_err() { break; }
                }
            }
            Ok(0) => {}
            Ok(_) => {}
            Err(_) => break,
        }
    }
}

fn run_madlions(
    dev: &hidapi::HidDevice,
    tx: &UnboundedSender<InputEvent>,
    layout: &'static [u16],
    stop: &AtomicBool,
) {
    let mut req = [0u8; 33];
    req[1] = 0x02;
    req[2] = 0x96;
    req[3] = 0x1C;
    req[8] = 0x04; //only 4 keys per response

    let mut offset = 0usize;
    let _ = dev.write(&req);

    let mut buf = [0u8; 64];

    while !stop.load(Ordering::Relaxed) {
        match dev.read_timeout(&mut buf, READ_TIMEOUT_MS) {
            Ok(n) if n >= 28 => {
                for i in 0..4usize {
                    let li = offset + i;
                    if li >= layout.len() {
                        continue;
                    }
                    let hid = layout[li];
                    if hid == 0 {
                        continue;
                    }
                    let byte_off = 7 + i * 5 + 3;
                    if byte_off + 1 >= n {
                        continue;
                    }
                    let travel = u16::from_be_bytes([buf[byte_off], buf[byte_off + 1]]);
                    if let Some(vk) = hid_to_vk(hid) {
                        let depth = if travel > 0 {
                            (travel as f32 / 350.0).min(1.0)
                        } else {
                            0.0
                        };
                        let _ = tx.send(InputEvent::AnalogDepth { rawcode: vk, depth });
                    }
                }
                offset += 4;
                if offset >= layout.len() {
                    offset = 0;
                }
                req[7] = offset as u8;
                if dev.write(&req).is_err() { break; }
            }
            Ok(0) => {}
            Ok(_) => {}
            Err(_) => break,
        }
    }
}

fn bytech_request() -> [u8; 64] {
    //report_id=9, cmd=0x97, sub=0x00, checksum in last byte
    let mut buf = [0u8; 64];
    buf[0] = 9;
    buf[1] = 0x97;
    let sum: u32 = 9 + buf[1..63].iter().map(|&b| b as u32).sum::<u32>();
    buf[63] = (255 - (sum % 256)) as u8;
    buf
}

fn run_bytech(dev: &hidapi::HidDevice, tx: &UnboundedSender<InputEvent>, stop: &AtomicBool) {
    let req = bytech_request();
    let mut last_poll = Instant::now() - Duration::from_secs(1);
    let mut buf = [0u8; 128];
    let mut prev_vks: HashSet<u16> = HashSet::new();

    while !stop.load(Ordering::Relaxed) {
        if last_poll.elapsed() >= Duration::from_millis(8) {
            if dev.write(&req).is_err() { break; }
            last_poll = Instant::now();
        }

        match dev.read_timeout(&mut buf, 5) {
            Ok(n) if n >= 7 => {
                //strip report ID if present
                //non-zero report ID then first byte is ID=9
                let off = if buf[0] == 9 { 1 } else { 0 };
                if n < off + 7 {
                    continue;
                }

                if buf[off] != 0x97 || buf[off + 1] != 0x01 {
                    continue;
                }

                let mut cur_vks: HashSet<u16> = HashSet::new();
                let count = buf[off + 5] as usize;
                let mut i = 0;
                while i < count && off + 6 + i * 4 + 3 < n {
                    let base = off + 6 + i * 4;
                    let pos = u16::from_be_bytes([buf[base], buf[base + 1]]);
                    let dist = u16::from_be_bytes([buf[base + 2], buf[base + 3]]);
                    i += 1;
                    if dist <= 10 {
                        continue;
                    }
                    if let Some(hid) = bytech_to_hid(pos) {
                        if let Some(vk) = hid_to_vk(hid) {
                            let _ = tx.send(InputEvent::AnalogDepth {
                                rawcode: vk,
                                depth: (dist as f32 / 355.0).min(1.0),
                            });
                            cur_vks.insert(vk);
                        }
                    }
                }
                for &vk in prev_vks.difference(&cur_vks) {
                    let _ = tx.send(InputEvent::AnalogDepth {
                        rawcode: vk,
                        depth: 0.0,
                    });
                }
                prev_vks = cur_vks;
            }
            Ok(_) => {}
            Err(_) => break,
        }
        thread::sleep(Duration::from_millis(1));
    }
}

//elpers
#[inline]
fn emit_hid(tx: &UnboundedSender<InputEvent>, hid: u16, depth: f32) {
    if let Some(vk) = hid_to_vk(hid) {
        let _ = tx.send(InputEvent::AnalogDepth { rawcode: vk, depth });
    }
}

fn razer_to_hid(s: u8) -> Option<u16> {
    Some(match s {
        0x6E => 0x29,
        0x70 => 0x3A,
        0x71 => 0x3B,
        0x72 => 0x3C,
        0x73 => 0x3D,
        0x74 => 0x3E,
        0x75 => 0x3F,
        0x76 => 0x40,
        0x77 => 0x41,
        0x78 => 0x42,
        0x79 => 0x43,
        0x7A => 0x44,
        0x7B => 0x45,
        0x01 => 0x35,
        0x02 => 0x1E,
        0x03 => 0x1F,
        0x04 => 0x20,
        0x05 => 0x21,
        0x06 => 0x22,
        0x07 => 0x23,
        0x08 => 0x24,
        0x09 => 0x25,
        0x0A => 0x26,
        0x0B => 0x27,
        0x0C => 0x2D,
        0x0D => 0x2E,
        0x0F => 0x2A,
        0x10 => 0x2B,
        0x11 => 0x14,
        0x12 => 0x1A,
        0x13 => 0x08,
        0x14 => 0x15,
        0x15 => 0x17,
        0x16 => 0x1C,
        0x17 => 0x18,
        0x18 => 0x0C,
        0x19 => 0x12,
        0x1A => 0x13,
        0x1B => 0x2F,
        0x1C => 0x30,
        0x2B => 0x28,
        0x1E => 0x39,
        0x1F => 0x04,
        0x20 => 0x16,
        0x21 => 0x07,
        0x22 => 0x09,
        0x23 => 0x0A,
        0x24 => 0x0B,
        0x25 => 0x0D,
        0x26 => 0x0E,
        0x27 => 0x0F,
        0x28 => 0x33,
        0x29 => 0x34,
        0x2A => 0x31,
        0x2C => 0xE1,
        0x2D => 0x64,
        0x2E => 0x1D,
        0x2F => 0x1B,
        0x30 => 0x06,
        0x31 => 0x19,
        0x32 => 0x05,
        0x33 => 0x11,
        0x34 => 0x10,
        0x35 => 0x36,
        0x36 => 0x37,
        0x37 => 0x38,
        0x39 => 0xE5,
        0x3A => 0xE0,
        0x3B => 0x409,
        0x3C => 0xE2,
        0x3D => 0x2C,
        0x3E => 0xE6,
        0x40 => 0xE4,
        0x7C => 0x46,
        0x7D => 0x48,
        0x7E => 0x47,
        0x4B => 0x49,
        0x50 => 0x4A,
        0x55 => 0x4B,
        0x4C => 0x4C,
        0x51 => 0x4D,
        0x56 => 0x4E,
        0x53 => 0x52,
        0x4F => 0x50,
        0x54 => 0x51,
        0x59 => 0x4F,
        0x5A => 0x53,
        0x5F => 0x54,
        0x64 => 0x55,
        0x69 => 0x56,
        0x5B => 0x5F,
        0x60 => 0x60,
        0x65 => 0x61,
        0x6A => 0x57,
        0x5C => 0x5C,
        0x61 => 0x5D,
        0x66 => 0x5E,
        0x5D => 0x59,
        0x62 => 0x5A,
        0x67 => 0x5B,
        0x6C => 0x58,
        0x63 => 0x62,
        0x68 => 0x63,
        0x7F => 0xE3,
        0x81 => 0x65,
        _ => return None,
    })
}

fn nuphy_to_hid(s: u16) -> Option<u16> {
    match s {
        0x0200 => Some(0xE1),
        0x2000 => Some(0xE5),
        0x0100 => Some(0xE0),
        0x0800 => Some(0xE3),
        0x0400 => Some(0xE2),
        0x4000 => Some(0xE6),
        0x8000 => Some(0xE7),
        0xFF05 => Some(0x409),
        0x1000 => Some(0xE4),
        0 => None,
        s if s < 0x100 => Some(s),
        _ => None,
    }
}

fn bytech_to_hid(s: u16) -> Option<u16> {
    Some(match s {
        1 => 0x29,
        2 => 0x3A,
        3 => 0x3B,
        4 => 0x3C,
        5 => 0x3D,
        6 => 0x3E,
        7 => 0x3F,
        8 => 0x40,
        9 => 0x41,
        10 => 0x42,
        11 => 0x43,
        12 => 0x44,
        13 => 0x45,
        14 => 0x35,
        15 => 0x1E,
        16 => 0x1F,
        17 => 0x20,
        18 => 0x21,
        19 => 0x22,
        20 => 0x23,
        21 => 0x24,
        22 => 0x25,
        23 => 0x26,
        24 => 0x27,
        25 => 0x2D,
        26 => 0x2E,
        27 => 0x2A,
        28 => 0x2B,
        29 => 0x14,
        30 => 0x1A,
        31 => 0x08,
        32 => 0x15,
        33 => 0x17,
        34 => 0x1C,
        35 => 0x18,
        36 => 0x0C,
        37 => 0x12,
        38 => 0x13,
        39 => 0x2F,
        40 => 0x30,
        41 => 0x31,
        42 => 0x39,
        43 => 0x04,
        44 => 0x16,
        45 => 0x07,
        46 => 0x09,
        47 => 0x0A,
        48 => 0x0B,
        49 => 0x0D,
        50 => 0x0E,
        51 => 0x0F,
        52 => 0x33,
        53 => 0x34,
        54 => 0x28,
        55 => 0xE1,
        56 => 0x1D,
        57 => 0x1B,
        58 => 0x06,
        59 => 0x19,
        60 => 0x05,
        61 => 0x11,
        62 => 0x10,
        63 => 0x36,
        64 => 0x37,
        65 => 0x38,
        66 => 0xE5,
        67 => 0xE0,
        68 => 0xE3,
        69 => 0xE2,
        70 => 0x2C,
        71 => 0xE6,
        72 => 0x409,
        73 => 0xE4,
        74 => 0x52,
        75 => 0x51,
        76 => 0x50,
        77 => 0x4F,
        99 => 0x4C,
        100 => 0x4A,
        102 => 0x4B,
        103 => 0x4E,
        _ => return None,
    })
}

//6 rows × 21 slots = 126 entries; 0x000 = no key
static DRUNKDEER: &[u16] = &[
    //row 0 (idx 0-20)
    0x029, 0x000, 0x03A, 0x03B, 0x03C, 0x03D, 0x03E, 0x03F, 0x040, 0x041, 0x042, 0x043, 0x044,
    0x045, 0x04C, 0x000, 0x000, 0x000, 0x000, 0x000, 0x000, //row 1 (idx 21-41)
    0x035, 0x01E, 0x01F, 0x020, 0x021, 0x022, 0x023, 0x024, 0x025, 0x026, 0x027, 0x02D, 0x02E,
    0x02A, 0x000, 0x04A, 0x000, 0x000, 0x000, 0x000, 0x000, //row 2 (idx 42-62)
    0x02B, 0x014, 0x01A, 0x008, 0x015, 0x017, 0x01C, 0x018, 0x00C, 0x012, 0x013, 0x02F, 0x030,
    0x031, 0x000, 0x04B, 0x000, 0x000, 0x000, 0x000, 0x000, //row 3 (idx 63-83)
    0x039, 0x004, 0x016, 0x007, 0x009, 0x00A, 0x00B, 0x00D, 0x00E, 0x00F, 0x033, 0x034, 0x000,
    0x028, 0x000, 0x04E, 0x000, 0x000, 0x000, 0x000, 0x000, //row 4 (idx 84-104)
    0x0E1, 0x000, 0x01D, 0x01B, 0x006, 0x019, 0x005, 0x011, 0x010, 0x036, 0x037, 0x038, 0x000,
    0x0E5, 0x052, 0x04D, 0x000, 0x000, 0x000, 0x000, 0x000, //row 5 (idx 105-125)
    0x0E0, 0x0E3, 0x0E2, 0x000, 0x000, 0x000, 0x02C, 0x000, 0x000, 0x000, 0x0E6, 0x409, 0x065,
    0x000, 0x050, 0x051, 0x04F, 0x000, 0x000, 0x000, 0x000,
];

static KC_Q1_HE: &[u16] = &[
    0x029, 0x03A, 0x03B, 0x03C, 0x03D, 0x03E, 0x03F, 0x040, 0x041, 0x042, 0x043, 0x044, 0x045,
    0x04C, 0x000, 0x035, 0x01E, 0x01F, 0x020, 0x021, 0x022, 0x023, 0x024, 0x025, 0x026, 0x027,
    0x02D, 0x02E, 0x02A, 0x04B, 0x02B, 0x014, 0x01A, 0x008, 0x015, 0x017, 0x01C, 0x018, 0x00C,
    0x012, 0x013, 0x02F, 0x030, 0x031, 0x04E, 0x039, 0x004, 0x016, 0x007, 0x009, 0x00A, 0x00B,
    0x00D, 0x00E, 0x00F, 0x033, 0x034, 0x028, 0x04A, 0x000, 0x0E1, 0x000, 0x01D, 0x01B, 0x006,
    0x019, 0x005, 0x011, 0x010, 0x036, 0x037, 0x000, 0x038, 0x0E5, 0x052, 0x0E0, 0x0E3, 0x0E2,
    0x000, 0x000, 0x000, 0x02C, 0x000, 0x000, 0x0E7, 0x409, 0x0E4, 0x050, 0x051, 0x04F,
];

static KC_Q3_HE: &[u16] = &[
    0x029, 0x03A, 0x03B, 0x03C, 0x03D, 0x03E, 0x03F, 0x040, 0x041, 0x042, 0x043, 0x044, 0x045,
    0x046, 0x403, 0x404, 0x035, 0x01E, 0x01F, 0x020, 0x021, 0x022, 0x023, 0x024, 0x025, 0x026,
    0x027, 0x02D, 0x02E, 0x02A, 0x049, 0x04A, 0x02B, 0x014, 0x01A, 0x008, 0x015, 0x017, 0x01C,
    0x018, 0x00C, 0x012, 0x013, 0x02F, 0x030, 0x031, 0x04C, 0x04D, 0x039, 0x004, 0x016, 0x007,
    0x009, 0x00A, 0x00B, 0x00D, 0x00E, 0x00F, 0x033, 0x034, 0x000, 0x028, 0x04B, 0x04E, 0x0E1,
    0x000, 0x01D, 0x01B, 0x006, 0x019, 0x005, 0x011, 0x010, 0x036, 0x037, 0x000, 0x038, 0x0E5,
    0x000, 0x052, 0x0E0, 0x0E3, 0x0E2, 0x000, 0x000, 0x000, 0x02C, 0x000, 0x000, 0x0E6, 0x0E7,
    0x409, 0x0E4, 0x050, 0x051, 0x04F,
];

static KC_Q5_HE: &[u16] = &[
    0x029, 0x000, 0x03A, 0x03B, 0x03C, 0x03D, 0x03E, 0x03F, 0x040, 0x041, 0x042, 0x043, 0x044,
    0x045, 0x04C, 0x403, 0x404, 0x405, 0x000, 0x035, 0x01E, 0x01F, 0x020, 0x021, 0x022, 0x023,
    0x024, 0x025, 0x026, 0x027, 0x02D, 0x02E, 0x02A, 0x04B, 0x053, 0x054, 0x055, 0x056, 0x02B,
    0x014, 0x01A, 0x008, 0x015, 0x017, 0x01C, 0x018, 0x00C, 0x012, 0x013, 0x02F, 0x030, 0x031,
    0x04E, 0x05F, 0x060, 0x061, 0x057, 0x039, 0x004, 0x016, 0x007, 0x009, 0x00A, 0x00B, 0x00D,
    0x00E, 0x00F, 0x033, 0x034, 0x028, 0x04A, 0x000, 0x05C, 0x05D, 0x05E, 0x000, 0x0E1, 0x000,
    0x01D, 0x01B, 0x006, 0x019, 0x005, 0x011, 0x010, 0x036, 0x037, 0x000, 0x038, 0x0E5, 0x052,
    0x059, 0x05A, 0x05B, 0x058, 0x0E0, 0x0E3, 0x0E2, 0x000, 0x000, 0x000, 0x02C, 0x000, 0x000,
    0x0E7, 0x409, 0x0E4, 0x050, 0x051, 0x04F, 0x000, 0x062, 0x063, 0x000,
];

static KC_K2_HE: &[u16] = &[
    0x029, 0x03A, 0x03B, 0x03C, 0x03D, 0x03E, 0x03F, 0x040, 0x041, 0x042, 0x043, 0x044, 0x045,
    0x046, 0x04C, 0x404, 0x035, 0x01E, 0x01F, 0x020, 0x021, 0x022, 0x023, 0x024, 0x025, 0x026,
    0x027, 0x02D, 0x02E, 0x02A, 0x04B, 0x000, 0x02B, 0x014, 0x01A, 0x008, 0x015, 0x017, 0x01C,
    0x018, 0x00C, 0x012, 0x013, 0x02F, 0x030, 0x031, 0x04E, 0x000, 0x039, 0x004, 0x016, 0x007,
    0x009, 0x00A, 0x00B, 0x00D, 0x00E, 0x00F, 0x033, 0x034, 0x028, 0x04A, 0x000, 0x000, 0x0E1,
    0x000, 0x01D, 0x01B, 0x006, 0x019, 0x005, 0x011, 0x010, 0x036, 0x037, 0x038, 0x0E5, 0x052,
    0x04D, 0x000, 0x0E0, 0x0E3, 0x0E2, 0x000, 0x000, 0x000, 0x02C, 0x000, 0x000, 0x0E6, 0x409,
    0x0E4, 0x050, 0x051, 0x04F, 0x000,
];

static KC_LMK_P1: &[u16] = &[
    0x029, 0x03A, 0x03B, 0x03C, 0x03D, 0x03E, 0x03F, 0x040, 0x041, 0x042, 0x043, 0x044, 0x045,
    0x04C, 0x000, 0x035, 0x01E, 0x01F, 0x020, 0x021, 0x022, 0x023, 0x024, 0x025, 0x026, 0x027,
    0x02D, 0x02E, 0x02A, 0x04A, 0x02B, 0x014, 0x01A, 0x008, 0x015, 0x017, 0x01C, 0x018, 0x00C,
    0x012, 0x013, 0x02F, 0x030, 0x031, 0x04B, 0x039, 0x004, 0x016, 0x007, 0x009, 0x00A, 0x00B,
    0x00D, 0x00E, 0x00F, 0x033, 0x034, 0x028, 0x04E, 0x000, 0x0E1, 0x000, 0x01D, 0x01B, 0x006,
    0x019, 0x005, 0x011, 0x010, 0x036, 0x037, 0x000, 0x038, 0x0E5, 0x052, 0x0E0, 0x0E3, 0x0E2,
    0x000, 0x000, 0x000, 0x02C, 0x000, 0x000, 0x0E7, 0x409, 0x0E4, 0x050, 0x051, 0x04F,
];

static MAD60_LAYOUT: &[u16] = &[
    0x029, 0x01E, 0x01F, 0x020, 0x021, 0x022, 0x023, 0x024, 0x025, 0x026, 0x027, 0x02D, 0x02E,
    0x02A, 0x02B, 0x014, 0x01A, 0x008, 0x015, 0x017, 0x01C, 0x018, 0x00C, 0x012, 0x013, 0x02F,
    0x030, 0x031, 0x039, 0x004, 0x016, 0x007, 0x009, 0x00A, 0x00B, 0x00D, 0x00E, 0x00F, 0x033,
    0x034, 0x000, 0x028, 0x0E1, 0x000, 0x01D, 0x01B, 0x006, 0x019, 0x005, 0x011, 0x010, 0x036,
    0x037, 0x038, 0x000, 0x0E5, 0x0E0, 0x0E3, 0x0E2, 0x000, 0x000, 0x000, 0x02C, 0x000, 0x000,
    0x0E7, 0x0E6, 0x065, 0x0E4, 0x409,
];

static MAD68_LAYOUT: &[u16] = &[
    0x029, 0x01E, 0x01F, 0x020, 0x021, 0x022, 0x023, 0x024, 0x025, 0x026, 0x027, 0x02D, 0x02E,
    0x02A, 0x049, 0x02B, 0x014, 0x01A, 0x008, 0x015, 0x017, 0x01C, 0x018, 0x00C, 0x012, 0x013,
    0x02F, 0x030, 0x031, 0x04C, 0x039, 0x004, 0x016, 0x007, 0x009, 0x00A, 0x00B, 0x00D, 0x00E,
    0x00F, 0x033, 0x034, 0x000, 0x028, 0x04B, 0x0E1, 0x000, 0x01D, 0x01B, 0x006, 0x019, 0x005,
    0x011, 0x010, 0x036, 0x037, 0x038, 0x0E5, 0x052, 0x04E, 0x0E0, 0x0E3, 0x0E2, 0x000, 0x000,
    0x000, 0x02C, 0x000, 0x000, 0x0E6, 0x409, 0x0E4, 0x050, 0x051, 0x04F,
];
