use crate::services::audio::{AudioService, PulseAudioService};
use futures::StreamExt;

/// Test function for PulseAudio reactive streams
#[allow(missing_docs)]
pub async fn test_pulse_reactive_streams() {
    println!("Creating PulseAudio service...");

    let service = match PulseAudioService::new().await {
        Ok(service) => {
            println!("✅ PulseAudio service created successfully");
            service
        }
        Err(e) => {
            println!("❌ Failed to create PulseAudio service: {e}");
            return;
        }
    };

    println!("Getting device stream...");
    let mut device_stream = service.devices();

    println!("⏳ Waiting for initial device list...");
    match tokio::time::timeout(tokio::time::Duration::from_secs(5), device_stream.next()).await {
        Ok(Some(devices)) => {
            println!("✅ Got {} devices:", devices.len());
            for device in &devices {
                println!(
                    "  - {} ({}): {} [{}]",
                    device.name.as_str(),
                    device.index.0,
                    device.description,
                    match device.device_type {
                        crate::services::audio::DeviceType::Input => "Input",
                        crate::services::audio::DeviceType::Output => "Output",
                    }
                );
            }

            if devices.is_empty() {
                println!(
                    "⚠️  No devices found - this might indicate a problem with PulseAudio/PipeWire compatibility"
                );
            }
        }
        Ok(None) => {
            println!("❌ Device stream ended without yielding devices");
            return;
        }
        Err(_) => {
            println!("⏰ Timeout waiting for device list");
            return;
        }
    }

    println!("✅ Reactive stream test successful!");
    println!("✅ Callback-to-stream conversion working!");

    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    println!("Test completed");
}
