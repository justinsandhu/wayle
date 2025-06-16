use std::path::Path;
use wayle::{Result, config::Config};

fn main() -> Result<()> {
    let config = Config::load_with_imports(Path::new("config.toml"))?;

    println!("Successfully loaded toml with imports");
    println!("{:#?}", config);

    if let Some(battery) = &config.modules.battery {
        println!(
            "\nBattery module: enabled={}, show_percentage={}",
            battery.enabled, battery.show_percentage
        );
    }

    println!("\nLog level: {}", config.general.log_level);

    Ok(())
}
