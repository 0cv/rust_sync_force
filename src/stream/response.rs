use serde::Deserialize;

use crate::stream::advice::Advice;

/// This response is the basic reponse for any that does not match the other
/// field of this enum.
#[derive(Deserialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BasicResponse {
    pub channel: String,
    pub successful: bool,
    pub error: Option<String>,
    pub advice: Option<Advice>,
    pub ext: Option<serde_json::Value>,
    pub client_id: Option<String>,
    pub id: Option<String>,
}

/// This response is returned upon a successful handshake request.
#[derive(Deserialize, PartialEq, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct HandshakeResponse {
    pub channel: String,
    pub successful: bool,
    pub version: String,
    pub minimum_version: Option<String>,
    pub client_id: String,
    pub supported_connection_types: Vec<String>,
    pub advice: Option<Advice>,
    pub ext: Option<serde_json::Value>,
    pub id: Option<String>,
    pub auth_successful: Option<bool>,
}

/// Represents an errored response from the cometd server. If an advice is provided,
/// the client might automatically retry the request.
#[derive(Deserialize, PartialEq, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ErroredResponse {
    pub channel: String,
    pub successful: bool,
    pub error: String,
    pub client_id: Option<String>,
    pub subscription: Option<String>,
    pub advice: Option<Advice>,
    pub ext: Option<serde_json::Value>,
    pub id: Option<String>,
}

/// This response is returned upon a successful publish request.
#[derive(Deserialize, PartialEq, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PublishResponse {
    pub channel: String,
    pub client_id: String,
    pub successful: bool,
    pub error: Option<String>,
    pub advice: Option<Advice>,
    pub ext: Option<serde_json::Value>,
    pub data: serde_json::Value,
    pub id: Option<String>,
}

/// This response is returned when a message is send to a channel the client
/// is subscribed to.
#[derive(Deserialize, PartialEq, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DeliveryResponse {
    pub channel: String,
    pub advice: Option<Advice>,
    pub data: Data,
    pub ext: Option<serde_json::Value>,
    pub id: Option<String>,
}

#[derive(Deserialize, PartialEq, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Data {
    pub event: Event,
    pub payload: serde_json::Value,
}

/// This response is returned when a message is send to a channel the client
/// is subscribed to.
#[derive(Deserialize, PartialEq, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub replay_id: i64,
}

/// Represents a response from the cometd server.
#[derive(Deserialize, PartialEq, Debug)]
#[serde(untagged)]
pub enum StreamResponse {
    ErroredResponse(ErroredResponse),
    /// This response is returned upon a successful handshake request.
    Handshake(HandshakeResponse),
    /// This response is returned upon a successful publish request.
    Publish(PublishResponse),
    /// This response is returned when a message is send to a channel the client
    /// is subscribed to.
    Delivery(DeliveryResponse),
    /// This response is the basic reponse for any that does not match the other
    /// field of this enum.
    Basic(BasicResponse),
}

impl StreamResponse {
    /// Returns an [Advice](Advice) if the server returned one.
    pub fn advice(&self) -> Option<Advice> {
        match self {
            StreamResponse::Handshake(resp) => resp.advice.clone(),
            StreamResponse::Publish(resp) => resp.advice.clone(),
            StreamResponse::Delivery(resp) => resp.advice.clone(),
            StreamResponse::Basic(resp) => resp.advice.clone(),
            _ => None,
        }
    }
}
