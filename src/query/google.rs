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
use failure::Fail;
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
use prost::{DecodeError, Message};
use serde::Serialize;
use std::{
    collections::{BTreeMap, HashMap},
    convert::TryInto,
    time::Duration,
};
use tonic::{
    metadata::MetadataValue,
    transport::{Certificate, Channel, ClientTlsConfig},
    Request, Response,
};

const ENDPOINT: &str = "https://logging.googleapis.com";
const DOMAIN: &str = "logging.googleapis.com";
const SCOPES: [&str; 1] = ["https://www.googleapis.com/auth/logging.read"];
const AUDIT_TYPE_URL: &str = "type.googleapis.com/google.cloud.audit.AuditLog";

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
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

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
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

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
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

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
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

#[derive(Serialize, Debug)]
#[serde(transparent)]
pub struct Struct {
    fields: BTreeMap<String, Value>,
}

impl From<prost_types::Struct> for Struct {
    fn from(r#struct: prost_types::Struct) -> Self {
        Self {
            fields: r#struct
                .fields
                .iter()
                .map(|(k, v)| (k.clone(), v.clone().into()))
                .collect(),
        }
    }
}

#[derive(Serialize, Debug)]
#[serde(transparent)]
pub struct ListValue {
    values: Vec<Value>,
}

impl From<prost_types::ListValue> for ListValue {
    fn from(value: prost_types::ListValue) -> Self {
        Self {
            values: value
                .values
                .iter()
                .map(|value| value.clone().into())
                .collect(),
        }
    }
}

#[derive(Serialize, Debug)]
#[serde(untagged)]
pub enum Kind {
    NullValue(i32),
    NumberValue(f64),
    StringValue(String),
    BoolValue(bool),
    StructValue(Struct),
    ListValue(ListValue),
}

impl From<prost_types::value::Kind> for Kind {
    fn from(kind: prost_types::value::Kind) -> Self {
        match kind {
            prost_types::value::Kind::NullValue(value) => Kind::NullValue(value),
            prost_types::value::Kind::NumberValue(value) => Kind::NumberValue(value),
            prost_types::value::Kind::StringValue(value) => Kind::StringValue(value),
            prost_types::value::Kind::BoolValue(value) => Kind::BoolValue(value),
            prost_types::value::Kind::StructValue(value) => Kind::StructValue(value.into()),
            prost_types::value::Kind::ListValue(value) => Kind::ListValue(value.into()),
        }
    }
}

#[derive(Serialize, Debug)]
#[serde(transparent)]
pub struct Value {
    kind: Option<Kind>,
}

