use std::path::PathBuf;

use clap::Parser;

mod dict;
mod dict_combine;
mod jlpt;

#[derive(Parser)]
#[command()]
struct Args {
    #[arg(long)]
    /// Overwrite generated entries file
    overwrite: bool,

    /// Directory containing JMDict, JLPT files, and where to save generated file (default: working directory)
    directory: Option<PathBuf>,
}

fn main() {
    let args = Args::parse();
    dict_combine::run(&args.directory.unwrap_or(".".into()), args.overwrite);
}
