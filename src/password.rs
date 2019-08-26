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

use failure::{Error, Fail};
use keyring::{Keyring, KeyringError};
use rpassword;
use std::fmt;

#[derive(Debug, Fail)]
#[fail(display = "Could not store password: {}", _0)]
struct PasswordStoreError(String);

#[derive(Debug, Fail)]
#[fail(display = "Could not obtain password: {}", _0)]
struct PasswordFetchError(String);

#[derive(Debug, Fail)]
struct NoPasswordError(String);

impl fmt::Display for NoPasswordError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "No password found for node {}.

Please invoke `50shades --node {} login` to fix this issue.",
            self.0, self.0
        )
    }
}

pub fn get(node: &str, user: &str) -> Result<String, Error> {
    let service = format!("50shades:{}", &node);
    let keyring = Keyring::new(&service, user);

    match keyring.get_password() {
        Ok(password) => Ok(password),
        Err(KeyringError::NoPasswordFound) => Err(NoPasswordError(String::from(node)).into()),
        Err(e) => Err(PasswordFetchError(format!("{}", e)).into()),
    }
}

pub fn set(node: &str, user: &str) -> Result<(), Error> {
    let service = format!("50shades:{}", &node);
    let keyring = Keyring::new(&service, user);

    let password = rpassword::read_password_from_tty(Some(&format!(
        "Please provide the password for {} at {}: ",
        user, &node
    )))?;

    match keyring.set_password(&password) {
        Ok(_) => Ok(()),
        Err(e) => Err(PasswordStoreError(format!("{}", e)).into()),
    }
}
