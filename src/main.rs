use clap::Parser;
#[derive(Parser)]
#[command(version, about = "Durable background jobs for local AI agents")]
struct Cli {}
fn main() {
    Cli::parse();
}
