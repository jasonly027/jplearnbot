use clap::Parser;
use jplearnbot::dict_combine;

#[derive(Parser)]
#[command()]
struct Args {
    #[arg(long)]
    /// Overwite already downloaded JMDict file
    overwrite: bool,
}

fn main() {
    let args = Args::parse();
    dict_combine::run(args.overwrite);
}
