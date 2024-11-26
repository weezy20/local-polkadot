mod cli;

use anyhow::anyhow;
use clap::Parser;
use cliclack::Confirm;
use std::{
    io::BufRead,
    path::PathBuf,
    process::{Child, Command},
};
macro_rules! f {
    ($($arg:tt)*) => {
        format!($($arg)*)
    };
}

const PJS: &'static str = "https://github.com/polkadot-js/apps/archive/refs/heads/master.zip";
const POLKADOT: &'static str =
    "https://github.com/paritytech/polkadot-sdk/releases/latest/download/polkadot";

fn main() -> anyhow::Result<()> {
    let cli = cli::Cli::parse();
    let (tx, rx) = std::sync::mpsc::channel();
    ctrlc::set_handler(move || tx.send(()).expect("Could not send signal on channel."))
        .expect("Error setting Ctrl-C handler");
    let (cwd, polkadot, apps) = setup(cli)?;

    // Run polkadot-js explorer
    let mut c1 = run_process("bun", &["run", "start"], apps.to_str().unwrap(), false);

    // Run Polkadot process
    let mut c2 = run_process(
        polkadot.to_str().unwrap(),
        &[
            "--chain",
            "polkadot",
            "--tmp",
            "--name",
            "myrpc",
            "--sync",
            "warp",
            "--rpc-cors",
            "all",
            "--rpc-methods",
            "Safe",
            "--rpc-port",
            "9944",
            "--no-telemetry",
        ],
        cwd.to_str().unwrap(),
        true,
    );

    println!("Press Ctrl-C to terminate process");
    rx.recv().expect("Could not receive from channel.");
    println!("\nCleaning up and Exiting...");
    c1.kill()
        .expect("Failed to kill polkadot-js explorer process");
    c2.kill().expect("Failed to kill Polkadot process");
    println!("All processes killed successfully.");
    Ok(())
}

fn run_process(command: &str, args: &[&str], working_dir: &str, capture_log: bool) -> Child {
    let mut cmd = Command::new(command);
    if !capture_log {
        let child = cmd
            .args(args)
            .current_dir(working_dir)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect(&f!("Failed to start {:?}", command));
        // Return handle to the spawned process
        child
    } else {
        let mut child = cmd
            .args(args)
            .current_dir(working_dir)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect(&f!("Failed to start {:?}", command));

        let stdout = child.stdout.take().expect("Failed to open stdout");
        let stderr = child.stderr.take().expect("Failed to open stderr");

        std::thread::spawn(move || {
            let reader = std::io::BufReader::new(stdout);
            for line in reader.lines() {
                println!("stdout> {}", line.expect("Failed to read line from stdout"));
            }
        });

        std::thread::spawn(move || {
            let reader = std::io::BufReader::new(stderr);
            for line in reader.lines() {
                eprintln!("{}", line.expect("Failed to read line from stderr"));
            }
        });
        // Return handle to the spawned process
        child
    }
}

fn setup(cli: cli::Cli) -> anyhow::Result<(PathBuf, PathBuf, PathBuf)> {
    let mut cwd = PathBuf::from(match &cli.path {
        Some(path) => path.clone(),
        None => {
            std::env::var("HOME").map_err(|_| anyhow!("$HOME not found, re-run with --path to specify where to download polkadot and polkadotjs"))?
        }
    });
    if cli.fresh {
        let home = PathBuf::from(
            std::env::var("HOME").map_err(|_| anyhow!("$HOME not found, use --path"))?,
        );
        let cwd = home.join(".local-polkadot");
        let confirm = Confirm::new(f!("Are you sure you want to remove {}?", cwd.display()))
            .initial_value(true)
            .interact()
            .map_err(|e| anyhow!("Failed to prompt for confirmation: {}", e))?;

        if !confirm {
            println!("Not removing {}", cwd.display());
        } else {
            std::fs::remove_dir_all(cwd)?;
        }
    }
    // If using default path, we need to work with $HOME/.local-polkadot to keep things clean
    if cli.path.is_none() {
        // Create .local-polkadot directory
        cwd = cwd.join(".local-polkadot");
        if !cwd.exists() {
            println!(
                "Creating directory `$HOME/.local-polkadot` {}",
                cwd.display()
            );
            std::fs::create_dir(&cwd).map_err(|e| {
                anyhow!("Failed to create directory `$HOME/.local-polkadot`: {}", e)
            })?;
        }
    } else {
        if !cwd.exists() {
            std::fs::create_dir_all(&cwd)
                .map_err(|e| anyhow!("Failed to create directory `{}`: {}", cwd.display(), e))?;
        }
    }
    if cwd.join("polkadot").exists() {
        println!("Polkadot already exists in the specified path. Skipping download...");
    } else {
        println!("Downloading polkadot into {}...", cwd.display());

        // Download the file
        let output = Command::new("curl")
            .arg("-L")
            .arg(POLKADOT)
            .arg("-o")
            .arg(cwd.join("polkadot"))
            .output()
            .map_err(|e| anyhow!("Failed to start download: {}", e))?;

        if !output.status.success() {
            return Err(anyhow!(
                "Download failed with status: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        println!("Polkadot exe download completed.");
    }
    // Download pjs.zip and unzip it into apps-master
    if cwd.join("apps-master").exists() {
        println!("PolkadotJS already exists in the specified path. Skipping download...");
    } else if !cwd.join("apps-master").exists() && cwd.join("pjs.zip").exists() {
        println!("PolkadotJS already exists in the specified path. Skipping download...");
        println!("Unzipping pjs.zip into {}/apps-master...", cwd.display());

        // Unzip the downloaded file
        let output = Command::new("unzip")
            .arg(cwd.join("pjs.zip"))
            .arg("-d")
            .arg(&cwd)
            .output()
            .map_err(|e| anyhow!("Failed to unzip file: {}", e))?;

        if !output.status.success() {
            return Err(anyhow!(
                "Unzip failed with status: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
    } else {
        println!(
            "Downloading polkadot-js into {}/apps-master...",
            cwd.display()
        );

        // Download the file
        let output = Command::new("curl")
            .arg("-L")
            .arg(PJS)
            .arg("-o")
            .arg(cwd.join("pjs.zip"))
            .output()
            .map_err(|e| anyhow!("Failed to start download: {}", e))?;

        if !output.status.success() {
            return Err(anyhow!(
                "Download failed with status: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        println!("Download completed.");
        // Unzip the downloaded file
        let output = Command::new("unzip")
            .arg(cwd.join("pjs.zip"))
            .arg("-d")
            .arg(&cwd)
            .output()
            .map_err(|e| anyhow!("Failed to unzip file: {}", e))?;

        if !output.status.success() {
            return Err(anyhow!(
                "Unzip failed with status: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
    }
    // fINALLY make everything ready to execute
    // Make the downloaded file executable
    let _ = Command::new("chmod")
        .arg("+x")
        .arg(cwd.join("polkadot"))
        .output()
        .map_err(|e| anyhow!("Failed to make file executable: {}", e))?;
    // Run bun install
    let _ = Command::new("bun")
        .arg("install")
        .current_dir(cwd.join("apps-master"))
        .output()
        .map_err(|e| anyhow!("Failed to run bun install: {}", e))?;

    Ok((cwd.clone(), cwd.join("polkadot"), cwd.join("apps-master")))
}
