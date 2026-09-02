use std::{error::Error, path::PathBuf, process::ExitCode};

use apodize_core::instance::{
    CreateInstanceRequest, FileInstanceRepository, Instance, InstanceId, InstanceService,
};
use clap::{Parser, Subcommand};
use directories::ProjectDirs;

#[derive(Debug, Parser)]
#[command(name = "apodize", version, about = "A lightweight Minecraft launcher for the terminal")]
struct Cli {
    /// Override the Apodize data directory.
    #[arg(long, global = true, value_name = "PATH")]
    data_dir: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Manage Minecraft instances.
    Instance {
        #[command(subcommand)]
        command: InstanceCommand,
    },
}

#[derive(Debug, Subcommand)]
enum InstanceCommand {
    /// Create a new instance.
    Create {
        name: String,

        /// Minecraft version assigned to the instance.
        #[arg(long)]
        minecraft: String,
    },

    /// List instances.
    List,

    /// Show one instance.
    Show { id: InstanceId },

    /// Remove one instance.
    Remove { id: InstanceId },
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<(), Box<dyn Error>> {
    let data_dir = match cli.data_dir {
        Some(path) => path,
        None => default_data_dir()?,
    };
    let repository = FileInstanceRepository::new(data_dir.join("instances"));
    let service = InstanceService::new(repository);

    match cli.command {
        Command::Instance { command } => handle_instance_command(&service, command)?,
    }

    Ok(())
}

fn handle_instance_command(
    service: &InstanceService<FileInstanceRepository>,
    command: InstanceCommand,
) -> Result<(), Box<dyn Error>> {
    match command {
        InstanceCommand::Create { name, minecraft } => {
            let instance = service.create(CreateInstanceRequest::new(name, minecraft))?;
            println!("Created instance:");
            print_instance(&instance);
        }
        InstanceCommand::List => {
            let instances = service.list()?;
            if instances.is_empty() {
                println!("No instances found.");
                return Ok(());
            }

            println!("{:<36}  {:<24}  MINECRAFT", "ID", "NAME");
            for instance in instances {
                println!(
                    "{:<36}  {:<24}  {}",
                    instance.id(),
                    instance.name(),
                    instance.minecraft().version()
                );
            }
        }
        InstanceCommand::Show { id } => {
            let instance = service.get(&id)?;
            print_instance(&instance);
        }
        InstanceCommand::Remove { id } => {
            service.remove(&id)?;
            println!("Removed instance {id}");
        }
    }

    Ok(())
}

fn print_instance(instance: &Instance) {
    println!("  ID: {}", instance.id());
    println!("  Name: {}", instance.name());
    println!("  Minecraft: {}", instance.minecraft().version());

    if let Some(loader) = instance.minecraft().loader() {
        println!("  Loader: {} {}", loader.kind(), loader.version());
    }

    match instance.java().executable() {
        Some(path) => println!("  Java: {}", path.display()),
        None => println!("  Java: automatic"),
    }
}

fn default_data_dir() -> Result<PathBuf, &'static str> {
    ProjectDirs::from("", "", "apodize")
        .map(|dirs| dirs.data_dir().to_path_buf())
        .ok_or("could not determine the platform data directory; use --data-dir")
}
