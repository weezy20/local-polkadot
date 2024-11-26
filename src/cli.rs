#[derive(clap::Parser)]
pub struct Cli {
   /// Path to store artifacts, defaults to `$HOME/.local-polkadot`
   #[arg(long, short)]
   pub path: Option<String>,
   /// Use --tmp if you don't want to store anything locally
   #[arg(long, conflicts_with = "path", default_value = "false")]
   pub tmp: bool,
   /// Cleanup existing artifacts (if present) by removing `$HOME/.local-polkadot`
   #[arg(long, conflicts_with = "tmp", default_value = "false")]
   pub fresh: bool,
}
