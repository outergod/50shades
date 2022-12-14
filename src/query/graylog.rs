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

use super::{search, BaseUrlError, ResponseError};
use crate::config::GraylogNode;
use crate::password;
use crate::template;
use chrono::prelude::*;
use chrono::Utc;
use failure::Error;
use handlebars::Handlebars;
use reqwest;
use reqwest::blocking::{Client, RequestBuilder};
use reqwest::header::ACCEPT;
use serde::{Deserialize, Serialize};
use serde_json::map::Map;
use serde_json::Value;
use std::collections::HashMap;
use std::hash::BuildHasher;
use url::Url;

#[derive(Serialize, Deserialize, Debug)]
struct Response {
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

pub fn node_client(node: &GraylogNode, name: &str) -> Result<RequestBuilder, Error> {
    let mut url = Url::parse(&node.url)?;

    match url.path_segments_mut() {
        Ok(mut path) => {
            path.extend(&["search", "universal", "absolute"]);
        }
        Err(()) => return Err(BaseUrlError.into()),
    }

    let password = password::get(name, &node.user)?;

    Ok(Client::new()
        .get(url.as_str())
        .basic_auth(node.user.clone(), Some(password))
        .header(ACCEPT, "application/json"))
}

fn handle_response(response: Response, handlebars: &Handlebars) {
    if let Some(mut messages) = response.messages {
        messages.reverse();
        for message in messages.iter() {
            if let Some(Value::Object(m)) = message.get("message") {
                match template::render(handlebars, &m) {
                    Ok(s) => println!("{}", &s),
                    Err(e) => eprintln!("Could not format line: {:?}", e),
                }
            }
        }
    }
}

pub fn run<S: BuildHasher>(
    client: &RequestBuilder,
    query: &HashMap<&str, String, S>,
    handlebars: &Handlebars,
) -> Result<(), Error> {
    let tuples: Vec<(&&str, &String)> = query.iter().collect();
    let client = client.try_clone().unwrap().query(&tuples);
    let response = match search::<Response>(client) {
        Ok(response) => response,
        Err(ResponseError::UnexpectedStatus(status, reason)) => {
            return Err(ResponseError::UnexpectedStatus(
                status,
                serde_json::from_str(&reason)
                    .and_then(|e: ErrorResponse| Ok(e.message))
                    .unwrap_or_else(|_| String::from("No details given")),
            )
            .into())
        }
        Err(e) => return Err(e.into()),
    };
    handle_response(response, handlebars);
    Ok(())
}

pub fn assign_query<S: BuildHasher>(query: &[String], params: &mut HashMap<&str, String, S>) {
    if !query.is_empty() {
        params.insert("query", query.join(" "));
    } else {
        params.insert("query", String::from("*"));
    }
}
