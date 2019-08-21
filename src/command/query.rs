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

use crate::lib;
use failure::Error;
use reqwest;
use reqwest::RequestBuilder;
use std::collections::HashMap;

pub fn run(
    builder: RequestBuilder,
    from: String,
    to: String,
    limit: Option<u64>,
    query: Vec<String>,
) -> Result<(), Error> {
    let mut params = HashMap::new();
    lib::assign_query(&query, &mut params);

    if let Some(limit) = limit {
        params.insert("limit", limit.to_string());
    }

    params.insert("from", from);
    params.insert("to", to);

    lib::run_query(&builder, &params)?;

    Ok(())
}
