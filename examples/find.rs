use rust_sync_force::{Client, Error};
use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
#[allow(dead_code)]
struct Account {
    #[serde(rename = "attributes")]
    attributes: Attribute,
    id: String,
    name: String,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct Attribute {
    url: String,
    #[serde(rename = "type")]
    sobject_type: String,
}

fn main() -> Result<(), Error> {
    env_logger::init();
    let client_id = env::var("SFDC_CLIENT_ID").unwrap();
    let client_secret = env::var("SFDC_CLIENT_SECRET").unwrap();
    let username = env::var("SFDC_USERNAME").unwrap();
    let password = env::var("SFDC_PASSWORD").unwrap();

    let mut client = Client::new(Some(client_id), Some(client_secret));
    client.login_with_credential(username, password)?;

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    let account_name = format!("Hello Rust {}", nanos);

    let mut params = HashMap::new();
    params.insert("Name", account_name);
    let acc = client.insert("Account", params)?;

    let res: Account = client.find_by_id("Account", &acc.id)?;
    println!("{:?}", res);

    Ok(())
}
