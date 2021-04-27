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
use crate::config::{Config, ElasticNode, GoogleNode, GraylogNode, NoConfigError, Node};
use crate::password;
use dialoguer::{Input, PasswordInput, Select};
use failure::{Error, Fail};
use url::Url;

#[derive(Debug, Fail)]
#[fail(display = "Config file does already exist. Not overwriting.")]
struct ConfigFileExistsError;

fn prompt_graylog(node: &str) -> Node {
    println!(
        "Please enter the Graylog connection details for the node {}.
Graylog's API endpoint is usually exposed as /api, e.g. https://graylog.example.com/api.
",
        node
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

    Node::Graylog(GraylogNode {
        user,
        url: url.to_string(),
    })
}

fn prompt_elastic(node: &str) -> Node {
    println!(
        "Please enter the Elasticsearch connection details for the node {}.",
        node
    );

    let url: Url;

    loop {
        if let Ok(s) = Input::<String>::new()
            .with_prompt("Elasticsearch URL")
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

    let user: Option<String>;

    loop {
        if let Ok(s) = Input::<String>::new()
            .with_prompt("Username (leave empty for no authentication)")
            .interact()
        {
            user = if !s.is_empty() { Some(s) } else { None };
            break;
        }
    }

    Node::Elastic(ElasticNode {
        user,
        url: url.to_string(),
    })
}

struct UserPass {
    user: String,
    password: String,
}

fn prompt_password() -> String {
    loop {
        if let Ok(s) = PasswordInput::new()
            .with_prompt("Password (not echoed)")
            .interact()
        {
            return s;
        }
    }
}

fn prompt(path: &str, node_name: &str) -> Result<(), Error> {
    println!("We'll set up a new configuration file at {}.", path);

    let node: Node;

    let selections = &["Graylog", "Elasticsearch", "Google"];

    loop {
        if let Ok(n) = Select::new()
            .with_prompt("Please select which node type you'd like to set up")
            .default(0)
            .items(&selections[..])
            .interact()
        {
            match selections[n] {
                "Graylog" => node = prompt_graylog(node_name),
                "Elasticsearch" => node = prompt_elastic(node_name),
                "Google" => node = Node::Google(GoogleNode { resources: vec![] }),
                &_ => panic!(),
            }

            break;
        }
    }

    let user_pass = match node {
        Node::Graylog(GraylogNode { ref user, .. }) => Some(UserPass {
            user: user.clone(),
            password: prompt_password(),
        }),
        Node::Elastic(ElasticNode {
            user: Some(ref user),
            ..
        }) => Some(UserPass {
            user: user.clone(),
            password: prompt_password(),
        }),
        Node::Elastic(ElasticNode { user: None, .. }) => None,
        Node::Google(_) => None,
    };

    let config = Config {
        nodes: vec![(node_name.to_owned(), node)].into_iter().collect(),
        templates: config::Templates::default(),
    };

    println!("Storing configuration...");
    config::write(&path, &config)?;

    if let Some(UserPass { user, password }) = user_pass {
        println!("Storing password in your keyring...");
        password::set(node_name, &user, &password)?;
    }

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
