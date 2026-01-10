use ctap_hid_fido2::{Cfg, FidoKeyHidFactory};
use hidapi::{HidApi, HidDevice};
use serde_cbor_2::{Value, from_slice, to_vec};
use rand::{rng, RngCore as RandRngCore};
use tauri::{State};
use std::sync::Mutex;
use std::collections::BTreeMap;
use crate::types::{FidoDeviceInfo, PicoCredential, PicoMemoryStats};
use std::collections::HashMap;

pub struct PicoVendorClient {
    device: HidDevice,
    cid: [u8; 4],
}

impl PicoVendorClient {
    pub fn discover() -> Result<Self, String> {
        let api = HidApi::new().map_err(|e| e.to_string())?;
        
        log::info!("Starting FIDO device discovery...");
        
        // Enumerate all HID devices
        for device_info in api.device_list() {
            let vid = device_info.vendor_id();
            let pid = device_info.product_id();
            
            log::debug!("Checking device VID:{:04X} PID:{:04X}", vid, pid);
            
            // Try to connect and check if it's a FIDO device
            match Self::new(vid, pid) {
                Ok(client) => {
                    // Try to get FIDO info - if it responds, it's our device
                    match client.get_info_manual() {
                        Ok(info) => {
                            log::info!("Found FIDO device at VID:{:04X} PID:{:04X} - AAGUID: {}", 
                                vid, pid, info.aaguid);
                            return Ok(client);
                        }
                        Err(e) => {
                            log::debug!("Device VID:{:04X} PID:{:04X} is not a FIDO device: {}", 
                                vid, pid, e);
                            continue;
                        }
                    }
                }
                Err(e) => {
                    log::debug!("Could not open device VID:{:04X} PID:{:04X}: {}", vid, pid, e);
                    continue;
                }
            }
        }
        
        Err("No FIDO device found during discovery".into())
    }

    pub fn new(vid: u16, pid: u16) -> Result<Self, String> {
        let api = HidApi::new().map_err(|e| e.to_string())?;
        let device = api.open(vid, pid).map_err(|e| e.to_string())?;

        let mut nonce = [0u8; 8];
        RandRngCore::fill_bytes(&mut rng(), &mut nonce);

        let mut init_packet = [0u8; 65];
        let offset = 1; 
        init_packet[offset..offset + 4].copy_from_slice(&[0xff, 0xff, 0xff, 0xff]);
        init_packet[offset + 4] = 0x86; 
        init_packet[offset + 5] = 0x00;
        init_packet[offset + 6] = 0x08;
        init_packet[offset + 7..offset + 15].copy_from_slice(&nonce);

        log::debug!("Sending INIT packet: {}", hex::encode(&init_packet[1..16]));
        device.write(&init_packet).map_err(|e| e.to_string())?;

        let mut res = [0u8; 64];
        device.read_timeout(&mut res, 1000).map_err(|e| e.to_string())?;
        
        log::debug!("INIT response: {}", hex::encode(&res));

        if &res[7..15] != &nonce {
            return Err(format!("Nonce mismatch. Sent: {}, Got: {}", 
                hex::encode(&nonce), hex::encode(&res[7..15])));
        }

        let mut cid = [0u8; 4];
        cid.copy_from_slice(&res[15..19]);
        
        log::info!("CTAPHID initialized with CID: {}", hex::encode(&cid));

        Ok(PicoVendorClient { device, cid })
    }

    fn write_packet(&self, cmd: u8, data: &[u8]) -> Result<(), String> {
        let mut frame = vec![0u8; 65];
        let offset = 1;
        
        frame[offset..offset+4].copy_from_slice(&self.cid);
        frame[offset+4] = 0x80 | cmd;
        frame[offset+5] = (data.len() >> 8) as u8;
        frame[offset+6] = (data.len() & 0xFF) as u8;
        
        let data_len = std::cmp::min(data.len(), 57);
        frame[offset+7..offset+7+data_len].copy_from_slice(&data[..data_len]);

        log::debug!("Sending HID packet - CMD: 0x{:02X}, Data len: {}, Packet: {}", 
            cmd, data.len(), hex::encode(&frame[offset..offset+7+data_len]));

        self.device.write(&frame).map_err(|e| e.to_string())?;
        
        if data.len() > 57 {
            let mut remaining = &data[57..];
            let mut seq = 0u8;
            while !remaining.is_empty() {
                let mut cont_frame = vec![0u8; 65];
                cont_frame[offset..offset+4].copy_from_slice(&self.cid);
                cont_frame[offset+4] = seq & 0x7F;
                
                let cont_len = std::cmp::min(remaining.len(), 59);
                cont_frame[offset+5..offset+5+cont_len].copy_from_slice(&remaining[..cont_len]);
                
                self.device.write(&cont_frame).map_err(|e| e.to_string())?;
                remaining = &remaining[cont_len..];
                seq += 1;
            }
        }
        Ok(())
    }

