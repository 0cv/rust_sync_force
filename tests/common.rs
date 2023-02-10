use anyhow::Result;
use rust_sync_force::{Client, response::CompositeResponse};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, env};

struct Credentials {
    client_id: String,
    client_secret: String,
    username: String,
    password: String,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct Account {
    #[serde(rename = "attributes")]
    pub attributes: Attribute,
    #[serde(rename = "ExKey__c")]
    pub exkey: Option<String>,
    pub id: Option<String>,
    pub name: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Attribute {
    pub url: Option<String>,
    #[serde(rename = "type")]
    pub sobject_type: String,
}

fn get_credentials() -> Result<Credentials> {
    Ok(Credentials {
        client_id: env::var("SFDC_CLIENT_ID")?,
        client_secret: env::var("SFDC_CLIENT_SECRET")?,
        username: env::var("SFDC_USERNAME")?,
        password: env::var("SFDC_PASSWORD")?,
    })
}

pub fn get_client() -> Result<Client> {
    let creds = get_credentials()?;

    let mut client = Client::new(Some(creds.client_id), Some(creds.client_secret));
    client.login_with_credential(creds.username, creds.password)?;

    Ok(client)
}

pub fn insert_account(client: &Client, name: &str) -> Result<String> {
    let mut params = HashMap::new();
    params.insert("Name", name);

    let res = client.insert("Account", params)?;

    Ok(res.id)
}

pub fn insert_accounts(client: &Client, names: Vec<String>) -> Result<Vec<CompositeResponse>> {
    let accounts = names.into_iter().map(|name| {
        Account {
            exkey: None,
            id: None,
            name: name.into(),
            attributes: Attribute { sobject_type: "Account".into(), url: None },
        }
    }).collect();

    let res = client
        .inserts( true, accounts)?;

    let vec_result: Result<Vec<CompositeResponse>, rust_sync_force::Error> = res.into_iter().collect();

    Ok(vec_result?)
}

pub fn update_accounts(client: &Client, vals: Vec<(String, String)>) -> Result<Vec<CompositeResponse>> {
    let accounts = vals.into_iter().map(|val| {
        Account {
            exkey: None,
            id: val.0.into(),
            name: val.1.into(),
            attributes: Attribute { sobject_type: "Account".into(), url: None },
        }
    }).collect();

    let res = client
        .updates( true, accounts)?;

    let vec_result: Result<Vec<CompositeResponse>, rust_sync_force::Error> = res.into_iter().collect();

    Ok(vec_result?)
}

pub fn upsert_accounts(client: &Client, vals: Vec<(String, String)>) -> Result<Vec<CompositeResponse>> {
    let accounts = vals.into_iter().map(|val| {
        Account {
            exkey: val.0.into(),
            id: None,
            name: val.1.into(),
            attributes: Attribute { sobject_type: "Account".into(), url: None },
        }
    }).collect();

    let res = client
        .upserts( true, "Account", "ExKey__c", accounts)?;
    
    let vec_result: Result<Vec<CompositeResponse>, rust_sync_force::Error> = res.into_iter().collect();

    Ok(vec_result?)
}

pub fn delete_account(client: &Client, id: &str) -> Result<()> {
    client.delete("Account", &id)?;
    Ok(())
}

pub fn delete_accounts(client: &Client, ids: Vec<String>) -> Result<Vec<CompositeResponse>> {
    let res = client
        .deletes( true, ids)?;

    let vec_result: Result<Vec<CompositeResponse>, rust_sync_force::Error> = res.into_iter().collect();

    Ok(vec_result?)
}

pub fn find_account(client: &Client, id: &str) -> Result<Account> {
    let res: Account = client.find_by_id("Account", id)?;
    Ok(res)
}

pub fn clean_records(client: &Client, records: Vec<CompositeResponse>) -> Result<Vec<CompositeResponse>> {
    let records_len = records.len();
    let account_ids = records
        .into_iter()
        .map(|record| record.id.unwrap())
        .collect();

    let deleted_records = delete_accounts(&client, account_ids)?;
    
    // all successfully deleted
    deleted_records.iter().for_each(|record| assert_eq!(true, record.success));
    assert_eq!(records_len, deleted_records.len());

    Ok(deleted_records)
}