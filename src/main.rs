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

    /// Performs one-time query against Graylog
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
pub mod password;

mod command {
    pub mod follow;
    pub mod init;
    pub mod login;
    pub mod query;
}

fn main() -> Result<(), ExitFailure> {
    let cli = Cli::from_args();

    let config = match cli.config {
        None => config::default(),
        Some(path) => Ok(path),
    }
    .and_then(|path| config::read(path));

    match cli.command {
        Command::Init {} => command::init::run(config, cli.node)?,

        Command::Login {} => command::login::run(config, cli.node)?,

        Command::Follow {
            from,
            latency,
            poll,
            query,
        } => command::follow::run(config, cli.node, from, latency, poll, query)?,

        Command::Query { from, to, query } => {
            command::query::run(config, cli.node, from, to, query)?
        }
    }

    Ok(())
}

pub mod lib {
    use crate::config;
    use chrono::prelude::*;
    use chrono::{Local, TimeZone, Utc};
    use failure::{Error, Fail};
    use reqwest;
    use reqwest::header::ACCEPT;
    use reqwest::Client;
    use reqwest::{RequestBuilder, StatusCode};
    use serde::{Deserialize, Serialize};
    use serde_json::map::Map;
    use serde_json::Value;
    use std::collections::HashMap;
    use std::hash::BuildHasher;
    use two_timer;
    use url::Url;

    #[derive(Serialize, Deserialize, Debug)]
    struct SearchResponse {
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
        messages: Option<Vec<Map<String, Value>>>,
        fields: Option<Vec<String>>,
        time: Option<u64>,
        built_query: Option<String>,
        used_indices: Option<Vec<Map<String, Value>>>,
        total_results: Option<u64>,
        decoration_stats: Option<Map<String, Value>>,
        query: Option<String>,
    }

    #[derive(Serialize, Deserialize, Debug)]
    struct ErrorResponse {
        r#type: String,
        message: String,
    }

    #[derive(Debug, Fail)]
    enum ResponseError {
        #[fail(display = "Authentication failed")]
        AuthenticationFailure,

        #[fail(display = "{}: {}", _0, _1)]
        Unexpected(StatusCode, String),
    }

    #[derive(Debug, Fail)]
    #[fail(display = "Not a valid base URL")]
    struct BaseUrlError;

    #[derive(Debug, Fail)]
    #[fail(display = "Could not interpret timestamp {}: {}", timestamp, message)]
    pub struct DateParseError {
        timestamp: String,
        message: String,
    }

    #[derive(Debug, Fail)]
    #[fail(display = "Could not determine local timezone")]
    pub struct LocalTimeZoneError;

    pub fn node_client(node: &config::Node, password: &str) -> Result<RequestBuilder, Error> {
        let mut url = Url::parse(&node.url)?;

        match url.path_segments_mut() {
            Ok(mut path) => {
                path.extend(&["search", "universal", "absolute"]);
            }
            Err(()) => return Err(BaseUrlError.into()),
        }

        Ok(Client::new()
            .get(url.as_str())
            .basic_auth(node.user.clone(), Some(password))
            .header(ACCEPT, "application/json"))
    }

    fn handle_response(response: SearchResponse) {
        if let Some(mut messages) = response.messages {
            messages.reverse();
            for message in messages.iter() {
                if let Some(Value::Object(m)) = message.get("message") {
                    println!(
                        "[{}] {}",
                        m.get("container_name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("-"),
                        m.get("message").and_then(|v| v.as_str()).unwrap_or("-")
                    );

                    // for (key, value) in m {
                    //     println!("{}: {}", key, value);
                    // }
                }
            }
        }
    }

    fn search(client: RequestBuilder) -> Result<SearchResponse, Error> {
        let mut response = client.send()?;
        let body = response.text()?;

        match response.status() {
            StatusCode::OK => Ok(serde_json::from_str(&body)?),
            StatusCode::UNAUTHORIZED => Err(ResponseError::AuthenticationFailure.into()),
            status => Err(ResponseError::Unexpected(
                status,
                serde_json::from_str(&body)
                    .and_then(|e: ErrorResponse| Ok(e.message))
                    .unwrap_or_else(|_| String::from("No details given")),
            )
            .into()),
        }
    }

    pub fn run_query<S: BuildHasher>(
        builder: &RequestBuilder,
        query: &HashMap<&str, String, S>,
    ) -> Result<(), Error> {
        let tuples: Vec<(&&str, &String)> = query.iter().collect();
        let client = builder.try_clone().unwrap().query(&tuples);
        handle_response(search(client)?);
        Ok(())
    }

    pub fn assign_query<S: BuildHasher>(query: &[String], params: &mut HashMap<&str, String, S>) {
        if !query.is_empty() {
            params.insert("query", query.join(" "));
        } else {
            params.insert("query", String::from("*"));
        }
    }

    fn convert_datetime(datetime: NaiveDateTime) -> Result<String, LocalTimeZoneError> {
        match Local::now()
            .timezone()
            .from_local_datetime(&datetime)
            .single()
        {
            None => Err(LocalTimeZoneError),
            Some(t) => Ok(t
                .with_timezone(&Utc)
                .format("%Y-%m-%dT%H:%M:%S%.3fZ")
                .to_string()),
        }
    }

    pub fn parse_timestamp(timestamp: &str) -> Result<(String, String), Error> {
        match two_timer::parse(timestamp, None) {
            Ok((from, to, _)) => Ok((convert_datetime(from)?, convert_datetime(to)?)),
            Err(e) => Err(DateParseError {
                timestamp: timestamp.into(),
                message: e.msg().into(),
            }
            .into()),
        }
    }
}
