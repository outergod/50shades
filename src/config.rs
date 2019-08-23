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

use dirs;
use failure::{Error, Fail};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::path::Path;

#[derive(Serialize, Deserialize, Debug)]
pub struct Node {
    pub url: String,
    pub user: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub nodes: HashMap<String, Node>,
}

#[derive(Debug, Fail)]
#[fail(display = "Couldn't load configuration file: {}", _0)]
struct ParseError(String);

#[derive(Debug, Fail)]
#[fail(display = "Node {} is not configured", _0)]
pub struct MissingNodeError(String);

#[derive(Debug, Fail)]
#[fail(display = "Could not determine default configuration path")]
pub struct ConfigPathError;

#[derive(Debug, Fail)]
#[fail(display = "Could not find configuration file at {}", _0)]
pub struct NoConfigError(String);

pub fn default() -> Result<String, ConfigPathError> {
    Ok(dirs::config_dir()
        .and_then(|path| {
            Some(
                Path::new(&path)
                    .join("50shades/config.toml")
                    .to_string_lossy()
                    .into_owned(),
            )
        })
        .ok_or(ConfigPathError)?)
}

pub fn read(path: String) -> Result<Config, Error> {
    let mut file = match File::open(path.clone()) {
        Ok(file) => file,
        Err(ref e) if e.kind() == io::ErrorKind::NotFound => Err(NoConfigError(path))?,
        Err(e) => Err(e)?,
    };

    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    match toml::from_str(&contents) {
        Ok(config) => Ok(config),
        Err(e) => Err(ParseError(e.to_string()))?,
    }
}

pub fn node<'a>(config: &'a Config, name: &str) -> Result<&'a Node, MissingNodeError> {
    Ok(config
        .nodes
        .get(name)
        .ok_or(MissingNodeError(String::from(name)))?)
}
