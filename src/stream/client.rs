// use reqwest::{Client as ReqwestClient, Response as ReqwestReponse, Url};
use serde::Serialize;
// use serde_json::json;
// use std::time::Duration;
use ureq::Response;

use crate::client::Client;
use crate::errors::Error;
use crate::stream::advice::{Advice, Reconnect};
use crate::stream::config::{COMETD_SUPPORTED_TYPES, COMETD_VERSION};
use crate::stream::StreamResponse;

use super::response::ErroredResponse;

/// The cometd client.
pub struct CometdClient {
    client: Client,
    stream_client_id: Option<String>,
    max_retries: i8,
    actual_retries: i8,
    subscriptions: Vec<String>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct HandshakePayload<'a> {
    channel: &'a str,
    version: &'a str,
    supported_connection_types: Vec<&'a str>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ConnectPayload<'a> {
    channel: &'a str,
    client_id: &'a str,
    connection_type: &'a str,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct DisconnectPayload<'a> {
    channel: &'a str,
    client_id: &'a str,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct SubscribeTopicPayload<'a> {
    pub channel: &'a str,
    pub client_id: &'a str,
    pub subscription: &'a str,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct PublishPayload<'a, T>
where
    T: Serialize + std::fmt::Debug,
{
    pub channel: &'a str,
    pub client_id: &'a str,
    pub data: T,
}

impl CometdClient {
    /// Creates a new cometd client.
    pub fn new(client: Client, subscriptions: Vec<String>) -> CometdClient {
        CometdClient {
            client,
            stream_client_id: None,
            actual_retries: 0,
            max_retries: 3,
            subscriptions: subscriptions,
        }
    }

    /// Sets the number of retries the client will attempt in case of an error or a retry advice is
    /// returned by the cometd server.
    pub fn set_retries(mut self, retries: i8) -> Self {
        self.max_retries = retries;
        self
    }

    fn send_request(&self, body: &impl Serialize) -> Result<Response, Error> {
        self.client.sfdc_post(
            format!("/cometd/{}", self.client.version.replace("v", "")),
            body,
        )
    }

    fn retry(&mut self) -> Result<Vec<StreamResponse>, Error> {
        self.actual_retries += 1;
        println!("Attempt n°{}", self.actual_retries);

        match &self.stream_client_id {
            Some(stream_client_id) => {
                let response = self.send_request(&ConnectPayload {
                    channel: "/meta/connect",
                    client_id: &stream_client_id,
                    connection_type: "long-polling",
                })?;

                self.handle_response(response)
            }
            None => Err(Error::GenericError(
                "No client id set for connect".to_string(),
            )),
        }
    }

    fn retry_handshake(&mut self) -> Result<Vec<StreamResponse>, Error> {
        self.actual_retries += 1;
        println!("Attempt n°{}", self.actual_retries);

        let response = self.send_request(&HandshakePayload {
            channel: "/meta/handshake",
            version: COMETD_VERSION,
            supported_connection_types: COMETD_SUPPORTED_TYPES.to_vec(),
        })?;

        self.handle_response(response)
    }

    fn handle_advice(
        &mut self,
        advice: &Advice,
        error: Option<&str>,
    ) -> Result<Vec<StreamResponse>, Error> {
        println!("Following advice from server");
        match advice.reconnect {
            Reconnect::Handshake => {
                if self.actual_retries <= self.max_retries {
                    match self.retry_handshake() {
                        Ok(_) => {
                            self.subscribe()?;
                            let responses = self.retry();
                            if responses.is_ok() {
                                self.actual_retries = 0;
                            }
                            responses
                        }
                        Err(err) => Err(err),
                    }
                } else {
                    Err(Error::GenericError(
                        error.unwrap_or("Max retries reached").to_string(),
                    ))
                }
            }
            Reconnect::Retry => {
                if self.actual_retries <= self.max_retries {
                    self.retry()
                } else {
                    Err(Error::GenericError(
                        error.unwrap_or("Max retries reached").to_string(),
                    ))
                }
            }
            Reconnect::None => Err(Error::GenericError(
                error
                    .unwrap_or("Service advised not to reconnect nor handshake")
                    .to_string(),
            )),
        }
    }

    /// Handles the error returned by the cometd server. If possible, it will
    /// automatically retry according to the client configuration. If it still
    /// fails after the retries, the original error will be returned.
    fn handle_error(
        &mut self,
        errored_response: &ErroredResponse,
    ) -> Result<Vec<StreamResponse>, Error> {
        match errored_response.advice {
            Some(ref advice) => self.handle_advice(advice, Some(&errored_response.error)),
            None => Err(Error::GenericError(format!(
                "Not retrying because the server did not provide advice{}",
                &errored_response.error
            ))),
        }
    }

    fn handle_response(&mut self, response: Response) -> Result<Vec<StreamResponse>, Error> {
        match response.into_json::<Vec<StreamResponse>>() {
            Ok(stream_responses) => {
                let mut responses = vec![];
                for stream_response in stream_responses.into_iter() {
                    match stream_response {
                        StreamResponse::ErroredResponse(error_responses) => {
                            let stream_responses = self.handle_error(&error_responses)?;

                            for stream_response in stream_responses.into_iter() {
                                responses.push(stream_response);
                            }
                        }
                        _ => {
                            if let Some(ref advice) = stream_response.advice() {
                                for stream_response in self.handle_advice(advice, None)? {
                                    responses.push(stream_response);
                                }
                            } else {
                                if let StreamResponse::Handshake(ref stream_response) =
                                    stream_response
                                {
                                    self.stream_client_id = Some(stream_response.client_id.clone());
                                }
                                responses.push(stream_response);
                            }
                        }
                    }
                }
                Ok(responses)
            }
            Err(e) => Err(Error::GenericError(format!(
                "Could not parse response: {}",
                e
            ))),
        }
    }

    fn handshake(&mut self) -> Result<Vec<StreamResponse>, Error> {
        let resps = self.retry_handshake();

        self.actual_retries = 0;
        resps
    }

    /// The cometd connect method. It will hang for a response from the server according
    /// to the timeout provided to the cometd client.
    /// If one or several sucess responses are returned to the request, it will return a `Vec`
    /// containing those responses.
    /// If an errored response is received but an advice is provided by the server, the client
    /// will try to follow this advice and re-attemp the connection. If the maximum number of retries
    /// is reached and the response still does not succeed, it will return an error.
    ///
    /// # Errors
    ///
    /// The cometd server's response could not be parsed.
    /// The cometd server returned a response that indicated an error and the request could not be
    /// retried or the maximum number of retries has been reached.
    pub fn connect(&mut self) -> Result<Vec<StreamResponse>, Error> {
        let resps = self.retry();

        self.actual_retries = 0;
        resps
    }

    /// The cometd disconnect method.
    /// If one or several sucess responses are returned to the request, it will return a `Vec`
    /// containing those responses.
    ///
    /// # Errors
    ///
    /// The cometd server's response could not be parsed.
    /// The cometd server returned a response that indicated an error and the request could not be
    /// retried or the maximum number of retries has been reached.
    pub fn disconnect(&mut self) -> Result<Vec<StreamResponse>, Error> {
        match &self.stream_client_id {
            Some(client_id) => {
                let response = self.send_request(&DisconnectPayload {
                    channel: "/meta/disconnect",
                    client_id,
                })?;

                self.handle_response(response)
            }
            None => Err(Error::GenericError(
                "No client id set for disconnect".to_string(),
            )),
        }
    }

    /// Init the cometd client. It will attempt to establish a handshake between
    /// the client and the server so it can make further requests.
    pub fn init(&mut self) -> Result<Vec<StreamResponse>, Error> {
        let stream_responses = self.handshake()?;
        self.subscribe()?;

        Ok(stream_responses)
    }

    /// The cometd subscribe method. It will ask the server to subscribe to a certain channel and therefore
    /// be updated when something is posted on this channel.
    /// If one or several sucess responses are returned to the request, it will return a `Vec`
    /// containing those responses.
    /// If an errored response is received but an advice is provided by the server, the client
    /// will try to follow this advice and re-attemp the connection. If the maximum number of retries
    /// is reached and the response still does not succeed, it will return an error.
    ///
    /// # Errors
    ///
    /// The cometd server's response could not be parsed.
    /// The cometd server returned a response that indicated an error and the request could not be
    /// retried or the maximum number of retries has been reached.
    pub fn subscribe(&mut self) -> Result<(), Error> {
        match self.stream_client_id.clone() {
            Some(client_id) => {
                for subscription in self.subscriptions.clone() {
                    let response = self.send_request(&SubscribeTopicPayload {
                        channel: "/meta/subscribe",
                        client_id: &client_id,
                        subscription: &subscription,
                    })?;

                    self.handle_response(response)?;
                }

                Ok(())
            }
            None => Err(Error::GenericError(
                "No client id set for subscribe".to_string(),
            )),
        }
    }

    /// The cometd subscribe method. It will ask the server to unsubscribe from a certain channel and therefore
    /// strop being updated when something is posted on this channel.
    /// If one or several sucess responses are returned to the request, it will return a `Vec`
    /// containing those responses.
    /// If an errored response is received but an advice is provided by the server, the client
    /// will try to follow this advice and re-attemp the connection. If the maximum number of retries
    /// is reached and the response still does not succeed, it will return an error.
    ///
    /// # Errors
    ///
    /// The cometd server's response could not be parsed.
    /// The cometd server returned a response that indicated an error and the request could not be
    /// retried or the maximum number of retries has been reached.
    pub fn unsubscribe(&mut self, subscription: &str) -> Result<Vec<StreamResponse>, Error> {
        match &self.stream_client_id {
            Some(client_id) => {
                let response = self.send_request(&SubscribeTopicPayload {
                    channel: "/meta/unsubscribe",
                    client_id,
                    subscription,
                })?;

                self.handle_response(response)
            }
            None => Err(Error::GenericError(
                "No client id set for unsubscribe".to_string(),
            )),
        }
    }

    /// The cometd plublish method. It will ask the server to publish a message to a certain channel.
    /// If one or several sucess responses are returned to the request, it will return a `Vec`
    /// containing those responses.
    /// If an errored response is received but an advice is provided by the server, the client
    /// will try to follow this advice and re-attemp the connection. If the maximum number of retries
    /// is reached and the response still does not succeed, it will return an error.
    ///
    /// # Errors
    ///
    /// The cometd server's response could not be parsed.
    /// The cometd server returned a response that indicated an error and the request could not be
    /// retried or the maximum number of retries has been reached.
    pub fn publish(
        &mut self,
        channel: &str,
        data: impl Serialize + std::fmt::Debug,
    ) -> Result<Vec<StreamResponse>, Error> {
        match &self.stream_client_id {
            Some(client_id) => {
                let response = self.send_request(&PublishPayload {
                    channel,
                    client_id,
                    data,
                })?;

                self.handle_response(response)
            }
            None => Err(Error::GenericError(
                "No client id set for unsubscribe".to_string(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use mockito::Server as MockServer;
    use serde_json::json;

    use super::CometdClient;
    use crate::Client;

    static RETRIES_MAX: i8 = 3;

    fn client(server: &MockServer) -> CometdClient {
        let mut client = Client::new(None, None);
        let url = MockServer::url(&server);
        client.set_instance_url(&url);
        client.set_access_token("this_is_access_token");
        CometdClient::new(client, vec![]).set_retries(RETRIES_MAX)
    }

    mod init {
        use super::*;

        #[test]
        fn returns_error_on_failure() {
            let mut server = MockServer::new_with_port(0);
            let _m = server
                .mock("POST", "/cometd/56.0")
                .with_status(200)
                .with_body(
                    json!([{
                        "channel": "/meta/handshake",
                        "error": "406::Unsupported version, or unsupported minimum version",
                        "successful": false
                    }])
                    .to_string(),
                )
                .create();

            let mut client = client(&server);

            assert!(client.init().is_err());
        }

        #[test]
        fn works() {
            let mut server = MockServer::new_with_port(0);
            let _m = server
                .mock("POST", "/cometd/56.0")
                .with_status(200)
                .with_body(
                    json!([{
                        "channel": "/meta/handshake",
                        "version": "1.0",
                        "successful": true,
                        "clientId": "1234",
                        "supportedConnectionTypes": ["long-polling"]
                    }])
                    .to_string(),
                )
                .create();
            let mut client = client(&server);

            assert!(client.init().is_ok());
        }
    }

    mod connect {
        use super::*;

        #[test]
        fn retries_if_server_advises_to() {
            let mut server = MockServer::new_with_port(0);
            let _m = server
                .mock("POST", "/cometd/56.0")
                .with_status(200)
                .match_body(
                    r#"{"channel":"/meta/handshake","version":"1.0","supportedConnectionTypes":["long-polling"]}"#,
                )
                .with_body(
                    json!([{
                        "channel": "/meta/handshake",
                        "version": "1.0",
                        "successful": true,
                        "clientId": "1234",
                        "supportedConnectionTypes": ["long-polling"]
                    }])
                    .to_string(),
                )
                .create();

            let connect_mock = server
                .mock("POST", "/cometd/56.0")
                .with_status(200)
                .match_body(
                    r#"{"channel":"/meta/connect","clientId":"1234","connectionType":"long-polling"}"#,
                )
                .with_body(
                    json!([{
                        "advice":{
                            "reconnect": "retry"
                        },
                        "channel": "/meta/connect",
                        "error": "400::Error",
                        "successful": false
                    }])
                    .to_string(),
                )
                .expect(RETRIES_MAX as usize + 1)
                .create();

            let mut client = client(&server);

            client.init().expect("Could not init client");
            client.connect().expect_err("Connect should not return Ok");
            connect_mock.assert();
        }

        #[test]
        fn handshake_if_advises_to() {
            let mut server = MockServer::new_with_port(0);
            let hs_mock = server
                .mock("POST", "/cometd/56.0")
                .with_status(200)
                .match_body(
                    r#"{"channel":"/meta/handshake","version":"1.0","supportedConnectionTypes":["long-polling"]}"#,
                )
                .with_body(
                    json!([{
                        "channel": "/meta/handshake",
                        "version": "1.0",
                        "successful": true,
                        "clientId": "1234",
                        "supportedConnectionTypes": ["long-polling"]
                    }])
                    .to_string(),
                )
                .expect(RETRIES_MAX as usize) // Will do : Handshake + Connect + Retry HS (1) + Connect (2) + Retry HS (3)
                .create();

            let _m = server
                .mock("POST", "/cometd/56.0")
                .with_status(200)
                .match_body(
                    r#"{"channel":"/meta/connect","clientId":"1234","connectionType":"long-polling"}"#,
                )
                .with_body(
                    json!([{
                        "advice":{
                            "reconnect": "handshake"
                        },
                        "channel": "/meta/connect",
                        "successful": false,
                        "error": "error"
                    }])
                    .to_string(),
                )
                .create();

            let mut client = client(&server);

            client.init().expect("Could not init client");
            let resp = client.connect().expect_err("Connect should not return Ok");
            println!("Connect returned error message: {:#?}", resp);
            hs_mock.assert();
        }
    }
}
