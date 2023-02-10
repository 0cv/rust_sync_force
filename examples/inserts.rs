use rust_sync_force::{Client, response::CompositeResponse, Error};
use serde::Serialize;
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct Account {
    #[serde(rename = "attributes")]
    attributes: Attribute,
    name: String,
}

#[derive(Serialize, Debug)]
struct Attribute {
    #[serde(rename = "type")]
    sobject_type: String,
}

fn main() -> Result<(), Error> {
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

    let account = Account {
        name: account_name,
        attributes: Attribute { sobject_type: "Account".into() },
    };

    let res = client
        .inserts( true, vec![account])?;

    let vec_result: Result<Vec<CompositeResponse>, rust_sync_force::Error> = res.into_iter().collect();

    println!("Account inserted: {:?}", vec_result?);

    Ok(())
}
