use std::{error::Error, sync::Arc};

use async_trait::async_trait;
use futures::Stream;

use super::{DeviceInfo, DeviceKey, DeviceType};
use crate::services::pulse::{AudioEvent, PulseService, volume::Volume};

/// Device management operations
#[async_trait]
pub trait DeviceManager {
    /// Error type for device operations
    type Error: Error + Send + Sync + 'static;

    /// Get specific device information
    ///
    /// # Errors
    /// Returns error if device is not found or communication fails
    async fn device(&self, device_key: DeviceKey) -> Result<DeviceInfo, Self::Error>;

    /// Get devices filtered by type
    ///
    /// # Errors
    /// Returns error if communication with audio backend fails
    async fn devices_by_type(
        &self,
        device_type: DeviceType,
    ) -> Result<Vec<DeviceInfo>, Self::Error>;

    /// Get current default input device
    ///
    /// # Errors
    /// Returns error if communication with audio backend fails
    async fn default_input(&self) -> Result<Option<DeviceInfo>, Self::Error>;

    /// Get current default output device
    ///
    /// # Errors
    /// Returns error if communication with audio backend fails
    async fn default_output(&self) -> Result<Option<DeviceInfo>, Self::Error>;

    /// Set default input device
    ///
    /// # Errors
    /// Returns error if device is not found or operation fails
    async fn set_default_input(&self, device_key: DeviceKey) -> Result<(), Self::Error>;

    /// Set default output device
    ///
    /// # Errors
    /// Returns error if device is not found or operation fails
    async fn set_default_output(&self, device_key: DeviceKey) -> Result<(), Self::Error>;
}

/// Device volume control operations
#[async_trait]
pub trait DeviceVolumeController {
    /// Error type for volume operations
    type Error: Error + Send + Sync + 'static;

    /// Set device volume using percentage level (0.0 to 4.0)
    ///
    /// Automatically handles channel count matching the device configuration.
    /// Volume level applies uniformly across all device channels.
    ///
    /// # Arguments
    /// * `device` - Target device index
    /// * `level` - Volume level where 0.0=mute, 1.0=normal, 4.0=maximum
    ///
    /// # Errors
    /// Returns error if device is not found, level is invalid, or operation fails
    async fn set_device_volume(&self, device_key: DeviceKey, level: f64)
    -> Result<(), Self::Error>;

    /// Set device mute state
    ///
    /// # Errors
    /// Returns error if device is not found or operation fails
    async fn set_device_mute(&self, device_key: DeviceKey, muted: bool) -> Result<(), Self::Error>;
}

/// Device monitoring streams
pub trait DeviceStreams {
    /// Stream of all devices
    fn devices(&self) -> impl Stream<Item = Vec<DeviceInfo>> + Send;

    /// Stream of input devices only
    fn input_devices(&self) -> impl Stream<Item = Vec<DeviceInfo>> + Send;

    /// Stream of output devices only
    fn output_devices(&self) -> impl Stream<Item = Vec<DeviceInfo>> + Send;

    /// Stream of default input device changes
    fn default_input(&self) -> impl Stream<Item = DeviceInfo> + Send;

    /// Stream of default output device changes
    fn default_output(&self) -> impl Stream<Item = DeviceInfo> + Send;

    /// Stream of volume changes for specific device
    fn device_volume(&self, device_key: DeviceKey) -> impl Stream<Item = Volume> + Send;

    /// Stream of mute changes for specific device
    fn device_mute(&self, device_key: DeviceKey) -> impl Stream<Item = bool> + Send;

    /// Stream of state changes for specific device
    fn device_state(&self, device_key: DeviceKey) -> impl Stream<Item = DeviceInfo> + Send;
}

impl DeviceStreams for PulseService {
    fn devices(&self) -> impl Stream<Item = Vec<DeviceInfo>> + Send {
        use async_stream::stream;

        let devices = Arc::clone(&self.devices);
        let mut device_list_rx = self.device_list_tx.subscribe();

        stream! {
            let mut current_devices = if let Ok(devices_guard) = devices.read() {
                let device_list: Vec<DeviceInfo> = devices_guard.values().cloned().collect();
                device_list
            } else {
                Vec::new()
            };

            if !current_devices.is_empty() {
                yield current_devices.clone();
            }

            while let Ok(device_list) = device_list_rx.recv().await {
                if current_devices != device_list {
                    current_devices = device_list.clone();
                    yield device_list;
                }
            }
        }
    }

