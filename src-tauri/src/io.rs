use crate::{fido::PicoState, types::{AppConfig, AppError, DeviceInfo, FullDeviceStatus}};

#[tauri::command]
pub async fn refresh_device_status(
	state: tauri::State<'_, PicoState>
) -> Result<FullDeviceStatus, AppError> {
	match crate::rescue::read_device_details() {
		Ok(details) => {
			let vid = u16::from_str_radix(&details.config.vid, 16).unwrap_or(0x1209);
			let pid = u16::from_str_radix(&details.config.pid, 16).unwrap_or(0x0001);
			
			if let Ok(client) = crate::fido::PicoVendorClient::new(vid, pid) {
				let mut state_lock = state.0.lock().unwrap();
				*state_lock = Some(client);
				drop(state_lock);
				
				log::info!("FIDO HID client initialized with VID:PID {}:{}", 
					details.config.vid, details.config.pid);
			} else {
				log::warn!("Failed to initialize FIDO client with VID:PID {}:{}", vid, pid);
			}
			
			Ok(details)
		},
        Err(e) => {
            log::warn!("Rescue mode failed: {}, trying FIDO auto-discovery", e);
            
            if let Ok(client) = crate::fido::PicoVendorClient::discover() {
                match client.get_info_manual() {
                    Ok(fido_info) => {
                        match client.get_memory_stats_structured() {
                            Ok(fido_stats) => {
                                let mut state_lock = state.0.lock().unwrap();
                                *state_lock = Some(client);
                                
                                return Ok(FullDeviceStatus {
                                    info: DeviceInfo {
                                        serial: fido_info.aaguid.clone(),
                                        flash_used: (fido_stats.used / 1024) as u32,
                                        flash_total: (fido_stats.total / 1024) as u32,
                                        firmware_version: fido_info.firmware_version.clone(),
                                    },
                                    config: AppConfig::default(),
                                    secure_boot: false,
                                    secure_lock: fido_info.options.get("clientPin").copied().unwrap_or(false),
                                });
                            },
                            Err(e) => {
                                log::warn!("FIDO memory stats unavailable: {}, using defaults", e);
                                let mut state_lock = state.0.lock().unwrap();
                                *state_lock = Some(client);
                                
                                return Ok(FullDeviceStatus {
                                    info: DeviceInfo {
                                        serial: fido_info.aaguid.clone(),
                                        flash_used: 0,
                                        flash_total: 0,
                                        firmware_version: fido_info.firmware_version.clone(),
                                    },
                                    config: AppConfig::default(),
                                    secure_boot: false,
                                    secure_lock: fido_info.options.get("clientPin").copied().unwrap_or(false),
                                });
                            }
                        }
                    },
                    Err(e) => {
                        return Err(AppError::Device(format!("FIDO get_info failed: {}", e)));
                    }
                }
            }
            
            Err(AppError::Device("Both Rescue and FIDO connections failed".into()))
        }
	}
}