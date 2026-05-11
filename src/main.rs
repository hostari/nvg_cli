mod api;
mod cli;
mod commands;
mod config;
mod output;

use clap::Parser;
use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    let args = cli::Cli::parse();

    match commands::dispatch(args).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("Error: {err}");
            commands::auth::exit_code_for(&err)
        }
    }
}
