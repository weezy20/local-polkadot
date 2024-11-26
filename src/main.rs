use std::{
    io::BufRead,
    process::{Child, Command},
};

macro_rules! f {
    ($($arg:tt)*) => {
        format!($($arg)*)
    };
}

fn main() -> Result<(), String> {
    let (tx, rx) = std::sync::mpsc::channel();
    ctrlc::set_handler(move || tx.send(()).expect("Could not send signal on channel."))
        .expect("Error setting Ctrl-C handler");
    // Run polkadot-js explorer
    let mut c1 = run_process(
        "bash",
        &["-c", "bun run start"],
        "/home/shah/pro/pjs",
        false,
    );

    // Run Polkadot process
    let mut c2 = run_process(
        "bash",
        &["-c", "./target/release/polkadot --chain polkadot --tmp --name myrpc --sync warp --rpc-cors all --rpc-methods Safe --rpc-port 9944 --no-telemetry"],
        "/home/shah/pro/polkadot-sdk",
        true
    );

    println!("Waiting for Ctrl-C...");
    rx.recv().expect("Could not receive from channel.");
    println!("\nGot it! Exiting...");
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
