//! TODO: Docs
use clap::{Parser, Subcommand};
use wayle::docs::DocsGenerator;

#[derive(Parser)]
#[command(name = "generate-docs")]
#[command(about = "Generate documentation for Wayle modules")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    All {
        #[arg(short, long, default_value = "docs/config/modules")]
        output: String,
    },
    Module {
        name: String,
        #[arg(short, long, default_value = "docs/config/modules")]
        output: String,
    },
    List,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::All { output } => {
            let generator = DocsGenerator::new().with_output_dir(output);
            generator.generate_all()?;
        }
        Commands::Module { name, output } => {
            let generator = DocsGenerator::new().with_output_dir(output);
            generator.generate_module_by_name(&name)?;
        }
        Commands::List => {
            let generator = DocsGenerator::new();
            let modules = generator.list_modules();
            println!("Available modules:");
            for module in modules {
                println!("  - {}", module);
            }
        }
    }

    Ok(())
}
