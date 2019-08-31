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

use chrono::prelude::*;
use chrono::{Local, TimeZone, Utc};
use failure::{Error, Fail};

#[derive(Debug, Fail)]
#[fail(display = "Could not interpret timestamp {}: {}", timestamp, message)]
pub struct DateParseError {
    timestamp: String,
    message: String,
}

#[derive(Debug, Fail)]
#[fail(display = "Could not determine local timezone")]
pub struct LocalTimeZoneError;

fn convert_datetime(datetime: NaiveDateTime) -> Result<String, LocalTimeZoneError> {
    match Local::now()
        .timezone()
        .from_local_datetime(&datetime)
        .single()
    {
        None => Err(LocalTimeZoneError),
        Some(t) => Ok(t
            .with_timezone(&Utc)
            .format("%Y-%m-%dT%H:%M:%S%.3fZ")
            .to_string()),
    }
}

pub fn parse_timestamp(timestamp: &str) -> Result<(String, String), Error> {
    match two_timer::parse(timestamp, None) {
        Ok((from, to, _)) => Ok((convert_datetime(from)?, convert_datetime(to)?)),
        Err(e) => Err(DateParseError {
            timestamp: timestamp.into(),
            message: e.msg().into(),
        }
        .into()),
    }
}
