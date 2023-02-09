extern crate rust_sync_force;

mod common;

use anyhow::Result;
use common::{clean_records, insert_account, insert_accounts, update_accounts, upsert_accounts, delete_account, find_account, get_client, Account};
use rust_sync_force::response::QueryResponse;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn insert_find_delete_record() -> Result<()> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    let account_name = format!("Hello Rust {}", nanos);
    let client = get_client()?;
    let id = insert_account(&client, &account_name)?;
    assert_ne!(String::new(), id);

    let record = find_account(&client, &id)?;

    assert_eq!(account_name, record.name);
    delete_account(&client, &id)?;

    Ok(())
}

#[test]
fn insert_update_delete_multiple_records() -> Result<()> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    let account_names: Vec<String> = (0..3).map(|i| format!("Hello Rust {}-{}", nanos, i)).collect();

    let client = get_client()?;
    let new_records = insert_accounts(&client, account_names.clone())?;

    // all successfully inserted
    new_records.iter().for_each(|record| assert_eq!(true, record.success));
    // 3 records;
    assert_eq!(3, new_records.len());

    let vals = new_records
        .into_iter()
        .map(|new_record| (new_record.id.unwrap(), format!("Hello Rust {}-new_name", nanos)))
        .collect();

    let updated_records = update_accounts(&client, vals)?;

    // all successfully updated
    updated_records.iter().for_each(|record| assert_eq!(true, record.success));
    // 3 records;
    assert_eq!(3, updated_records.len());

    clean_records(&client, updated_records)?;

    Ok(())
}

#[test]
fn upsert_multiple_records() -> Result<()> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    let vals = (0..3)
        .into_iter()
        .map(|i| (
            format!("ext_id_{}_{}", nanos, i), 
            format!("Hello Rust {}", nanos)
        ))
        .collect();

    let client = get_client()?;
    let new_records = upsert_accounts(&client, vals)?;

    // all successfully upserted
    new_records.iter().for_each(|record| assert_eq!(true, record.success));
    // 3 records;
    assert_eq!(3, new_records.len());

    clean_records(&client, new_records)?;

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
    let id = insert_account(&client, format!("Hello Rust {}", nanos).as_ref())?;

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
    let id = insert_account(&client, &original_account_name)?;

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
    let id = insert_account(&client, &account_name)?;

    let query = format!("SELECT ID, NAME FROM ACCOUNT WHERE ID = '{}'", id);
    let query_result: QueryResponse<Account> = client.query(&query)?;

    assert_eq!(account_name, query_result.records[0].name);

    delete_account(&client, &id)?;
    Ok(())
}
