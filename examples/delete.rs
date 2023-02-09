use rust_sync_force::{Client, Error};
use std::collections::HashMap;
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};

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

    let mut params = HashMap::new();
    params.insert("Name", account_name);

    let res = client.insert("Account", params)?;
    println!("Account inserted {:?}", res);

    let res = client.delete("Account", &res.id)?;
    println!("Account deleted {:?}", res);

    Ok(())
}
