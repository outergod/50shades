// This file is part of 50shades.
//
// Copyright 2019 Communicatio.Systems GmbH
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use exitfailure::ExitFailure;
use structopt::StructOpt;

/// 50shades (of Graylog)
#[derive(Debug, StructOpt)]
#[structopt(raw(setting = "structopt::clap::AppSettings::ColoredHelp"))]
struct Cli {
    /// Node to query
    #[structopt(long, short, default_value = "default")]
    node: String,

    /// Template to use for output
    #[structopt(long, short, default_value = "default")]
    template: String,

    /// Path to custom configuration file
    #[structopt(long, short)]
    config: Option<String>,

    #[structopt(subcommand)]
    command: Command,
}

#[derive(Debug, StructOpt)]
enum Command {
    /// Initializes the configuration file
    #[structopt(name = "init")]
    Init {},

    /// Stores new password for specified node
    #[structopt(name = "login")]
    Login {},

    /// Performs one-time query
    #[structopt(name = "query")]
    Query {
        #[structopt(long = "search-from", short = "@", default_value = "2 minutes ago")]
        from: String,

        #[structopt(long = "search-to", short = "#", default_value = "now")]
        to: String,

        #[structopt(name = "QUERY")]
        query: Vec<String>,
    },

    /// Follows the tail of a query (like tail -f on a log file)
    #[structopt(name = "follow")]
    Follow {
        #[structopt(long = "search-from", short = "@", default_value = "10 seconds ago")]
        from: String,

        #[structopt(long, default_value = "2")]
        latency: i64,

        #[structopt(long, default_value = "1000")]
        poll: u64,

        #[structopt(name = "QUERY")]
        query: Vec<String>,
    },
}

pub mod config;
pub mod datetime;
pub mod password;
pub mod query;
pub mod template;

mod command {
    pub mod follow;
    pub mod init;
    pub mod login;
    pub mod query;
}

#[tokio::main]
async fn main() -> Result<(), ExitFailure> {
    let cli = Cli::from_args();

    let config = match cli.config {
        None => config::default(),
        Some(path) => Ok(path),
    }
    .and_then(config::read);

    match cli.command {
        Command::Init {} => command::init::run(config, cli.node)?,

        Command::Login {} => command::login::run(config, cli.node)?,

        Command::Follow {
            from,
            latency,
            poll,
            query,
        } => {
            command::follow::run(config, cli.node, cli.template, from, latency, poll, query).await?
        }

        Command::Query { from, to, query } => {
            command::query::run(config, cli.node, cli.template, from, to, query).await?
        }
    }

    Ok(())
}
