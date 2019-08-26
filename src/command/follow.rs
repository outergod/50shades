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
use crate::lib;
use crate::password;
use chrono::prelude::*;
use failure::Error;
use std::collections::HashMap;
use std::ops::Sub;
use std::{thread, time};

pub fn run(
    config: Config,
    name: String,
    from: Option<String>,
    latency: i64,
    poll: u64,
    query: Vec<String>,
) -> Result<(), Error> {
    let node = config::node(&config, &name)?;
    let builder = lib::node_client(&node, &password::get(&name, &node.user)?)?;

    let mut params = HashMap::new();
    let mut _from = from.unwrap_or_else(|| {
        Utc::now()
            .sub(chrono::Duration::seconds(latency))
            .to_rfc3339_opts(SecondsFormat::Millis, true)
    });
    let sleep = time::Duration::from_millis(poll);
    lib::assign_query(&query, &mut params);

    loop {
        let now = &Utc::now()
            .sub(chrono::Duration::seconds(latency))
            .to_rfc3339_opts(SecondsFormat::Millis, true);

        params.insert("from", _from);
        params.insert("to", String::from(now));

        lib::run_query(&builder, &params)?;

        _from = String::from(now);
        thread::sleep(sleep);
    }
}