    fn read_response(&self) -> Result<Vec<u8>, String> {
        loop {
            let mut res = [0u8; 64];
            self.device.read_timeout(&mut res, 2000).map_err(|e| e.to_string())?;
            
            log::debug!("Raw HID response: {}", hex::encode(&res[0..16]));
            
            let cmd = res[4] & 0x7F;
            
            if cmd == 0x3B {
                log::debug!("Received KEEPALIVE, waiting for actual response...");
                continue;
            }
            
            if cmd == 0x3F {
                let error_code = res[7];
                return Err(format!("CTAPHID error: 0x{:02X}", error_code));
            }
            
            let total_len = ((res[5] as usize) << 8) | (res[6] as usize);
            
            log::debug!("Response CMD: 0x{:02X}, Length: {}", cmd, total_len);
            
            let mut data = Vec::with_capacity(total_len);
            
            let first_chunk = std::cmp::min(total_len, 57);
            data.extend_from_slice(&res[7..7+first_chunk]);

            while data.len() < total_len {
                self.device.read_timeout(&mut res, 2000).map_err(|e| e.to_string())?;
                let cont_chunk = std::cmp::min(total_len - data.len(), 59);
                data.extend_from_slice(&res[5..5+cont_chunk]);
            }
            
            return Ok(data);
        }
    }

    pub fn get_info_manual(&self) -> Result<FidoDeviceInfo, String> {
        let ctap_cmd = vec![0x04];
        
        self.write_packet(0x10, &ctap_cmd)?;
        let response = self.read_response()?;

        log::debug!("FIDO GetInfo response length: {} bytes", response.len());
        log::debug!("FIDO GetInfo response hex: {}", hex::encode(&response));

        if response.is_empty() {
            return Err("Empty response from device".into());
        }

        if response[0] != 0x00 {
            return Err(format!("CTAP error: 0x{:02X}", response[0]));
        }

        let info_val: Value = from_slice(&response[1..]).map_err(|e| {
            log::error!("CBOR parse error: {}. Response was: {}", e, hex::encode(&response));
            e.to_string()
        })?;

        if let Value::Map(m) = info_val {
            let mut options_map = HashMap::new();
            if let Some(Value::Map(opts)) = m.get(&Value::Integer(0x04)) {
                for (k, v) in opts {
                    if let (Value::Text(name), Value::Bool(val)) = (k, v) {
                        options_map.insert(name.clone(), *val);
                    }
                }
            }

            let get_strings = |key: i128| -> Vec<String> {
                match m.get(&Value::Integer(key)) {
                    Some(Value::Array(arr)) => arr.iter().filter_map(|v| {
                        if let Value::Text(s) = v { Some(s.clone()) } else { None }
                    }).collect(),
                    _ => Vec::new(),
                }
            };

            let get_u32 = |key: i128| -> u32 {
                match m.get(&Value::Integer(key)) {
                    Some(Value::Integer(v)) => *v as u32,
                    _ => 0,
                }
            };

            Ok(FidoDeviceInfo {
                versions: get_strings(0x01),
                extensions: get_strings(0x02),
                aaguid: match m.get(&Value::Integer(0x03)) {
                    Some(Value::Bytes(b)) => hex::encode_upper(b),
                    _ => String::new(),
                },
                options: options_map,
                max_msg_size: get_u32(0x05) as i32,
                pin_protocols: match m.get(&Value::Integer(0x06)) {
                    Some(Value::Array(arr)) => arr.iter().filter_map(|v| {
                        if let Value::Integer(i) = v { Some(*i as u32) } else { None }
                    }).collect(),
                    _ => Vec::new(),
                },
                min_pin_length: get_u32(0x0D),
                firmware_version: format!("{}.{}", get_u32(0x0E) >> 8, get_u32(0x0E) & 0xFF),
            })
        } else {
            Err("Invalid GetInfo response".into())
        }
    }

    pub fn get_memory_stats(&self) -> Result<Value, String> {
        let mut map = BTreeMap::new();
        map.insert(Value::Integer(0x01), Value::Integer(0x01));
        let payload = to_vec(&Value::Map(map)).map_err(|e| e.to_string())?;

        let mut data = vec![0x06];
        data.extend(payload);

        self.write_packet(0x10, &data)?;
        let response = self.read_response()?;
        
        if response.is_empty() || response[0] != 0x00 {
            return Err(format!("CTAP error: {:02X?}", response));
        }
        
        from_slice(&response[1..]).map_err(|e| e.to_string())
    }

