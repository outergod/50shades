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
use failure::Fail;
use reqwest;
use reqwest::header::ACCEPT;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::map::Map;
use serde_json::Value;
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
                for (key, value) in m {
                    println!("{}: {}", key, value);
                }
            }
        }
    }
}

fn main() -> Result<(), ExitFailure> {
    let cli = Cli::from_args();

    let mut url = Url::parse(&cli.host)?;

    match url.path_segments_mut() {
        Ok(mut path) => {
            path.extend(&["search", "universal", "absolute"]);
        }
        Err(()) => Err(BaseUrlError {})?,
    }

    let client = Client::new();
    let mut response = client
        .get(url.as_str())
        .basic_auth(cli.username, Some(cli.password))
        .header(ACCEPT, "application/json")
        .query(&[
            ("query", cli.query.unwrap_or(String::from("*"))),
            ("from", cli.from),
            ("to", cli.to),
            ("limit", String::from("1")),
        ])
        .send()?;

    let body = response.text()?;

    match response.status() {
        StatusCode::OK => handle_response(serde_json::from_str(&body)?),
        StatusCode::UNAUTHORIZED => Err(ResponseError::AuthenticationFailure)?,
        status => Err(ResponseError::Unexpected(
            status,
            serde_json::from_str(&body)
                .and_then(|e: ErrorResponse| Ok(String::from(e.message)))
                .unwrap_or(String::from("No details given")),
        ))?,
    }

    Ok(())
}
