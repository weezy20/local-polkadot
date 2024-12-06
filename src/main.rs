mod cli;

use anyhow::{anyhow, Context};
use clap::Parser;
use cliclack::Confirm;
use console::style;
use crossterm::{
    cursor, execute,
    terminal::{Clear, ClearType},
};
use reqwest::blocking::Client;
use std::{
    fs,
    io::{self, BufRead},
    path::{Path, PathBuf},
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
        let yarn = which::which("yarn")
            .context("`yarn` not found in PATH. Please install yarn and try again.")?;
        processes.push((
            "polkadot-js",
            run_process(&yarn, &["run", "start"], &apps, false)?,
        ));
    }
    // Run Polkadot process
    processes.push((
        "polkadot",
        run_process(
            &fs::canonicalize(polkadot)?,
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
            &cwd,
            true,
        )?,
    ));
    println!(
        "\x1b[1m{}: {}\x1b[0m",
        style("Explorer").cyan(),
        style("http://localhost:3000").green()
    );
    println!(
        "\x1b[1m{}:    {}\x1b[0m",
        style("Local").red(),
        style("http://localhost:3000/?rpc=ws%3A%2F%2F127.0.0.1%3A9944#/explorer").green()
    );
    println!("\x1b[1m========= Press Ctrl-C to terminate all processes =========\x1b[0m");
    rx.recv().expect("Could not receive from channel.");
    execute!(io::stdout(), Clear(ClearType::FromCursorDown)).unwrap();
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

fn run_process(
    command: &Path,
    args: &[&str],
    working_dir: &Path,
    capture_log: bool,
) -> anyhow::Result<Child> {
    let mut cmd = Command::new(command);
    if !capture_log {
        cmd.args(args)
            .current_dir(working_dir.canonicalize()?)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .with_context(|| f!("Failed to start {:?} in {}", command, working_dir.display()))
    } else {
        let mut child = cmd
            .args(args)
            .current_dir(working_dir.canonicalize()?)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .with_context(|| f!("Failed to start {:?} in {}", command, working_dir.display()))?;

        // capture_log maybe used only for node-processes as such, it is sensible to only spawn a thread to print stderr
        let stderr = child.stderr.take().expect("Failed to open stderr");
        execute!(io::stdout(), Clear(ClearType::FromCursorUp)).unwrap();

        std::thread::spawn(move || {
            let reader = std::io::BufReader::new(stderr);
            for line in reader.lines() {
                // Move the cursor to the position below the pinned lines
                execute!(io::stdout(), cursor::MoveTo(0, 0)).unwrap();
                // Clear the line before printing new log
                execute!(io::stdout(), Clear(ClearType::UntilNewLine)).unwrap();
                // Print log
                eprintln!("{}", line.expect("Failed to read line from stderr"));
            }
        });
        // Return handle to the spawned process
        Ok(child)
    }
}

