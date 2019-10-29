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

use failure::Fail;
use reqwest::{RequestBuilder, StatusCode};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

pub mod elastic;
pub mod graylog;

#[derive(Serialize, Deserialize, Debug)]
struct ErrorResponse {
    r#type: String,
    message: String,
}

#[derive(Debug, Fail)]
pub enum ResponseError {
    #[fail(display = "Authentication failed")]
    AuthenticationFailure,

    #[fail(display = "{:?}", _0)]
    RequestError(reqwest::Error),

    #[fail(display = "{:?}", _0)]
    Conversion(serde_json::Error),

    #[fail(display = "{}: {}", _0, _1)]
    UnexpectedStatus(StatusCode, String),
}

impl From<reqwest::Error> for ResponseError {
    fn from(error: reqwest::Error) -> Self {
        ResponseError::RequestError(error)
    }
}

impl From<serde_json::Error> for ResponseError {
    fn from(error: serde_json::Error) -> Self {
        ResponseError::Conversion(error)
    }
}

#[derive(Debug, Fail)]
#[fail(display = "Not a valid base URL")]
pub struct BaseUrlError;

pub fn search<T>(client: RequestBuilder) -> Result<T, ResponseError>
where
    T: DeserializeOwned,
{
    let mut response = client.send()?;
    let body = response.text()?;

    match response.status() {
        StatusCode::OK => Ok(serde_json::from_str::<T>(&body)?),
        StatusCode::UNAUTHORIZED => Err(ResponseError::AuthenticationFailure),
        status => Err(ResponseError::UnexpectedStatus(status, body)),
    }
}
