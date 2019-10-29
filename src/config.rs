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
use std::default::Default;
use std::fs;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::ops::Deref;
use std::path::Path;
use toml;

const DEFAULT_TEMPLATE: &str = r#"[{{default container_name "-"}}] {{message}}"#;

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum Node {
    Graylog(GraylogNode),
    Elastic(ElasticNode),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GraylogNode {
    pub url: String,
    pub user: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ElasticNode {
    pub url: String,
    pub user: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Templates(HashMap<String, String>);

impl Default for Templates {
    fn default() -> Self {
        let mut hm = HashMap::<String, String>::new();
        hm.insert("default".to_owned(), DEFAULT_TEMPLATE.to_owned());
        Self(hm)
    }
}

impl Deref for Templates {
    type Target = HashMap<String, String>;

    fn deref(&self) -> &HashMap<String, String> {
        &self.0
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub nodes: HashMap<String, Node>,
    #[serde(default)]
    pub templates: Templates,
}

#[derive(Debug, Fail)]
#[fail(display = "Couldn't load configuration file: {}", _0)]
struct ParseError(String);

#[derive(Debug, Fail)]
#[fail(display = "Node {} is not configured", _0)]
pub struct MissingNodeError(String);

#[derive(Debug, Fail)]
#[fail(display = "Template {} is not configured", _0)]
pub struct MissingTemplateError(String);

#[derive(Debug, Fail)]
#[fail(display = "Could not determine default configuration path")]
pub struct ConfigPathError;

#[derive(Debug, Fail)]
#[fail(display = "Could not find configuration file at {}", _0)]
pub struct NoConfigError(pub String);

#[derive(Debug, Fail)]
#[fail(display = "Unsupported node type: {}", _0)]
pub struct NodeTypeError(pub String);

pub fn default() -> Result<String, Error> {
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
        Err(ref e) if e.kind() == io::ErrorKind::NotFound => return Err(NoConfigError(path).into()),
        Err(e) => return Err(e.into()),
    };

    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    match toml::from_str(&contents) {
        Ok(config) => Ok(config),
        Err(e) => Err(ParseError(e.to_string()).into()),
    }
}

pub fn node<'a>(config: &'a Config, name: &str) -> Result<&'a Node, MissingNodeError> {
    Ok(config
        .nodes
        .get(name)
        .ok_or_else(|| MissingNodeError(String::from(name)))?)
}

pub fn template<'a>(config: &'a Config, name: &str) -> Result<&'a str, MissingTemplateError> {
    Ok(config
        .templates
        .get(name)
        .ok_or_else(|| MissingTemplateError(String::from(name)))?)
}

pub fn write(path: &str, config: &Config) -> Result<(), Error> {
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(&parent)?;
    }

    let mut file = File::create(path)?;
    file.write_all(toml::to_string_pretty(config)?.as_bytes())?;
    Ok(())
}
