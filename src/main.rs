mod cli;
mod commands;
mod config;
mod display;
mod error;
mod git;
mod naming;
mod herdr;

use crate::cli::Command as CliCmd;
use crate::display::print_error;
use crate::error::MatError;
use std::env;

fn main() {
    let command = match cli::parse() {
        Ok(cmd) => cmd,
        Err(e) => {
            print_error(&e.to_string());
            std::process::exit(1);
        }
    };

    let result = match command {
        CliCmd::Create {
            task_type,
            task_name,
            source,
            no_worktree,
        } => {
            let r = (|| -> Result<(), MatError> {
                let config = crate::config::Config::load()?;
                let git = crate::git::GitClient::new(crate::git::RealRunner);
                let herdr = crate::herdr::HerdrClient::new(crate::git::RealRunner);
                let app_name = naming::get_app_name();
                let current_dir = env::current_dir()?;
                commands::create::handle_create(
                    &task_type,
                    &task_name,
                    source.as_deref(),
                    no_worktree,
                    &config,
                    &git,
                    &herdr,
                    &app_name,
                    &current_dir,
                )
            })();
            r
        }
        CliCmd::Close { no_merge } => {
            let r = (|| -> Result<(), MatError> {
                let config = crate::config::Config::load()?;
                let git = crate::git::GitClient::new(crate::git::RealRunner);
                let herdr = crate::herdr::HerdrClient::new(crate::git::RealRunner);
                let current_dir = env::current_dir()?;
                commands::close::handle_close(no_merge, &config, &git, &herdr, &current_dir)
            })();
            r
        }
        CliCmd::ConfigList => commands::config::handle_config_list(),
        CliCmd::ConfigGet { key } => commands::config::handle_config_get(&key),
        CliCmd::ConfigSet {
            key,
            value,
            global,
        } => commands::config::handle_config_set(&key, &value, global),
    };

    if let Err(e) = result {
        print_error(&e.to_string());
        std::process::exit(1);
    }
}


