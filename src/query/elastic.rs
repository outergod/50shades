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
use crate::config::ElasticNode;
use crate::password;
use crate::template;
use failure::Error;
use handlebars::Handlebars;
use reqwest;
use reqwest::header::ACCEPT;
use reqwest::{Client, RequestBuilder};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use url::Url;

#[derive(Serialize, Debug, Default)]
pub struct Range {
    pub gt: Option<String>,
    pub gte: Option<String>,
    pub lt: Option<String>,
    pub lte: Option<String>,
}

type Bool = Option<Vec<Box<Query>>>;

#[derive(Serialize, Debug, Default)]
pub struct QueryBool {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub must: Bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub should: Bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub must_not: Bool,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Query {
    SimpleQueryString {
        query: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        fields: Option<Vec<String>>,
    },
    QueryString {
        query: String,
    },
    Range(HashMap<String, Range>),
    Bool(QueryBool),
}

#[derive(Serialize, Debug)]
pub struct Request {
    pub size: Option<u32>,
    pub sort: HashMap<String, String>,
    pub query: Query,
}

#[derive(Deserialize, Debug)]
struct Hit {
    _index: String,
    _type: String,
    _id: String,
    _score: Option<f32>,
    _source: HashMap<String, Value>,
    sort: Vec<u64>,
}

#[derive(Deserialize, Debug)]
struct Total {
    value: u32,
    relation: String,
}

#[derive(Deserialize, Debug)]
struct Hits {
    total: Total,
    max_score: Option<f32>,
    hits: Vec<Hit>,
}

#[derive(Deserialize, Debug)]
struct Shards {
    total: u32,
    successful: u32,
    skipped: u32,
    failed: u32,
}

#[derive(Deserialize, Debug)]
struct Response {
    took: u32,
    timed_out: bool,
    _shards: Shards,
    hits: Hits,
}

#[derive(Deserialize, Debug)]
struct Cause {
    root_cause: Option<Vec<Cause>>,
    r#type: String,
    reason: String,
    line: u32,
    col: u32,
}

#[derive(Deserialize, Debug)]
struct ErrorResponse {
    error: Cause,
    status: u32,
}

pub fn node_client(node: &ElasticNode, name: &str) -> Result<RequestBuilder, Error> {
    let mut url = Url::parse(&node.url)?;

    match url.path_segments_mut() {
        Ok(mut path) => {
            path.extend(&["_search"]);
        }
        Err(()) => return Err(BaseUrlError.into()),
    }

    let client = Client::new()
        .post(url.as_str())
        .header(ACCEPT, "application/json");

    if let Some(ref user) = node.user {
        let password = password::get(name, user)?;
        Ok(client.basic_auth(user.clone(), Some(password)))
    } else {
        Ok(client)
    }
}

fn handle_response(response: Response, handlebars: &Handlebars) {
    for hit in response.hits.hits.iter() {
        match template::render(handlebars, &hit._source) {
            Ok(s) => println!("{}", &s),
            Err(e) => eprintln!("Could not format line: {:?}", e),
        }
    }
}

pub fn run(
    client: &RequestBuilder,
    request: &Request,
    handlebars: &Handlebars,
) -> Result<(), Error> {
    let client = client.try_clone().unwrap().json(request);
    let response = match search::<Response>(client) {
        Ok(response) => response,
        Err(ResponseError::UnexpectedStatus(status, reason)) => {
            return Err(ResponseError::UnexpectedStatus(
                status,
                serde_json::from_str(&reason)
                    .and_then(|e: ErrorResponse| {
                        Ok(format!("{}: {}", e.error.r#type, e.error.reason))
                    })
                    .unwrap_or_else(|_| String::from("No details given")),
            )
            .into())
        }
        Err(e) => return Err(e.into()),
    };
    handle_response(response, handlebars);
    Ok(())
}
