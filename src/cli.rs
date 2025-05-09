use std::path::PathBuf;

#[derive(clap::Parser)]
pub struct Cli {
    /// Path to store artifacts, defaults to `$HOME/.local-polkadot`
    #[arg(long, short)]
    pub path: Option<PathBuf>,
    /// Use --tmp if you don't want to store anything locally
    /// All artifacts will be deleted after the process exits
    #[arg(long, conflicts_with = "fresh", default_value = "false")]
    pub tmp: bool,
    /// Cleanup existing artifacts (if present) by removing `$HOME/.local-polkadot`
    /// Useful when run as the only flag: `local-polkadot --fresh`
    #[arg(long, conflicts_with = "tmp", default_value = "false")]
    pub fresh: bool,

    /// Run local-polkadot without downloading/running the explorer: polkadotjs
    #[arg(alias = "skip-pjs", long = "skip-polkadotjs", default_value = "false")]
    pub skip_polkadotjs: bool,

    /// Run local-polkadot with only polkadot-js explorer and not the polkadot node.
    #[arg(
        alias = "explorer",
        alias = "pjs",
        long = "polkadotjs",
        default_value = "false",
        conflicts_with = "skip_polkadotjs"
    )]
    pub only_polkadotjs: bool,
}
