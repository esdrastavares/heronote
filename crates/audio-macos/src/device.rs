use cpal::traits::{DeviceTrait, HostTrait};
use heronote_audio_core::{AudioDevice, AudioError, DeviceType};

/// List all available audio devices on macOS
pub fn list_devices() -> Result<Vec<AudioDevice>, AudioError> {
    let host = cpal::default_host();
    let mut devices = Vec::new();

    let default_input = host
        .default_input_device()
        .and_then(|d| d.name().ok());
    let default_output = host
        .default_output_device()
        .and_then(|d| d.name().ok());

    // List input devices
    if let Ok(input_devices) = host.input_devices() {
        for device in input_devices {
            if let Ok(name) = device.name() {
                // Skip macOS TAP virtual devices
                if name.contains("TAP") {
                    continue;
                }

                let is_default = default_input.as_ref() == Some(&name);
                devices.push(AudioDevice::new(name, DeviceType::Input, is_default));
            }
        }
    }

    // List output devices
    if let Ok(output_devices) = host.output_devices() {
        for device in output_devices {
            if let Ok(name) = device.name() {
                let is_default = default_output.as_ref() == Some(&name);
                devices.push(AudioDevice::new(name, DeviceType::Output, is_default));
            }
        }
    }

    Ok(devices)
}

/// Get the default input device
pub fn get_default_input_device() -> Result<cpal::Device, AudioError> {
    let host = cpal::default_host();
    host.default_input_device()
        .ok_or(AudioError::NoDeviceFound)
}

/// Get a specific input device by name
pub fn get_input_device_by_name(name: &str) -> Result<cpal::Device, AudioError> {
    let host = cpal::default_host();

    if let Ok(devices) = host.input_devices() {
        for device in devices {
            if let Ok(device_name) = device.name() {
                if device_name == name {
                    return Ok(device);
                }
            }
        }
    }

    Err(AudioError::DeviceNotAvailable(name.to_string()))
}
