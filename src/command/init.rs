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
use crate::config::{Config, NoConfigError, Node};
use crate::password;
use dialoguer::{Input, PasswordInput};
use failure::{Error, Fail};
use url::Url;

#[derive(Debug, Fail)]
#[fail(display = "Config file does already exist. Not overwriting.")]
struct ConfigFileExistsError;

fn prompt(path: &str, node: &str) -> Result<(), Error> {
    println!(
        "We'll set up a new configuration file at {}.
Please enter the Graylog connection details for the node {}.
Graylog's API endpoint is usually exposed as /api, e.g. https://graylog.example.com/api.
",
        path, node
    );

    let url: Url;

    loop {
        if let Ok(s) = Input::<String>::new()
            .with_prompt("Graylog API URL")
            .interact()
        {
            match Url::parse(&s) {
                Ok(u) => {
                    url = u;
                    break;
                }
                Err(_) => println!("Not a valid URL."),
            }
        }
    }

    let user: String;

    loop {
        if let Ok(s) = Input::<String>::new().with_prompt("Username").interact() {
            user = s;
            break;
        }
    }

    let password: String;

    loop {
        if let Ok(s) = PasswordInput::new()
            .with_prompt("Password (not echoed)")
            .interact()
        {
            password = s;
            break;
        }
    }

    let config = Config {
        nodes: vec![(
            node.to_owned(),
            Node {
                url: url.into_string(),
                user: user.clone(),
            },
        )]
        .into_iter()
        .collect(),
    };

    println!("Storing configuration...");
    config::write(&path, &config)?;
    println!("Storing password in your keyring...");
    password::set(node, &user, &password)?;
    println!("Done. You should now be able to use 50shades. 
Please edit {} to add more nodes and invoke 50shades with the `login` command to store the corresponding passwords.", &path);

    Ok(())
}

pub fn run(config: Result<Config, Error>, node: String) -> Result<(), Error> {
    match config {
        Ok(_) => Err(ConfigFileExistsError.into()),
        Err(e) => match e.downcast::<NoConfigError>() {
            Ok(e) => {
                prompt(&e.0, &node)?;
                Ok(())
            }
            Err(e) => Err(e),
        },
    }
}
