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

use crate::config;
use crate::config::{Config, ElasticNode, GoogleNode, GraylogNode, Node};
use crate::datetime;
use crate::query::{elastic, google, graylog};
use crate::template;
use chrono::prelude::*;
use failure::Error;
use googapis::google::logging::v2::TailLogEntriesRequest;
use handlebars::Handlebars;
use maplit::hashmap;
use std::collections::HashMap;
use std::ops::Sub;
use std::{thread, time};

async fn follow_graylog(
    node: &GraylogNode,
    node_name: &str,
    handlebars: &Handlebars,
    from: &str,
    latency: i64,
    poll: u64,
    query: &[String],
) -> Result<(), Error> {
    let client = graylog::node_client(&node, node_name)?;

    let mut params = HashMap::new();
    let mut from = datetime::parse_timestamp(&from)?.0;
    let sleep = time::Duration::from_millis(poll);
    graylog::assign_query(&query, &mut params);

    loop {
        let now = &Utc::now()
            .sub(chrono::Duration::seconds(latency))
            .to_rfc3339_opts(SecondsFormat::Millis, true);

        params.insert("limit", "0".into());
        params.insert("from", from);
        params.insert("to", String::from(now));

        graylog::run(&client, &params, &handlebars).await?;

        from = String::from(now);
        thread::sleep(sleep);
    }
}

async fn follow_elastic(
    node: &ElasticNode,
    node_name: &str,
    handlebars: &Handlebars,
    from: &str,
    latency: i64,
    poll: u64,
    query: &[String],
) -> Result<(), Error> {
    let client = elastic::node_client(node, &node_name)?;

    let mut from = datetime::parse_timestamp(&from)?.0;
    let sleep = time::Duration::from_millis(poll);

    loop {
        let now = &Utc::now()
            .sub(chrono::Duration::seconds(latency))
            .to_rfc3339_opts(SecondsFormat::Millis, true);

        let range = elastic::Query::Range(hashmap! {
            "@timestamp".to_owned() => elastic::Range {
                gte: Some(from),
                lt: Some(now.to_string()),
                ..Default::default()
            }
        });

        let request = elastic::Request {
            size: Some(10000),
            sort: hashmap! {
                "@timestamp".to_owned() => "asc".to_owned()
            },
            query: if !query.is_empty() {
                elastic::Query::Bool(elastic::QueryBool {
                    must: Some(vec![
                        Box::new(elastic::Query::QueryString {
                            query: query.join(" "),
                        }),
                        Box::new(range),
                    ]),
                    ..Default::default()
                })
            } else {
                range
            },
        };

        elastic::run(&client, &request, &handlebars).await?;

        from = String::from(now);
        thread::sleep(sleep);
    }
}

async fn follow_google(
    node: &GoogleNode,
    handlebars: &Handlebars,
    from: &str,
    query: &[String],
) -> Result<(), Error> {
    let from = datetime::parse_timestamp(&from)?.0;
    let range = format!(r#"timestamp >= "{}""#, from);
    let query = if query.is_empty() {
        range
    } else {
        format!("{} AND {}", range, query.join(" "))
    };

    let request = TailLogEntriesRequest {
        resource_names: node.resources.clone(),
        filter: query,
        ..Default::default()
    };

    google::follow(request, &handlebars).await?;
    Ok(())
}

pub async fn run(
    config: Result<Config, Error>,
    node_name: String,
    template: String,
    from: String,
    latency: i64,
    poll: u64,
    query: Vec<String>,
) -> Result<(), Error> {
    let (node, template) = match config {
        Ok(ref config) => (
            config::node(config, &node_name)?,
            config::template(config, &template)?,
        ),
        Err(e) => return Err(e),
    };

    let handlebars = template::compile(&template)?;

    match node {
        Node::Graylog(node) => {
            follow_graylog(node, &node_name, &handlebars, &from, latency, poll, &query).await
        }
        Node::Elastic(node) => {
            follow_elastic(node, &node_name, &handlebars, &from, latency, poll, &query).await
        }
        Node::Google(node) => follow_google(node, &handlebars, &from, &query).await,
    }
}