impl From<prost_types::Value> for Value {
    fn from(value: prost_types::Value) -> Self {
        Self {
            kind: value.kind.map(|kind| kind.into()),
        }
    }
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ResourceLocation {
    pub current_locations: Vec<String>,
    pub original_locations: Vec<String>,
}

impl From<google::cloud::audit::ResourceLocation> for ResourceLocation {
    fn from(location: google::cloud::audit::ResourceLocation) -> Self {
        Self {
            current_locations: location
                .current_locations
                .iter()
                .map(|s| s.clone().into())
                .collect(),
            original_locations: location
                .original_locations
                .iter()
                .map(|s| s.clone().into())
                .collect(),
        }
    }
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub code: i32,
    pub message: String,
}

impl From<google::rpc::Status> for Status {
    fn from(status: google::rpc::Status) -> Self {
        Self {
            code: status.code,
            message: status.message,
        }
    }
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AuthenticationInfo {
    pub principal_email: String,
    pub authority_selector: String,
    pub third_party_principal: Option<Struct>,
    pub service_account_key_name: String,
    pub principal_subject: String,
}

impl From<google::cloud::audit::AuthenticationInfo> for AuthenticationInfo {
    fn from(info: google::cloud::audit::AuthenticationInfo) -> Self {
        Self {
            principal_email: info.principal_email,
            authority_selector: info.authority_selector,
            third_party_principal: info.third_party_principal.map(|principal| principal.into()),
            service_account_key_name: info.service_account_key_name,
            principal_subject: info.principal_subject,
        }
    }
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizationInfo {
    pub resource: String,
    pub permission: String,
    pub granted: bool,
}

impl From<google::cloud::audit::AuthorizationInfo> for AuthorizationInfo {
    fn from(info: google::cloud::audit::AuthorizationInfo) -> Self {
        Self {
            resource: info.resource,
            permission: info.permission,
            granted: info.granted,
        }
    }
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RequestMetadata {
    pub caller_ip: String,
    pub caller_supplied_user_agent: String,
    pub caller_network: String,
}

impl From<google::cloud::audit::RequestMetadata> for RequestMetadata {
    fn from(data: google::cloud::audit::RequestMetadata) -> Self {
        Self {
            caller_ip: data.caller_ip,
            caller_supplied_user_agent: data.caller_supplied_user_agent,
            caller_network: data.caller_network,
        }
    }
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AuditLog {
    pub service_name: String,
    pub method_name: String,
    pub resource_name: String,
    pub resource_location: Option<ResourceLocation>,
    pub resource_original_state: Option<Struct>,
    pub num_response_items: i64,
    pub status: Option<Status>,
    pub authentication_info: Option<AuthenticationInfo>,
    pub authorization_info: Vec<AuthorizationInfo>,
    pub request_metadata: Option<RequestMetadata>,
    pub request: Option<Struct>,
    pub response: Option<Struct>,
    pub metadata: Option<Struct>,
}

impl From<google::cloud::audit::AuditLog> for AuditLog {
    fn from(log: google::cloud::audit::AuditLog) -> Self {
        Self {
            service_name: log.service_name,
            method_name: log.method_name,
            resource_name: log.resource_name,
            resource_location: log.resource_location.map(|location| location.into()),
            resource_original_state: log.resource_original_state.map(|state| state.into()),
            num_response_items: log.num_response_items,
            status: log.status.map(|status| status.into()),
            authentication_info: log.authentication_info.map(|info| info.into()),
            authorization_info: log
                .authorization_info
                .iter()
                .map(|info| info.clone().into())
                .collect(),
            request_metadata: log.request_metadata.map(|data| data.into()),
            request: log.request.map(|request| request.into()),
            response: log.response.map(|response| response.into()),
            metadata: log.metadata.map(|data| data.into()),
        }
    }
}

#[derive(Serialize, Debug)]
#[serde(tag = "@type")]
pub enum ProtoPayload {
    #[serde(rename(serialize = AUDIT_TYPE_URL))]
    AuditLog(AuditLog),
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
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
    pub text_payload: Option<String>,
    pub json_payload: Option<BTreeMap<String, Value>>,
    pub proto_payload: Option<ProtoPayload>,
}

#[derive(Debug, Fail)]
enum DecodePayloadError {
    #[fail(display = "{}", _0)]
    Decode(DecodeError),
    #[fail(display = "Not a supported payload type: {}", type_url)]
    UnsupportedType { type_url: String },
}

fn decode_payload(payload: prost_types::Any) -> Result<ProtoPayload, DecodePayloadError> {
    let value = payload.value.as_slice();
    match payload.type_url.as_str() {
        AUDIT_TYPE_URL => match google::cloud::audit::AuditLog::decode(value) {
            Ok(log) => Ok(ProtoPayload::AuditLog(log.into())),
            Err(e) => Err(DecodePayloadError::Decode(e)),
        },
        url => Err(DecodePayloadError::UnsupportedType {
            type_url: url.to_string(),
        }),
    }
}

impl From<google::logging::v2::LogEntry> for LogEntry {
    fn from(entry: google::logging::v2::LogEntry) -> Self {
        let (text_payload, json_payload, proto_payload) = match entry.payload {
            Some(google::logging::v2::log_entry::Payload::ProtoPayload(payload)) => {
                match decode_payload(payload) {
                    Ok(payload) => (None, None, Some(payload)),
                    Err(e) => (Some(e.to_string()), None, None),
                }
            }
            Some(google::logging::v2::log_entry::Payload::TextPayload(text)) => {
                (Some(text), None, None)
            }
            Some(google::logging::v2::log_entry::Payload::JsonPayload(json)) => {
                let json = json
                    .fields
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone().into()))
                    .collect();
                (None, Some(json), None)
            }
            None => (None, None, None),
        };

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
            text_payload,
            json_payload,
            proto_payload,
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

fn handle_response(
    response: Response<ListLogEntriesResponse>,
    handlebars: &Handlebars,
) -> Option<String> {
    let response = response.into_inner();

    for entry in response.entries.iter() {
        match crate::template::render(handlebars, &LogEntry::from(entry.clone())) {
            Ok(s) => println!("{}", &s),
            Err(e) => eprintln!("Could not format line: {:?}", e),
        }
    }

    if response.next_page_token.is_empty() {
        None
    } else {
        Some(response.next_page_token)
    }
}

pub async fn run(request: ListLogEntriesRequest, handlebars: &Handlebars) -> Result<(), Error> {
    let mut client = client().await?;
    let query = client.list_log_entries(Request::new(request.clone()));

    let response = match query.await {
        Ok(response) => response,
        Err(e) => return Err(e.into()),
    };

    let mut token = handle_response(response, handlebars);

    while let Some(page_token) = token {
        let mut request = request.clone();
        request.page_token = page_token;
        let query = client.list_log_entries(Request::new(request));

        let response = match query.await {
            Ok(response) => response,
            Err(e) => return Err(e.into()),
        };

        token = handle_response(response, handlebars);
    }

    Ok(())
}
