mod cli;

use anyhow::{anyhow, Context};
use clap::Parser;
use cliclack::Confirm;
use reqwest::blocking::Client;
use std::{
    fs,
    io::BufRead,
    path::PathBuf,
    process::{Child, Command},
    vec,
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
    let tmp = cli.tmp;
    let Resources {
        cwd,
        polkadot,
        apps,
    } = setup(cli)?;

    let mut processes: Vec<(&'static str, Child)> = vec![];

    if let Some(apps) = &apps {
        // Run polkadot-js explorer
        processes.push((
            "polkadot-js",
            run_process("yarn", &["run", "start"], apps.to_str().unwrap(), false),
        ));
    }
    // Run Polkadot process
    processes.push((
        "polkadot",
        run_process(
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
        ),
    ));

    println!("\x1b[1m========= Press Ctrl-C to terminate all processes =========\x1b[0m");
    rx.recv().expect("Could not receive from channel.");
    println!("\nCleaning up and Exiting...");
    for (name, mut p) in processes {
        println!("Killing {}", name);
        p.kill().expect(&f!("Failed to kill {:?}", name));
    }
    println!("All processes killed successfully.");
    if tmp {
        fs::remove_dir_all(&cwd)?;
        println!("Removed temp dir {}", cwd.display());
    }
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

fn setup(cli: cli::Cli) -> anyhow::Result<Resources> {
    let mut cwd = PathBuf::from(match &cli.path {
        Some(path) => path.clone(),
        None => std::env::var("HOME").map_err(|_| anyhow!("$HOME not found, re-run with --tmp or --path to specify where to download polkadot and polkadotjs"))?
    });
    // --tmp and --fresh flags are mutually exclusive
    if cli.tmp {
        cwd = std::env::temp_dir();
    } else if cli.fresh {
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
            fs::remove_dir_all(cwd)?;
        }
    }
    // We are either using the default or a temporary directory
    if cli.path.is_none() {
        // Create .local-polkadot directory
        cwd = cwd.join(".local-polkadot");
        if !cwd.exists() {
            println!("Creating .local-polkadot @ {}", cwd.display());
            fs::create_dir(&cwd).map_err(|e| anyhow!("Failed to create .local-polkadot {}", e))?;
        }
    } else {
        // --path was provided but the directory does not exist on local fs
        if !cwd.exists() {
            let confirm =
                Confirm::new(f!("Directory {} does not exist. Create it?", cwd.display()))
                    .initial_value(true)
                    .interact()
                    .map_err(|e| anyhow!("Failed to prompt for confirmation: {}", e))?;
            if confirm {
                fs::create_dir_all(&cwd).map_err(|e| {
                    anyhow!("Failed to create directory `{}`: {}", cwd.display(), e)
                })?;
            } else {
                println!("Exiting...");
                std::process::exit(1);
            }
        }
    }
    // Download
    let client = Client::new();
    std::thread::scope(|s| {
        // Polkadot binary
        s.spawn(|| -> anyhow::Result<()> {
            if cwd.join("polkadot").exists() {
                println!("Polkadot already exists in the specified path. Skipping download...");
            } else {
                println!("Downloading polkadot into {}...", cwd.display());

                // Download the file
                client
                    .get(POLKADOT)
                    .send()
                    .context(f!("Failed to download {}", POLKADOT))?
                    .copy_to(&mut fs::File::create(cwd.join("polkadot"))?)
                    .expect("returns binary data");

                println!("Polkadot exe download completed.");
            }
            Ok(())
        });
        // Polkadot-js/apps zip
        s.spawn(|| -> anyhow::Result<()> {
            if !cli.skip_polkadotjs {
                // Download pjs.zip and unzip it into apps-master
                if cwd.join("apps-master").exists() {
                    println!(
                        "PolkadotJS already exists in the specified path. Skipping download..."
                    );
                } else if !cwd.join("apps-master").exists() && cwd.join("pjs.zip").exists() {
                    println!(
                        "PolkadotJS already exists in the specified path. Skipping download..."
                    );
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

                    client
                        .get(PJS)
                        .send()
                        .context(f!("Failed to download {}", PJS))?
                        .copy_to(&mut fs::File::create(cwd.join("pjs.zip"))?)
                        .map_err(|e| anyhow!("Failed to write to file: {}", e))?;

                    println!("Polkadot-js download completed.");
                    // Unzip the downloaded file
                    println!("Extracting pjs.zip into {}/apps-master...", cwd.display());
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
                    println!("Extraction of pjs.zip completed.");
                }
            }
            Ok(())
        });
    });

    // FINALLY make everything ready to execute
    // Make the downloaded file executable
    let _ = Command::new("chmod")
        .arg("+x")
        .arg(cwd.join("polkadot"))
        .output()
        .map_err(|e| anyhow!("Failed to make file executable: {}", e))?;
    // Run yarn install
    println!("Running yarn install...");
    if !cli.skip_polkadotjs {
        let _ = Command::new("yarn")
            .arg("install")
            .current_dir(cwd.join("apps-master"))
            .output()
            .map_err(|e| anyhow!("Failed to run yarn install: {}", e))?;
    }
    println!("Yarn install completed.");

    Ok(Resources {
        cwd: cwd.clone(),
        polkadot: cwd.join("polkadot"),
        apps: if cli.skip_polkadotjs {
            None
        } else {
            Some(cwd.join("apps-master"))
        },
    })
}

struct Resources {
    cwd: PathBuf,
    polkadot: PathBuf,
    // Maybe skipped with --skip-polkadotjs / --skip-pjs
    apps: Option<PathBuf>,
}