    pub fn get_memory_stats_structured(&self) -> Result<PicoMemoryStats, String> {
        let stats_value = self.get_memory_stats()?;
        
        if let Value::Map(m) = stats_value {
            let get_u64 = |key: i128| -> u64 {
                match m.get(&Value::Integer(key)) {
                    Some(Value::Integer(v)) => *v as u64,
                    _ => 0,
                }
            };

            Ok(PicoMemoryStats {
                free: get_u64(0x01),
                used: get_u64(0x02),
                total: get_u64(0x03),
                files: get_u64(0x04),
                flash_size: get_u64(0x05),
            })
        } else {
            Err("Invalid memory stats format".into())
        }
    }

    pub fn list_credentials(&self, pin_auth: Vec<u8>) -> Result<Vec<PicoCredential>, String> {
        let mut map = BTreeMap::new();
        map.insert(Value::Integer(0x01), Value::Integer(0x01)); 
        map.insert(Value::Integer(0x03), Value::Integer(1)); 
        map.insert(Value::Integer(0x04), Value::Bytes(pin_auth));

        let payload = to_vec(&Value::Map(map)).map_err(|e| e.to_string())?;
        let mut data = vec![0x0A]; 
        data.extend(payload);

        self.write_packet(0x10, &data)?; 
        let response = self.read_response()?;
        
        let credentials_val: Value = from_slice(&response[1..]).map_err(|e| e.to_string())?;
        
        let mut result = Vec::new();
            if let Value::Array(creds) = credentials_val {
                for c in creds {
                    if let Value::Map(cm) = c {
                        let get_string = |key: i128| -> String {
                            match cm.get(&Value::Integer(key)) {
                                Some(Value::Text(s)) => s.clone(),
                                _ => String::new(),
                            }
                        };

                        let get_bytes = |key: i128| -> Vec<u8> {
                            match cm.get(&Value::Integer(key)) {
                                Some(Value::Bytes(b)) => b.clone(),
                                _ => Vec::new(),
                            }
                        };

                        result.push(PicoCredential {
                            rp_id: get_string(0x03),
                            user_id: get_string(0x04),
                            user_name: get_string(0x05),
                            user_display_name: get_string(0x06),
                            credential_id: hex::encode(get_bytes(0x07)),
                        });
                    }
                }
            }
            Ok(result)
        }

    pub fn delete_credential(&self, credential_id: Vec<u8>, pin_auth: Vec<u8>) -> Result<(), String> {
        let mut map = BTreeMap::new();
        map.insert(Value::Integer(0x01), Value::Integer(0x03)); 
        map.insert(Value::Integer(0x02), Value::Bytes(credential_id));
        map.insert(Value::Integer(0x03), Value::Integer(1)); 
        map.insert(Value::Integer(0x04), Value::Bytes(pin_auth));

        let payload = to_vec(&Value::Map(map)).map_err(|e| e.to_string())?;
        let mut data = vec![0x0A];
        data.extend(payload);

        self.write_packet(0x10, &data)?;
        let _ = self.read_response()?;
        Ok(())
    }

    pub fn set_hardware_config(&self, command_id: u64, value: u64, pin_auth: Vec<u8>) -> Result<(), String> {
        let mut subpara = BTreeMap::new();
        subpara.insert(Value::Integer(0x01), Value::Integer(command_id as i128));
        subpara.insert(Value::Integer(0x03), Value::Integer(value as i128));

        let mut map = BTreeMap::new();
        map.insert(Value::Integer(0x01), Value::Integer(0xFF)); 
        map.insert(Value::Integer(0x02), Value::Map(subpara));
        map.insert(Value::Integer(0x03), Value::Integer(1)); 
        map.insert(Value::Integer(0x04), Value::Bytes(pin_auth));

        let payload = to_vec(&Value::Map(map)).map_err(|e| e.to_string())?;
        let mut data = vec![0x0D]; 
        data.extend(payload);

        self.write_packet(0x10, &data)?; 
        let _ = self.read_response()?;
        Ok(())
    }

    pub fn set_led_brightness(&self, level: u8, pin_auth: Vec<u8>) -> Result<(), String> {
        self.set_hardware_config(0x76a85945985d02fd, level as u64, pin_auth)
    }

    pub fn update_vid_pid(&self, vid: u16, pid: u16, pin_auth: Vec<u8>) -> Result<(), String> {
        let val = ((vid as u64) << 16) | (pid as u64);
        self.set_hardware_config(0x6fcb19b0cbe3acfa, val, pin_auth)
    }

