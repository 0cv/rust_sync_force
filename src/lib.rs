//! Crate for interacting with the Salesforce API
//!
//! This crate includes the tools connecting to Salesforce and manipulating
//! Salesforce objects
//!
//! # Example
//!
//! The following example will connect to Salesforce and insert an Account
//! object
//!
//!
//! ```rust,no_run
//! use rust_sync_force::{Client, Error};
//! use serde::Deserialize;
//! use std::collections::HashMap;
//! use std::env;
//!
//! #[derive(Deserialize, Debug)]
//! #[serde(rename_all = "PascalCase")]
//! struct Account {
//!     #[serde(rename = "attributes")]
//!     attributes: Attribute,
//!     id: String,
//!     name: String,
//! }
//!
//! #[derive(Deserialize, Debug)]
//! struct Attribute {
//!     url: String,
//!     #[serde(rename = "type")]
//!     sobject_type: String,
//! }
//!
//! fn main() -> Result<(), Error> {
//!     let client_id = env::var("SFDC_CLIENT_ID").unwrap();
//!     let client_secret = env::var("SFDC_CLIENT_SECRET").unwrap();
//!     let username = env::var("SFDC_USERNAME").unwrap();
//!     let password = env::var("SFDC_PASSWORD").unwrap();
//!
//!     let mut client = Client::new(Some(client_id), Some(client_secret));
//!     client.login_with_credential(username, password)?;

//!     let mut params = HashMap::new();
//!     params.insert("Name", "hello rust");

//!     let res = client.insert("Account", params)?;
//!     println!("{:?}", res);

//!     Ok(())
//! }
//! ```

extern crate thiserror;
extern crate ureq;

pub mod client;
pub mod errors;
pub mod response;
pub mod utils;

pub type Client = client::Client;
pub type Error = errors::Error;
