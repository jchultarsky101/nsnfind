use std::process::ExitCode;

mod cli;
mod client;
mod config;
mod error;
mod nsn;
mod output;
mod soap;

#[tokio::main]
async fn main() -> ExitCode {
    match cli::run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err:#}");
            ExitCode::FAILURE
        }
    }
}
