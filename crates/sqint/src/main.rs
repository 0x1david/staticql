mod analyzer;
mod cli;
mod config;
mod files;
mod handlers;
use clap::Parser;
use cli::{Cli, Commands};
use config::{Config, DEFAULT_CONFIG, DEFAULT_CONFIG_NAME};
use finder::FinderConfig;
use logging::{Logger, always_log, debug};

//TODO: Impl all README features
//TODO: Big Refactor + Tests + Asserts
fn main() {
    let cli = Cli::parse();
    let config = cli.merge_with_config(files::load_config());
    setup_logging(&cli, &config);

    debug!("CLI arguments parsed: {:?}", cli);
    debug!("Configuration loaded successfully");

    match cli.command {
        None => {
            debug!("No explicit command provided, defaulting to check");
            handlers::handle_check(&config.into(), &cli);
        }
        Some(ref comm) => {
            debug!("Processing command: {:?}", comm);
            match comm {
                Commands::Check(_) => {
                    handlers::handle_check(&config.into(), &cli);
                }
                Commands::Init(_) => {
                    handlers::handle_init();
                }
            }
        }
    }

    let exit_code = Logger::exit_code();
    let sql_errors = Logger::get_sql_error_count();
    let sql_total = Logger::get_sql_var_count();

    if sql_total > 0 {
        always_log!(
            "Sqint: {} sql strings checked, {} errors found",
            sql_total,
            sql_errors
        );
    }

    std::process::exit(exit_code);
}

fn setup_logging(cli: &Cli, cfg: &Config) {
    let ll = cli.loglevel.unwrap_or(cfg.loglevel);
    debug!("Logging initialized at level: {:?}", ll);
    Logger::init(ll);
}