fn setup(cli: cli::Cli) -> anyhow::Result<Resources> {
    let mut cwd = PathBuf::from(match &cli.path {
        Some(path) => path.clone(),
        None => PathBuf::from(&std::env::var("HOME").map_err(|_| {
            anyhow!(
                "User $HOME not found, re-run with --path to specify where to download artifacts"
            )
        })?),
    });
    // --tmp and --fresh flags are mutually exclusive
    // cwd/.local-polkadot
    cwd = if cli.tmp {
        let s: String = std::iter::repeat_with(fastrand::alphanumeric)
            .take(10)
            .collect();
        // --tmp + --path <path>
        let tmp_dir = if cli.path.is_some() {
            cwd.join(f!(".tmp-local-polkadot-{s}")) // allow --path create_dir_all to be handled downstream
        } else {
            // --tmp only
            let tmp_dir = PathBuf::from("/tmp").join(f!(".tmp-local-polkadot-{}", s)); // create immediately
            fs::create_dir(&tmp_dir)
                .map_err(|e| anyhow!(f!("Failed to create temporary dir {:?}", e)))?;
            tmp_dir
        };
        tmp_dir
    } else {
        cwd.join(".local-polkadot")
    };
    if cwd.exists() && cwd.is_dir() && cli.fresh {
        let confirm = Confirm::new(f!("Are you sure you want to remove {}?", cwd.display()))
            .initial_value(true)
            .interact()
            .map_err(|e| anyhow!("Failed to prompt for confirmation: {}", e))?;

        if !confirm {
            println!("Not removing (--fresh was a no-op){}", cwd.display());
        } else {
            fs::remove_dir_all(&cwd)?;
            fs::create_dir(&cwd)
                .map_err(|e| anyhow!("Failed to create directory `{}`: {}", cwd.display(), e))?;
        }
    }
    // Create .local-polkadot directory if not created (--path <to/non/existing/dir>)
    // Or using $HOME/.local-polkadot for the first time (no-prompt)
    if !cwd.exists() {
        // if --path is specified to a non-existing directory tree, we confirm before
        // running create_dir_all
        if cli.path.is_some() {
            let confirm = match cli.tmp {
                true => true,
                false => Confirm::new(f!("Directory {} does not exist. Create it?", cwd.display()))
                    .initial_value(true)
                    .interact()
                    .map_err(|e| anyhow!("Failed to prompt for confirmation: {}", e))?,
            };
            if confirm {
                fs::create_dir_all(&cwd).map_err(|e| {
                    anyhow!("Failed to create directory `{}`: {}", cwd.display(), e)
                })?;
            } else {
                println!("Exiting...");
                std::process::exit(1);
            }
        } else {
            println!("Creating local-polkadot directory at {}", cwd.display());
            fs::create_dir(&cwd).map_err(|e| {
                anyhow!(
                    "Failed to create local-polkadot directory {} due to {}",
                    cwd.display(),
                    e
                )
            })?;
        }
    }

    // Download
    let multi = cliclack::multi_progress("Spinning up the wheels 🚗");
    let client = Client::new();
    std::thread::scope(|s| {
        // Polkadot binary
        s.spawn(|| -> anyhow::Result<()> {
            if cwd.join("polkadot").exists() {
                println!("Polkadot already exists in the specified path. Skipping download...");
            } else {
                let spinner = multi.add(cliclack::spinner());
                spinner.start(f!("Downloading polkadot into {}...", cwd.display()));
                // Download the file
                // println!("Downloading Polkadot exe ...");
                client
                    .get(POLKADOT)
                    .send()
                    .context(f!("Failed to download {}", POLKADOT))?
                    .copy_to(&mut fs::File::create(cwd.join("polkadot"))?)
                    .expect("returns binary data");

                // println!("Polkadot exe download completed.");
                spinner.stop(f!(
                    "{} Polkadot exe download completed.",
                    style("✔").green()
                ));
            }
            Ok(())
        });

        // Polkadot-js/apps zip
        s.spawn(|| -> anyhow::Result<()> {
            let spinner = multi.add(cliclack::spinner());
            if !cli.skip_polkadotjs {
                if cwd.join("apps-master").exists() {
                    println!(
                        "Polkadot-js already exists in the specified path. Skipping download..."
                    );
                } else if cwd.join("pjs.zip").exists() {
                    println!(
                        "Polkadot-js zip already exists in the specified path. Skipping download..."
                    );
                    spinner.start(f!(
                        "Extracting pjs.zip into {}/apps-master...",
                        cwd.display()
                    ));
                    let mut archive = zip::ZipArchive::new(fs::File::open(cwd.join("pjs.zip"))?)?;
                    unzip(&mut archive, &cwd)?;
                    spinner.stop("Extraction of pjs.zip completed.");
                } else {
                    spinner.start(f!(
                        "Downloading polkadot-js into {}/apps-master...",
                        cwd.display()
                    ));

                    client
                        .get(PJS)
                        .send()
                        .context(f!("Failed to download {}", PJS))?
                        .copy_to(&mut fs::File::create(cwd.join("pjs.zip"))?)
                        .map_err(|e| anyhow!("Failed to write to file: {}", e))?;

                    spinner.set_message(f!("Polkadot-js download completed."));
                    // Unzip the downloaded file
                    spinner.set_message(f!(
                        "Extracting pjs.zip into {}/apps-master...",
                        cwd.display()
                    ));
                    let mut archive = zip::ZipArchive::new(fs::File::open(cwd.join("pjs.zip"))?)?;
                    unzip(&mut archive, &cwd)?;
                    spinner.set_message("Extraction of pjs.zip completed.");
                    // Run yarn install if applicable
                    if !cli.skip_polkadotjs {
                        spinner.set_message(f!(
                            "Running yarn install in {}...",
                            cwd.join("apps-master").display()
                        ));
                        let yarn = which::which("yarn").context(
                            "`yarn` not found in PATH. Please install yarn and try again.",
                        )?;
                        let _ = Command::new(yarn)
                            .arg("install")
                            .current_dir(cwd.join("apps-master"))
                            .output()
                            .map_err(|e| {
                                anyhow!(
                                    "Failed to run yarn install in {}: {}",
                                    cwd.join("apps-master").display(),
                                    e
                                )
                            })?;
                        spinner.set_message("Yarn install completed");
                        spinner.stop(f!(
                            "{} Polkadot-js installation completed",
                            style("✔").green()
                        ));
                    }
                }
            }
            Ok(())
        });
    });
    multi.stop();
    // FINALLY make everything ready to execute
    // Make the downloaded file executable
    let _ = Command::new("chmod")
        .arg("+x")
        .arg(cwd.join("polkadot"))
        .output()
        .map_err(|e| anyhow!("Failed to make file executable: {}", e))?;

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

fn unzip(archive: &mut zip::ZipArchive<fs::File>, cwd: &PathBuf) -> anyhow::Result<()> {
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        let outpath = match file.enclosed_name() {
            Some(path) => cwd.join(path),
            None => continue,
        };
        if file.is_dir() {
            fs::create_dir_all(&outpath).unwrap();
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(p).unwrap();
                }
            }
            let mut outfile = fs::File::create(&outpath).unwrap();
            std::io::copy(&mut file, &mut outfile).unwrap();
        }

        // Get and Set permissions
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            if let Some(mode) = file.unix_mode() {
                fs::set_permissions(&outpath, fs::Permissions::from_mode(mode)).unwrap();
            }
        }
    }
    Ok(())
}
