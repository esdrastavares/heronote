use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DeviceType {
    Input,
    Output,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioDevice {
    pub name: String,
    pub device_type: DeviceType,
    pub is_default: bool,
}

impl AudioDevice {
    pub fn new(name: String, device_type: DeviceType, is_default: bool) -> Self {
        Self {
            name,
            device_type,
            is_default,
        }
    }
}
