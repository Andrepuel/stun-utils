use clap::Parser;

mod client;
mod server;
mod telnet;

#[derive(clap::Parser)]
pub struct Args {
    #[clap(subcommand)]
    subcommand: Subcommand,
}

#[derive(clap::Subcommand)]
pub enum Subcommand {
    Server(server::Args),
    Client(client::Args),
    Telnet(telnet::Args),
}

fn main() {
    env_logger::init();

    let args = Args::parse();

    match args.subcommand {
        Subcommand::Server(args) => args.main(),
        Subcommand::Client(args) => args.main(),
        Subcommand::Telnet(args) => args.main(),
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Stun(#[from] stun::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    WebrtcUtil(#[from] webrtc_util::Error),
    #[error(transparent)]
    Turn(#[from] turn::Error),
    #[error("Expected \"turn:server username password\"")]
    TurnParse
}
