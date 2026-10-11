use std::path::PathBuf;

use clap::Parser;

#[derive(Debug, Parser)]
#[command(name = "fsa", version, arg_required_else_help = true)]
pub struct Cli {
    #[arg(value_name = "INPUT")]
    pub input: PathBuf,

    #[arg(short, long, value_name = "OUTPUT")]
    pub output: Option<PathBuf>,
}
