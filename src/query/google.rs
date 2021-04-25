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

use failure::Error;

use googapis::google::logging::v2::{
    logging_service_v2_client::LoggingServiceV2Client, ListLogEntriesRequest,
};
use googapis::CERTIFICATES;
use handlebars::Handlebars;
use tonic::{
    metadata::MetadataValue,
    transport::{Certificate, Channel, ClientTlsConfig},
    Request,
};

const ENDPOINT: &str = "https://logging.googleapis.com";
const DOMAIN: &str = "logging.googleapis.com";
const SCOPES: [&str; 1] = ["https://www.googleapis.com/auth/logging.read"];

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

    println!("RESPONSE={:?}", response);

    Ok(())
}
