use rust_sync_force::{Client, Error};
use std::env;

fn main() -> Result<(), Error> {
    let username = env::var("SFDC_USERNAME").unwrap();
    let password = env::var("SFDC_PASSWORD").unwrap();

    let mut client = Client::new(None, None);
    client.login_by_soap(username, password)?;
    Ok(())
}
