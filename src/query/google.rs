// This file is part of 50shades.
//
// Copyright 2021 Alexander Dorn
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

use chrono::{DateTime, NaiveDateTime, Utc};
use failure::Error;
use googapis::{
    google::{
        self,
        logging::v2::{
            logging_service_v2_client::LoggingServiceV2Client, ListLogEntriesRequest,
            ListLogEntriesResponse,
        },
    },
    CERTIFICATES,
};
use handlebars::Handlebars;
use serde::Serialize;
use std::{collections::HashMap, convert::TryInto, time::Duration};
use tonic::{
    metadata::MetadataValue,
    transport::{Certificate, Channel, ClientTlsConfig},
    Request, Response,
};

const ENDPOINT: &str = "https://logging.googleapis.com";
const DOMAIN: &str = "logging.googleapis.com";
const SCOPES: [&str; 1] = ["https://www.googleapis.com/auth/logging.read"];

#[derive(Serialize)]
pub struct MonitoredResource {
    pub r#type: String,
    pub labels: HashMap<String, String>,
}

impl From<google::api::MonitoredResource> for MonitoredResource {
    fn from(resource: google::api::MonitoredResource) -> Self {
        Self {
            r#type: resource.r#type,
            labels: resource.labels,
        }
    }
}

#[derive(Serialize)]
pub struct HttpRequest {
    pub request_method: String,
    pub request_url: String,
    pub request_size: i64,
    pub status: i32,
    pub response_size: i64,
    pub user_agent: String,
    pub remote_ip: String,
    pub server_ip: String,
    pub referer: String,
    pub latency: Option<Duration>,
    pub cache_lookup: bool,
    pub cache_hit: bool,
    pub cache_validated_with_origin_server: bool,
    pub cache_fill_bytes: i64,
    pub protocol: String,
}

impl From<google::logging::r#type::HttpRequest> for HttpRequest {
    fn from(request: google::logging::r#type::HttpRequest) -> Self {
        Self {
            request_method: request.request_method,
            request_url: request.request_url,
            request_size: request.request_size,
            status: request.status,
            response_size: request.response_size,
            user_agent: request.user_agent,
            remote_ip: request.remote_ip,
            server_ip: request.server_ip,
            referer: request.referer,
            latency: request.latency.map(|duration| {
                Duration::new(
                    duration.seconds.try_into().unwrap(),
                    duration.nanos.try_into().unwrap(),
                )
            }),
            cache_lookup: request.cache_lookup,
            cache_hit: request.cache_hit,
            cache_validated_with_origin_server: request.cache_validated_with_origin_server,
            cache_fill_bytes: request.cache_fill_bytes,
            protocol: request.protocol,
        }
    }
}

#[derive(Serialize)]
pub struct LogEntryOperation {
    pub id: String,
    pub producer: String,
    pub first: bool,
    pub last: bool,
}

impl From<google::logging::v2::LogEntryOperation> for LogEntryOperation {
    fn from(operation: google::logging::v2::LogEntryOperation) -> Self {
        Self {
            id: operation.id,
            producer: operation.producer,
            first: operation.first,
            last: operation.last,
        }
    }
}

#[derive(Serialize)]
pub struct LogEntrySourceLocation {
    pub file: String,
    pub line: i64,
    pub function: String,
}

impl From<google::logging::v2::LogEntrySourceLocation> for LogEntrySourceLocation {
    fn from(location: google::logging::v2::LogEntrySourceLocation) -> Self {
        Self {
            file: location.file,
            line: location.line,
            function: location.function,
        }
    }
}

#[derive(Serialize)]
pub struct LogEntry {
    pub log_name: String,
    pub resource: Option<MonitoredResource>,
    pub timestamp: Option<DateTime<Utc>>,
    pub receive_timestamp: Option<DateTime<Utc>>,
    pub severity: i32,
    pub insert_id: String,
    pub http_request: Option<HttpRequest>,
    pub labels: HashMap<String, String>,
    pub operation: Option<LogEntryOperation>,
    pub trace: String,
    pub span_id: String,
    pub trace_sampled: bool,
    pub source_location: Option<LogEntrySourceLocation>,
    pub payload: Option<String>,
}

impl From<google::logging::v2::LogEntry> for LogEntry {
    fn from(entry: google::logging::v2::LogEntry) -> Self {
        Self {
            log_name: entry.log_name,
            resource: entry.resource.map(|resource| resource.into()),
            timestamp: entry.timestamp.map(|timestamp| {
                let dt = NaiveDateTime::from_timestamp(
                    timestamp.seconds,
                    timestamp.nanos.try_into().unwrap(),
                );
                DateTime::from_utc(dt, Utc)
            }),
            receive_timestamp: entry.receive_timestamp.map(|timestamp| {
                let dt = NaiveDateTime::from_timestamp(
                    timestamp.seconds,
                    timestamp.nanos.try_into().unwrap(),
                );
                DateTime::from_utc(dt, Utc)
            }),
            severity: entry.severity,
            insert_id: entry.insert_id,
            http_request: entry.http_request.map(|request| request.into()),
            labels: entry.labels,
            operation: entry.operation.map(|operation| operation.into()),
            trace: entry.trace,
            span_id: entry.span_id,
            trace_sampled: entry.trace_sampled,
            source_location: entry.source_location.map(|location| location.into()),
            payload: Some("".into()),
        }
    }
}

pub async fn client() -> Result<LoggingServiceV2Client<Channel>, Error> {
    let authentication_manager = gcp_auth::init().await?;
    let token = authentication_manager.get_token(&SCOPES).await?;

    let bearer_token = format!("Bearer {}", token.as_str());
    let header_value = MetadataValue::from_str(&bearer_token)?;

    let tls_config = ClientTlsConfig::new()
        .ca_certificate(Certificate::from_pem(CERTIFICATES))
        .domain_name(DOMAIN);

    let channel = Channel::from_static(ENDPOINT)
        .tls_config(tls_config)?
        .connect()
        .await?;

    Ok(LoggingServiceV2Client::with_interceptor(
        channel,
        move |mut req: Request<()>| {
            req.metadata_mut()
                .insert("authorization", header_value.clone());
            Ok(req)
        },
    ))
}

fn handle_response(response: Response<ListLogEntriesResponse>, handlebars: &Handlebars) {
    for entry in response.into_inner().entries.iter() {
        match crate::template::render(handlebars, &LogEntry::from(entry.clone())) {
            Ok(s) => println!("{}", &s),
            Err(e) => eprintln!("Could not format line: {:?}", e),
        }
    }
}

pub async fn run(handlebars: &Handlebars) -> Result<(), Error> {
    let mut client = client().await?;
    let query = client.list_log_entries(Request::new(ListLogEntriesRequest {
        resource_names: (["projects/solvemate-prod".into()]).to_vec(),
        ..Default::default()
    }));

    let response = match query.await {
        Ok(response) => response,
        Err(e) => return Err(e.into()),
    };

    // println!("RESPONSE={:?}", response);
    handle_response(response, handlebars);

    Ok(())
}
