use rust_sync_force::stream::{CometdClient, StreamResponse};
use rust_sync_force::{Client, Error};
use serde::Deserialize;
use std::collections::HashMap;
use std::env;

#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
pub struct SFChangeEventHeader {
    pub commitNumber: usize,
    pub commitUser: String,
    pub sequenceNumber: usize,
    pub entityName: String,
    pub changeType: String,
    pub commitTimestamp: usize,
    pub recordIds: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
pub struct SFPayload {
    pub LastModifiedDate: Option<String>,
    pub ChangeEventHeader: SFChangeEventHeader,
}

pub fn listen_sf(mut client: CometdClient) {
    println!("Listen SF loop started");
    loop {
        let responses = client.connect();

        match responses {
            Ok(responses) => {
                for response in responses {
                    if let StreamResponse::Delivery(resp) = response {
                        match serde_json::from_value::<SFPayload>(resp.data.payload.clone()) {
                            Ok(data) => {
                                println!("Data: {:#?}", data);
                                // Here you should have your patterns matching your own objects
                            }
                            Err(err) => {
                                println!(
                                    "SF delivery data could not be parsed: {:?}\nData:{:?}",
                                    err, resp
                                )
                            }
                        }
                    }
                }
            }
            Err(err) => println!("{}", err.to_string()),
        }
    }
}

fn main() -> Result<(), Error> {
    let client_id = env::var("SFDC_CLIENT_ID").unwrap();
    let client_secret = env::var("SFDC_CLIENT_SECRET").unwrap();
    let username = env::var("SFDC_USERNAME").unwrap();
    let password = env::var("SFDC_PASSWORD").unwrap();

    let mut client = Client::new(Some(client_id), Some(client_secret));
    client.login_with_credential(username, password)?;

    let mut stream_client = rust_sync_force::stream::CometdClient::new(
        client,
        HashMap::from([("/data/AccountChangeEvent".to_string(), -1)]),
    );

    stream_client.init().expect("Could not init cometd client");

    println!("Cometd client successfully initialized");

    listen_sf(stream_client);

    Ok(())
}
