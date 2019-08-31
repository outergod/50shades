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
use crate::config::Config;
use crate::datetime;
use crate::password;
use crate::query;
use crate::template;
use failure::Error;
use std::collections::HashMap;

pub fn run(
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
    let builder = query::node_client(&node, &password::get(&node_name, &node.user)?)?;

    let from = datetime::parse_timestamp(&from)?.0;
    let to = datetime::parse_timestamp(&to)?.1;

    let mut params = HashMap::new();
    query::assign(&query, &mut params);

    params.insert("limit", "0".into());
    params.insert("from", from);
    params.insert("to", to);

    query::run(&builder, &params, &handlebars)?;

    Ok(())
}
