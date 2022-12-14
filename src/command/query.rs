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
use failure::Error;
use googapis::google::logging::v2::ListLogEntriesRequest;
use handlebars::Handlebars;
use maplit::hashmap;
use std::collections::HashMap;

async fn query_graylog(
    node: &GraylogNode,
    node_name: &str,
    handlebars: &Handlebars,
    from: &str,
    to: &str,
    query: &[String],
) -> Result<(), Error> {
    let client = graylog::node_client(node, node_name)?;

    let from = datetime::parse_timestamp(&from)?.0;
    let to = datetime::parse_timestamp(&to)?.1;

    let mut params = HashMap::new();
    graylog::assign_query(&query, &mut params);

    params.insert("limit", "0".into());
    params.insert("from", from);
    params.insert("to", to);

    graylog::run(&client, &params, &handlebars).await?;

    Ok(())
}

async fn query_elastic(
    node: &ElasticNode,
    node_name: &str,
    handlebars: &Handlebars,
    from: &str,
    to: &str,
    query: &[String],
) -> Result<(), Error> {
    let client = elastic::node_client(node, &node_name)?;

    let from = datetime::parse_timestamp(&from)?.0;
    let to = datetime::parse_timestamp(&to)?.1;

    let range = elastic::Query::Range(hashmap! {
        "@timestamp".to_owned() => elastic::Range {
            gte: Some(from),
            lt: Some(to),
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
    Ok(())
}

async fn query_google(
    node: &GoogleNode,
    handlebars: &Handlebars,
    from: &str,
    to: &str,
    query: &[String],
) -> Result<(), Error> {
    let from = datetime::parse_timestamp(&from)?.0;
    let to = datetime::parse_timestamp(&to)?.1;
    let range = format!(r#"timestamp >= "{}" AND timestamp < "{}""#, from, to);
    let query = if query.is_empty() {
        range
    } else {
        format!("{} AND {}", range, query.join(" "))
    };

    let request = ListLogEntriesRequest {
        resource_names: node.resources.clone(),
        filter: query,
        page_size: 1000,
        ..Default::default()
    };

    google::query(request, &handlebars).await?;
    Ok(())
}

pub async fn run(
    config: Result<Config, Error>,
    node_name: String,
    template: String,
    from: String,
    to: String,
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
            query_graylog(node, &node_name, &handlebars, &from, &to, &query).await
        }
        Node::Elastic(node) => {
            query_elastic(node, &node_name, &handlebars, &from, &to, &query).await
        }
        Node::Google(node) => query_google(node, &handlebars, &from, &to, &query).await,
    }
}
