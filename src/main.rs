// This file is part of 50shades.
//
// 50shades is free software: you can redistribute it and/or modify it under the
// terms of the GNU General Public License as published by the Free Software
// Foundation, either version 3 of the License, or (at your option) any later
// version.
//
// 50shades is distributed in the hope that it will be useful, but WITHOUT ANY
// WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A
// PARTICULAR PURPOSE.  See the GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License along with
// this program.  If not, see <https://www.gnu.org/licenses/>.

use chrono::prelude::*;
use exitfailure::ExitFailure;
use failure::{Error, Fail};
use reqwest;
use reqwest::header::ACCEPT;
use reqwest::{Client, RequestBuilder, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::map::Map;
use serde_json::Value;
use std::collections::HashMap;
use std::ops::Sub;
use std::{thread, time};
use structopt::StructOpt;
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

/// 50shades (of Graylog)
#[derive(Debug, StructOpt)]
#[structopt(raw(setting = "structopt::clap::AppSettings::ColoredHelp"))]
struct Cli {
    #[structopt(long, short)]
    host: String,

    #[structopt(long, short)]
    username: String,

    #[structopt(long, short)]
    password: String,

    #[structopt(long = "search-from", short = "@")]
    from: String,

    #[structopt(long = "search-to", short = "#")]
    to: String,

    #[structopt(long, short)]
    limit: Option<u64>,

    #[structopt(long, short)]
    follow: bool,

    #[structopt(long, default_value = "2")]
    latency: i64,

    #[structopt(long, default_value = "1000")]
    poll: u64,

    #[structopt(name = "QUERY")]
    query: Option<String>,
}

#[derive(Debug, Fail)]
#[fail(display = "Not a valid base URL")]
struct BaseUrlError;

#[derive(Debug, Fail)]
enum ResponseError {
    #[fail(display = "Authentication failed")]
    AuthenticationFailure,

    #[fail(display = "{}: {}", _0, _1)]
    Unexpected(StatusCode, String),
}

fn handle_response(response: SearchResponse) {
    if let Some(messages) = response.messages {
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
        StatusCode::UNAUTHORIZED => Err(ResponseError::AuthenticationFailure)?,
        status => Err(ResponseError::Unexpected(
            status,
            serde_json::from_str(&body)
                .and_then(|e: ErrorResponse| Ok(String::from(e.message)))
                .unwrap_or(String::from("No details given")),
        ))?,
    }
}

// fn query(builder: &QueryBuilder, query: &Hashmap) {
//     let tuples: Vec<(&&str, &String)> = query.iter().collect();
//     let client = builder.try_clone().unwrap().query(&tuples);
//     handle_response(search(client)?);
// }

fn main() -> Result<(), ExitFailure> {
    let cli = Cli::from_args();

    let mut url = Url::parse(&cli.host)?;

    match url.path_segments_mut() {
        Ok(mut path) => {
            path.extend(&["search", "universal", "absolute"]);
        }
        Err(()) => Err(BaseUrlError)?,
    }

    let builder = Client::new()
        .get(url.as_str())
        .basic_auth(cli.username, Some(cli.password))
        .header(ACCEPT, "application/json");

    let mut query = HashMap::new();
    query.insert("query", cli.query.unwrap_or(String::from("*")));

    if let Some(limit) = cli.limit {
        query.insert("limit", limit.to_string());
    }

    if cli.follow {
        let mut from = cli.from;
        let sleep = time::Duration::from_millis(cli.poll);

        loop {
            let ref now = Utc::now()
                .sub(chrono::Duration::seconds(cli.latency))
                .to_rfc3339_opts(SecondsFormat::Millis, true);

            query.insert("from", from);
            query.insert("to", String::from(now));

            let tuples: Vec<(&&str, &String)> = query.iter().collect();
            let client = builder.try_clone().unwrap().query(&tuples);
            handle_response(search(client)?);

            from = String::from(now);

            thread::sleep(sleep);
        }
    } else {
        query.insert("from", cli.from);
        query.insert("to", cli.to);

        let tuples: Vec<(&&str, &String)> = query.iter().collect();
        let client = builder.try_clone().unwrap().query(&tuples);
        handle_response(search(client)?);
    }

    Ok(())
}
