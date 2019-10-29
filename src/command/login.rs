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
use crate::config::{Config, ElasticNode, Node};
use crate::password;
use failure::{Error, Fail};

#[derive(Debug, Fail)]
#[fail(display = "No username set for node")]
struct NoUserError;

pub fn run(config: Result<Config, Error>, node: String) -> Result<(), Error> {
    let config = match config {
        Ok(ref config) => config::node(config, &node)?,
        Err(e) => return Err(e),
    };

    let user = match config {
        Node::Graylog(node) => &node.user,
        Node::Elastic(ElasticNode {
            user: Some(user), ..
        }) => &user,
        Node::Elastic(ElasticNode { user: None, .. }) => return Err(NoUserError.into()),
    };

    password::prompt(&node, user)
}
