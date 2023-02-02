extern crate rust_sync_force;

mod common;

use anyhow::Result;
use common::{create_account, delete_account, find_account, get_client, Account};
use rust_sync_force::response::QueryResponse;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn create_find_delete_record() -> Result<()> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    let account_name = format!("Hello Rust {}", nanos);
    let client = get_client()?;
    let id = create_account(&client, &account_name)?;
    assert_ne!(String::new(), id);

    let record = find_account(&client, &id)?;

    assert_eq!(account_name, record.name);
    delete_account(&client, &id)?;

    Ok(())
}

#[test]
fn update_record() -> Result<()> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    let new_account_name = format!("Bye Rust {}", nanos);

    let client = get_client()?;
    let id = create_account(&client, format!("Hello Rust {}", nanos).as_ref())?;

    let mut params = HashMap::new();
    params.insert("Name", &new_account_name);

    client.update("Account", &id, params)?;

    let record = find_account(&client, &id)?;
    assert_eq!(new_account_name, record.name);

    delete_account(&client, &id)?;
    Ok(())
}

#[test]
fn upsert_record() -> Result<()> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    let original_account_name = format!("Hello Rust {}", nanos);
    let new_account_name = format!("Bye Rust {}", nanos);

    let client = get_client()?;
    let id = create_account(&client, &original_account_name)?;

    let mut params = HashMap::new();
    params.insert("Name", &new_account_name);

    client.upsert("Account", "Id", &id, params)?;

    let record = find_account(&client, &id)?;
    assert_eq!(new_account_name, record.name);

    delete_account(&client, &id)?;
    Ok(())
}

#[test]
fn check_versions() -> Result<()> {
    let client = get_client()?;
    let versions = client.versions()?;

    assert_ne!(0, versions.len());
    Ok(())
}

#[test]
fn query_record() -> Result<()> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    let account_name = format!("Hello Rust {}", nanos);

    let client = get_client()?;
    let id = create_account(&client, &account_name)?;

    let query = format!("SELECT ID, NAME FROM ACCOUNT WHERE ID = '{}'", id);
    let query_result: QueryResponse<Account> = client.query(&query)?;

    assert_eq!(account_name, query_result.records[0].name);

    delete_account(&client, &id)?;
    Ok(())
}