    fn input_devices(&self) -> impl Stream<Item = Vec<DeviceInfo>> + Send {
        use async_stream::stream;
        use futures::{StreamExt, pin_mut};

        let devices_stream = self.devices();
        stream! {
            pin_mut!(devices_stream);
            while let Some(device_list) = devices_stream.next().await {
                let input_devices: Vec<DeviceInfo> = device_list
                    .into_iter()
                    .filter(|d| d.device_type == DeviceType::Input)
                    .collect();
                yield input_devices;
            }
        }
    }

    fn output_devices(&self) -> impl Stream<Item = Vec<DeviceInfo>> + Send {
        use async_stream::stream;
        use futures::{StreamExt, pin_mut};

        let devices_stream = self.devices();
        stream! {
            pin_mut!(devices_stream);
            while let Some(device_list) = devices_stream.next().await {
                let output_devices: Vec<DeviceInfo> = device_list
                    .into_iter()
                    .filter(|d| d.device_type == DeviceType::Output)
                    .collect();
                yield output_devices;
            }
        }
    }

    fn default_input(&self) -> impl Stream<Item = DeviceInfo> + Send {
        use async_stream::stream;

        let default_input = Arc::clone(&self.default_input);
        let mut events_rx = self.events_tx.subscribe();

        stream! {
            {
                let device_opt = if let Ok(default_guard) = default_input.read() {
                    default_guard.as_ref().cloned()
                } else {
                    None
                };
                if let Some(device) = device_opt {
                    yield device;
                }
            }

            while let Ok(AudioEvent::DefaultInputChanged(device_info)) = events_rx.recv().await {
                yield device_info;
            }
        }
    }

    fn default_output(&self) -> impl Stream<Item = DeviceInfo> + Send {
        use async_stream::stream;

        let default_output = Arc::clone(&self.default_output);
        let mut events_rx = self.events_tx.subscribe();

        stream! {
            {
                let device_opt = if let Ok(default_guard) = default_output.read() {
                    default_guard.as_ref().cloned()
                } else {
                    None
                };
                if let Some(device) = device_opt {
                    yield device;
                }
            }

            while let Ok(AudioEvent::DefaultOutputChanged(device_info)) = events_rx.recv().await {
                yield device_info;
            }
        }
    }

    fn device_volume(&self, device: DeviceKey) -> impl Stream<Item = Volume> + Send {
        use async_stream::stream;
        use futures::{StreamExt, pin_mut};

        let events_stream = self.events();
        stream! {
            pin_mut!(events_stream);
            while let Some(event) = events_stream.next().await {
                if let AudioEvent::DeviceVolumeChanged { device_key, volume, .. } = event {
                    if device_key == device {
                        yield volume;
                    }
                }
            }
        }
    }

    fn device_mute(&self, device: DeviceKey) -> impl Stream<Item = bool> + Send {
        use async_stream::stream;
        use futures::{StreamExt, pin_mut};

        let events_stream = self.events();
        stream! {
            pin_mut!(events_stream);
            while let Some(event) = events_stream.next().await {
                if let AudioEvent::DeviceMuteChanged { device_key, muted, .. } = event {
                    if device == device_key {
                        yield muted;
                    }
                }
            }
        }
    }

    fn device_state(&self, device_key: DeviceKey) -> impl Stream<Item = DeviceInfo> + Send {
        use async_stream::stream;
        use futures::{StreamExt, pin_mut};

        let events_stream = self.events();
        stream! {
            pin_mut!(events_stream);
            while let Some(event) = events_stream.next().await {
                match event {
                    AudioEvent::DeviceChanged(device_info) if device_info.key == device_key => {
                        yield device_info;
                    }
                    _ => {}
                }
            }
        }
    }
}
