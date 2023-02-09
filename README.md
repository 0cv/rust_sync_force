[![crate-name at crates.io](https://img.shields.io/crates/v/rust_sync_force.svg)](https://crates.io/crates/rust_sync_force)
[![crate-name at docs.rs](https://docs.rs/rust_sync_force/badge.svg)](https://docs.rs/rust_sync_force)
[![Rust](https://github.com/0cv/rust_sync_force/actions/workflows/rust.yml/badge.svg)](https://github.com/0cv/rust_sync_force/actions/workflows/rust.yml)

## Rust Sync Force

Salesforce Client for Rust Sync.

Async version is here https://github.com/tzmfreedom/rustforce

## Usage

```rust
use rust_sync_force::{Client, Error};
use rust_sync_force::response::{QueryResponse, ErrorResponse};
use serde::Deserialize;
use std::env;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct Account {
    #[serde(rename = "attributes")]
    attributes: Attribute,
    id: String,
    name: String,
}

#[derive(Deserialize, Debug)]
struct Attribute {
    url: String,
    #[serde(rename = "type")]
    sobject_type: String,
}

fn main() -> Result<(), Error> {
    
    let client_id = env::var("SFDC_CLIENT_ID").unwrap();
    let client_secret = env::var("SFDC_CLIENT_SECRET").unwrap();
    let username = env::var("SFDC_USERNAME").unwrap();
    let password = env::var("SFDC_PASSWORD").unwrap();

    let mut client = Client::new(client_id, client_secret);
    client.login_with_credential(username, password)?;

    let res: QueryResponse<Account> = client.query("SELECT Id, Name FROM Account WHERE id = '0012K00001drfGYQAY'".to_string())?;
    println!("{:?}", res);

    Ok(())
}
```

### Authentication

Username Password Flow
```rust
let mut client = Client::new(client_id, client_secret);
client.login_with_credential(username, password)?;
```

[WIP]Authorization Code Grant

### Refresh Token

```rust
let r = client.refresh("xxxx")?;
```

### Query Records

```rust
let r: Result<QueryResponse<Account>, Error> = client.query("SELECT Id, Name FROM Account")?;
```

### Query All Records

```rust
let r: Result<QueryResponse<Account>, Error> = client.query_all("SELECT Id, Name FROM Account")?;
```

### Find By Id

```rust
let r: Result<Account, Error> = client.find_by_id("Account", "{sf_id}")?;
```

### Insert a Record

```rust
let mut params = HashMap::new();
params.insert("Name", "hello rust");
let r = client.insert("Account", params)?;
println!("{:?}", r);
```

### Insert multiple Records

```rust
let account1 = Account {
    name: "account name1".into(),
    attributes: Attribute { sobject_type: "Account".into() },
};

let account2 = Account {
    name: "account name2".into(),
    attributes: Attribute { sobject_type: "Account".into() },
};

let r = client.inserts(true, vec![account1, account2])?;
```

### Update a Record

```rust
let r = client.update("Account", "{sobject_id}", params)?;
```

### Update multiple Records

```rust
let account = Account {
    id: "account_id".into(),
    name: "new_name".into(),
    attributes: Attribute { sobject_type: "Account".into() },
};

let r = client.updates(true, vec![account])?;
```

### Upsert a Record

```rust
let r = client.upsert("Account", "external_key_name", "external_key_value", params)?;
```

### Upsert multiple Records

```rust
let account = Account {
    exkey: "external_key_id".into(),
    name: "new_name".into(),
    attributes: Attribute { sobject_type: "Account".into() },
};

let r = client.upserts(true, "Account", "ExKey__c", vec![account])?;
```

### Delete a Record

```rust
let r = client.delete("Account", "{sobject_id}")?;
```

### Delete multiple Records

```rust
let r = client.deletes(true, vec!["account_id".into()])?;
```

### Describe Global

```rust
let r = client.describe_global()?;
```

### Describe SObject

```rust
let r = client.describe("Account")?;
```

### Versions

```rust
let versions = client.versions()?;
```

### Search(SOSL)

```rust
let r = client.search("FIND {Rust}")?;
```