    pub fn enable_secure_lock(&self, pin_auth: Vec<u8>) -> Result<(), String> {
        self.set_hardware_config(0x03e43f56b34285e2, 0, pin_auth)
    }
}

pub struct PicoState(pub Mutex<Option<PicoVendorClient>>);

#[tauri::command]
pub async fn discover_fido_device(
    state: State<'_, PicoState>,
) -> Result<String, String> {
    let client = PicoVendorClient::discover()?;
    let info = client.get_info_manual()?;
    
    let mut state_lock = state.0.lock().unwrap();
    *state_lock = Some(client);

    Ok(format!("Discovered FIDO device - AAGUID: {}", info.aaguid))
}

#[tauri::command]
pub async fn connect_pico_vendor(
    state: State<'_, PicoState>,
    vid: String,
    pid: String,
) -> Result<String, String> {
    let v = u16::from_str_radix(&vid, 16).map_err(|_| "Invalid VID")?;
    let p = u16::from_str_radix(&pid, 16).map_err(|_| "Invalid PID")?;

    let client = PicoVendorClient::new(v, p)?;
    let mut state_lock = state.0.lock().unwrap();
    *state_lock = Some(client);

    Ok("Connected to Pico via HID".into())
}

#[tauri::command]
pub fn get_fido_info(state: State<'_, PicoState>) -> Result<FidoDeviceInfo, String> {
	let lock = state.0.lock().unwrap();
	let client = lock.as_ref().ok_or("Device not connected. Please connect via HID first.")?;
	log::info!("get_fido_info: Attempting to fetch FIDO info from device");
	let result = client.get_info_manual();
	match &result {
		Ok(info) => log::info!("get_fido_info: Successfully retrieved FIDO info - AAGUID: {}", info.aaguid),
		Err(e) => log::error!("get_fido_info: Failed to get info - {}", e),
	}
	result
}

#[tauri::command]
pub(crate) fn change_fido_pin(
    current_pin: Option<String>,
    new_pin: String,
) -> Result<String, String> {
    let cfg = Cfg::init();
    let device = FidoKeyHidFactory::create(&cfg)
        .map_err(|e| format!("Failed to connect to FIDO device: {:?}", e))?;

    match current_pin {
        Some(old) => {
            device
                .change_pin(&old, &new_pin)
                .map_err(|e| format!("Failed to change PIN: {:?}", e))?;
            Ok("PIN Changed Successfully".into())
        }
        None => {
            device
                .set_new_pin(&new_pin)
                .map_err(|e| format!("Failed to set PIN: {:?}", e))?;
            Ok("PIN Set Successfully".into())
        }
    }
}

#[tauri::command]
pub(crate) fn set_min_pin_length(
    current_pin: String,
    min_pin_length: u8,
) -> Result<String, String> {
    let cfg = Cfg::init();
    let device = FidoKeyHidFactory::create(&cfg)
        .map_err(|e| format!("Failed to connect to FIDO device: {:?}", e))?;

    device
        .set_min_pin_length(min_pin_length, Some(&current_pin))
        .map_err(|e| format!("Failed to set minimum PIN length: {:?}", e))?;

    Ok(format!(
        "Minimum PIN length successfully set to {}",
        min_pin_length
    ))
}

#[tauri::command]
pub fn get_fido_memory_stats(state: State<'_, PicoState>) -> Result<PicoMemoryStats, String> {
    let lock = state.0.lock().unwrap();
    let client = lock.as_ref().ok_or("Device not connected via HID")?;
    client.get_memory_stats_structured()
}

#[tauri::command]
pub fn list_fido_credentials(
    state: State<'_, PicoState>,
    pin_auth: Vec<u8>
) -> Result<Vec<PicoCredential>, String> {
    let lock = state.0.lock().unwrap();
    let client = lock.as_ref().ok_or("Device not connected via HID")?;
    client.list_credentials(pin_auth)
}

#[tauri::command]
pub fn set_fido_led_brightness(
    state: State<'_, PicoState>,
    level: u8,
    pin_auth: Vec<u8>
) -> Result<String, String> {
    let lock = state.0.lock().unwrap();
    let client = lock.as_ref().ok_or("Device not connected via HID")?;
    client.set_led_brightness(level, pin_auth)?;
    Ok("Brightness updated".into())
}

#[tauri::command]
pub fn update_fido_vid_pid(
    state: State<'_, PicoState>,
    vid: u16,
    pid: u16,
    pin_auth: Vec<u8>
) -> Result<String, String> {
    let lock = state.0.lock().unwrap();
    let client = lock.as_ref().ok_or("Device not connected via HID")?;
    client.update_vid_pid(vid, pid, pin_auth)?;
    Ok("VID/PID updated. Re-plug device to apply.".into())
}