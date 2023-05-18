use crate::errors::Error;
use crate::response::{
    AccessToken, CompositeBodyRequest, CompositeResponse, DescribeGlobalResponse, ErrorResponse,
    QueryResponse, SearchResponse, TokenErrorResponse, TokenResponse, UpsertResponse,
    VersionResponse,
};
use crate::utils::substring_before;

use regex::Regex;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;
use ureq::Response;

/// Represents a Salesforce Client
pub struct Client {
    http_client: ureq::Agent,
    client_id: Option<String>,
    client_secret: Option<String>,
    login_endpoint: String,
    instance_url: Option<String>,
    access_token: Option<AccessToken>,
    pub version: String,
}

impl Client {
    /// Inserts a new client when passed a Client ID and Client Secret. These
    /// can be obtained by creating a connected app in Salesforce
    pub fn new(client_id: Option<String>, client_secret: Option<String>) -> Self {
        let http_client = ureq::AgentBuilder::new().build();
        Client {
            http_client,
            client_id,
            client_secret,
            login_endpoint: "https://login.salesforce.com".to_string(),
            access_token: None,
            instance_url: None,
            version: "v56.0".to_string(),
        }
    }

    /// Set the login endpoint. This is useful if you want to connect to a
    /// Sandbox
    pub fn set_login_endpoint(&mut self, endpoint: &str) -> &mut Self {
        self.login_endpoint = endpoint.to_string();
        self
    }

    /// Set API Version
    pub fn set_version(&mut self, version: &str) -> &mut Self {
        self.version = version.to_string();
        self
    }

    pub fn set_instance_url(&mut self, instance_url: &str) -> &mut Self {
        self.instance_url = Some(instance_url.to_string());
        self
    }

    /// Set Access token if you've already obtained one via one of the OAuth2
    /// flows
    pub fn set_access_token(&mut self, access_token: &str) -> &mut Self {
        self.access_token = Some(AccessToken {
            token_type: "Bearer".to_string(),
            value: access_token.to_string(),
            issued_at: "".to_string(),
        });
        self
    }

    /// This will fetch an access token when provided with a refresh token
    pub fn refresh(&mut self, refresh_token: &str) -> Result<&mut Self, Error> {
        let token_url = format!("{}/services/oauth2/token", self.login_endpoint);
        let params = [
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", self.client_id.as_ref().unwrap()),
            ("client_secret", self.client_secret.as_ref().unwrap()),
        ];
        let res = self
            .http_client
            .post(token_url.as_str())
            .send_form(&params)?;

        let r: TokenResponse = res.into_json()?;
        self.access_token = Some(AccessToken {
            value: r.access_token,
            issued_at: r.issued_at,
            token_type: "Bearer".to_string(),
        });
        self.instance_url = Some(r.instance_url);
        Ok(self)
    }

    /// Login to Salesforce with username and password
    pub fn login_with_credential(
        &mut self,
        username: String,
        password: String,
    ) -> Result<&mut Self, Error> {
        let token_url = format!("{}/services/oauth2/token", self.login_endpoint);
        let params = [
            ("grant_type", "password"),
            ("client_id", self.client_id.as_ref().unwrap()),
            ("client_secret", self.client_secret.as_ref().unwrap()),
            ("username", &username),
            ("password", &password),
        ];

        match self.http_client.post(&token_url).send_form(&params) {
            Ok(res) => {
                let r: TokenResponse = res.into_json()?;
                self.access_token = Some(AccessToken {
                    value: r.access_token,
                    issued_at: r.issued_at,
                    token_type: r.token_type.ok_or(Error::NotLoggedIn)?,
                });
                self.instance_url = Some(r.instance_url);
                Ok(self)
            }
            Err(ureq::Error::Status(code, res)) => {
                let url = res.get_url().to_string();
                let error_response: TokenErrorResponse = res.into_json()?;
                Err(Error::SfdcError {
                    status: code,
                    url: url,
                    transport_error: None,
                    sfdc_errors: Some(vec![ErrorResponse {
                        message: Value::String(error_response.error_description),
                        error_code: error_response.error,
                        fields: None,
                    }]),
                })
            }
            Err(ureq::Error::Transport(transport)) => Err(Error::SfdcError {
                status: 0,
                url: transport.url().unwrap().to_string(),
                transport_error: Some(transport.to_string()),
                sfdc_errors: None,
            }),
        }
    }

    pub fn login_by_soap(
        &mut self,
        username: String,
        password: String,
    ) -> Result<&mut Self, Error> {
        let token_url = format!(
            "{login_endpoint}/services/Soap/u/{version}",
            login_endpoint = self.login_endpoint,
            version = self.version
        );
        let body = [
            "<se:Envelope xmlns:se='http://schemas.xmlsoap.org/soap/envelope/'>",
            "<se:Header/>",
            "<se:Body>",
            "<login xmlns='urn:partner.soap.sforce.com'>",
            format!("<username>{}</username>", username).as_str(),
            format!("<password>{}</password>", password).as_str(),
            "</login>",
            "</se:Body>",
            "</se:Envelope>",
        ]
        .join("");
        match self
            .http_client
            .post(token_url.as_str())
            .set("Content-Type", "text/xml")
            .set("SOAPAction", "\"\"")
            .send_string(&body)
        {
            Ok(res) => {
                let body_response = res.into_string()?;
                let re_access_token = Regex::new(r"<sessionId>([^<]+)</sessionId>")
                    .expect(&format!("Session ID is missing: '{}'", body_response).to_string());
                let re_instance_url = Regex::new(r"<serverUrl>([^<]+)</serverUrl>")
                    .expect(&format!("Server URL is missing: '{}'", body_response).to_string());
                self.access_token = Some(AccessToken {
                    value: String::from(
                        re_access_token
                            .captures(body_response.as_str())
                            .unwrap()
                            .get(1)
                            .unwrap()
                            .as_str(),
                    ),
                    issued_at: "".to_string(),
                    token_type: "Bearer".to_string(),
                });
                self.instance_url = Some(substring_before(
                    re_instance_url
                        .captures(body_response.as_str())
                        .unwrap()
                        .get(1)
                        .unwrap()
                        .as_str(),
                    "/services/",
                ));
                Ok(self)
            }
            Err(ureq::Error::Status(code, response)) => {
                let url = response.get_url().to_string();
                let body_response = response.into_string()?;
                println!("Error Code: {}. Error Response: {}", code, body_response);
                let re_message = Regex::new(r"<faultstring>([^<]+)</faultstring>")
                    .expect(&format!("Faultstring is missing: '{}'", body_response).to_string());
                let re_error_code = Regex::new(r"<faultcode>([^<]+)</faultcode>")
                    .expect(&format!("Faultcode is missing: '{}'", body_response).to_string());
                Err(Error::SfdcError {
                    status: code,
                    url: url,
                    transport_error: None,
                    sfdc_errors: Some(vec![ErrorResponse {
                        message: Value::String(String::from(
                            re_message
                                .captures(body_response.as_str())
                                .unwrap()
                                .get(1)
                                .unwrap()
                                .as_str(),
                        )),
                        error_code: String::from(
                            re_error_code
                                .captures(body_response.as_str())
                                .unwrap()
                                .get(1)
                                .unwrap()
                                .as_str(),
                        ),
                        fields: None,
                    }]),
                })
            }
            Err(ureq::Error::Transport(transport)) => Err(Error::SfdcError {
                status: 0,
                url: transport.url().unwrap().to_string(),
                transport_error: Some(transport.to_string()),
                sfdc_errors: None,
            }),
        }
    }

    /// Query record using SOQL
    pub fn query<T: DeserializeOwned>(&self, query: &str) -> Result<QueryResponse<T>, Error> {
        self.query_with(query, "query")
    }

    /// Query All records using SOQL
    pub fn query_all<T: DeserializeOwned>(&self, query: &str) -> Result<QueryResponse<T>, Error> {
        self.query_with(query, "queryAll")
    }

    fn query_with<T: DeserializeOwned>(
        &self,
        query: &str,
        query_with: &str,
    ) -> Result<QueryResponse<T>, Error> {
        // Recursive query starts with /services/data/
        let res = if query.starts_with("/services/data/") {
            let query_url = format!(
                "{}{}",
                self.instance_url.as_ref().unwrap(),
                query.to_string()
            );
            self.sfdc_get(query_url, None)?
        } else {
            let query_url = format!("{}/{}/", self.base_path(), query_with);
            self.sfdc_get(query_url, Some(vec![("q", query)]))?
        };

        // println!("ReS => {:?}", res.into_string()?);

        // Err(Error::NotLoggedIn)

        let mut json: QueryResponse<T> = res.into_json()?;
        if !json.done {
            let next_records_url = json.next_records_url.as_ref().unwrap();
            let mut recursive_json: QueryResponse<T> = self.query(&next_records_url)?;
            json.records.append(&mut recursive_json.records);
            json.next_records_url = recursive_json.next_records_url;
            json.done = recursive_json.done;
        }
        Ok(json)
    }

    /// Find records using SOSL
    pub fn search(&self, query: &str) -> Result<SearchResponse, Error> {
        let res = self.sfdc_get(
            format!("{}/search/", self.base_path()),
            Some(vec![("q", query)]),
        )?;
        Ok(res.into_json()?)
    }

    /// Get all supported API versions
    pub fn versions(&self) -> Result<Vec<VersionResponse>, Error> {
        let res = self.sfdc_get(
            format!(
                "{}/services/data/",
                self.instance_url.as_ref().ok_or(Error::NotLoggedIn)?
            ),
            None,
        )?;
        Ok(res.into_json()?)
    }

    /// Finds a record by ID
    pub fn find_by_id<T: DeserializeOwned>(
        &self,
        sobject_type: &str,
        id: &str,
    ) -> Result<T, Error> {
        let res = self.sfdc_get(
            format!("{}/sobjects/{}/{}", self.base_path(), sobject_type, id),
            None,
        )?;
        Ok(res.into_json()?)
    }

    /// Insert an SObject
    pub fn insert<T: Serialize>(
        &self,
        sobject_type: &str,
        params: T,
    ) -> Result<UpsertResponse, Error> {
        let res = self.sfdc_post(
            format!("{}/sobjects/{}", self.base_path(), sobject_type),
            params,
        )?;
        Ok(res.into_json()?)
    }

    /// Insert multiple SObjects
    pub fn inserts<T: Serialize>(
        &self,
        all_or_none: bool,
        records: Vec<T>,
    ) -> Result<Vec<Result<CompositeResponse, Error>>, Error> {
        let res = self.sfdc_post(
            format!("{}/composite/sobjects", self.base_path(),),
            self.get_composite_body_request(all_or_none, records),
        )?;

        Ok(self.partition_composite_results(res)?)
    }

    /// Updates an SObject
    pub fn update<T: Serialize>(
        &self,
        sobject_type: &str,
        id: &str,
        params: T,
    ) -> Result<(), Error> {
        self.sfdc_patch(
            format!("{}/sobjects/{}/{}", self.base_path(), sobject_type, id),
            params,
        )?;
        Ok(())
    }

    /// Updates multiple SObjects
    pub fn updates<T: Serialize>(
        &self,
        all_or_none: bool,
        records: Vec<T>,
    ) -> Result<Vec<Result<CompositeResponse, Error>>, Error> {
        let res = self.sfdc_patch(
            format!("{}/composite/sobjects", self.base_path(),),
            self.get_composite_body_request(all_or_none, records),
        )?;

        Ok(self.partition_composite_results(res)?)
    }

    /// Upserts an SObject with key
    pub fn upsert<T: Serialize>(
        &self,
        sobject_type: &str,
        key_name: &str,
        key: &str,
        params: T,
    ) -> Result<Option<UpsertResponse>, Error> {
        let res = self.sfdc_patch(
            format!(
                "{}/sobjects/{}/{}/{}",
                self.base_path(),
                sobject_type,
                key_name,
                key
            ),
            params,
        )?;

        match res.status() {
            201 => Ok(res.into_json()?),
            _ => Ok(None),
        }
    }

    /// Upserts multiple SObjects with key
    pub fn upserts<T: Serialize>(
        &self,
        all_or_none: bool,
        sobject_type: &str,
        key_name: &str,
        records: Vec<T>,
    ) -> Result<Vec<Result<CompositeResponse, Error>>, Error> {
        let res = self.sfdc_patch(
            format!(
                "{}/composite/sobjects/{}/{}",
                self.base_path(),
                sobject_type,
                key_name,
            ),
            self.get_composite_body_request(all_or_none, records),
        )?;

        Ok(self.partition_composite_results(res)?)
    }

    fn get_composite_body_request<T>(
        &self,
        all_or_none: bool,
        records: Vec<T>,
    ) -> CompositeBodyRequest<T> {
        CompositeBodyRequest {
            all_or_none: all_or_none,
            records: records.into(),
        }
    }

    /// Deletes an SObject
    pub fn delete(&self, sobject_type: &str, id: &str) -> Result<(), Error> {
        let resource_url = format!("{}/sobjects/{}/{}", self.base_path(), sobject_type, id);
        self.sfdc_delete(resource_url, None)?;
        Ok(())
    }

    /// Deletes multiple SObjects
    pub fn deletes(
        &self,
        all_or_none: bool,
        ids: Vec<String>,
    ) -> Result<Vec<Result<CompositeResponse, Error>>, Error> {
        let resource_url = format!("{}/composite/sobjects", self.base_path());
        let res = self.sfdc_delete(
            resource_url,
            Some(vec![
                ("ids", &ids.join(",")),
                ("allOrNone", &all_or_none.to_string()),
            ]),
        )?;

        Ok(self.partition_composite_results(res)?)
    }

    fn partition_composite_results(
        &self,
        res: Response,
    ) -> Result<Vec<Result<CompositeResponse, Error>>, Error> {
        let status = res.status();
        let url = res.get_url().to_string();

        let vec_response: Vec<CompositeResponse> = res.into_json()?;
        let results = vec_response
            .into_iter()
            .map(|response| {
                if response.success || response.errors.is_empty() {
                    Ok(response)
                } else {
                    Err(Error::SfdcError {
                        status,
                        url: url.to_string(),
                        sfdc_errors: Some(
                            response
                                .errors
                                .into_iter()
                                .map(|error| ErrorResponse {
                                    message: Value::String(error.message),
                                    error_code: error.status_code,
                                    fields: Some(error.fields),
                                })
                                .collect(),
                        ),
                        transport_error: None,
                    })
                }
            })
            .collect();

        Ok(results)
    }

    /// Describes all objects
    pub fn describe_global(&self) -> Result<DescribeGlobalResponse, Error> {
        let resource_url = format!("{}/sobjects/", self.base_path());
        let res = self.sfdc_get(resource_url, None)?;
        Ok(res.into_json()?)
    }

    /// Describes specific object
    pub fn describe(&self, sobject_type: &str) -> Result<String, Error> {
        let resource_url = format!("{}/sobjects/{}/describe", self.base_path(), sobject_type);
        let res = self.sfdc_get(resource_url, None)?;
        Ok(res.into_string()?)
    }

    pub fn sfdc_get(
        &self,
        url_or_path: String,
        params: Option<Vec<(&str, &str)>>,
    ) -> Result<Response, Error> {
        let mut req = self
            .http_client
            .get(&self.get_sfdc_url(url_or_path))
            .set("Authorization", &self.get_auth()?);

        let req = if let Some(params) = params {
            for param in params.into_iter() {
                req = req.query(&param.0, &param.1);
            }
            req
        } else {
            req
        };

        Ok(req.call()?)
    }

    pub fn sfdc_post<T: Serialize>(&self, url_or_path: String, body: T) -> Result<Response, Error> {
        let res = self
            .http_client
            .post(&self.get_sfdc_url(url_or_path))
            .set("Authorization", &self.get_auth()?)
            .send_json(&body)?;

        Ok(res)
    }

    pub fn sfdc_patch<T: Serialize>(
        &self,
        url_or_path: String,
        body: T,
    ) -> Result<Response, Error> {
        let res = self
            .http_client
            .patch(&self.get_sfdc_url(url_or_path))
            .set("Authorization", &self.get_auth()?)
            .send_json(&body)?;

        Ok(res)
    }

    pub fn sfdc_put<T: Serialize>(&self, url_or_path: String, body: T) -> Result<Response, Error> {
        let res = self
            .http_client
            .put(&self.get_sfdc_url(url_or_path))
            .set("Authorization", &self.get_auth()?)
            .send_json(&body)?;

        Ok(res)
    }

    pub fn sfdc_delete(
        &self,
        url_or_path: String,
        params: Option<Vec<(&str, &str)>>,
    ) -> Result<Response, Error> {
        let mut req = self
            .http_client
            .delete(&self.get_sfdc_url(url_or_path))
            .set("Authorization", &self.get_auth()?);

        let req = if let Some(params) = params {
            for param in params.into_iter() {
                req = req.query(&param.0, &param.1);
            }
            req
        } else {
            req
        };

        Ok(req.call()?)
    }

    fn get_sfdc_url(&self, url_or_path: String) -> String {
        if url_or_path.starts_with("https://") || url_or_path.starts_with("http://") {
            url_or_path
        } else {
            format!("{}{}", self.instance_url.as_ref().unwrap(), url_or_path)
        }
    }

    fn get_auth(&self) -> Result<String, Error> {
        Ok(format!(
            "Bearer {}",
            self.access_token.as_ref().ok_or(Error::NotLoggedIn)?.value
        ))
    }

    fn base_path(&self) -> String {
        format!(
            "{}/services/data/{}",
            self.instance_url.as_ref().unwrap(),
            self.version
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::{errors::Error, response::QueryResponse};
    use mockito::Server as MockServer;
    use serde::{Deserialize, Serialize};
    use serde_json::json;

    #[derive(Deserialize, Serialize)]
    #[serde(rename_all = "PascalCase")]
    struct Account {
        id: String,
        name: String,
    }

    #[test]
    fn login_with_credentials() -> Result<(), Error> {
        let mut server = MockServer::new();
        let _m = server
            .mock("POST", "/services/oauth2/token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "access_token": "this_is_access_token",
                    "issued_at": "2019-10-01 00:00:00",
                    "id": "12345",
                    "instance_url": "https://ap.salesforce.com",
                    "signature": "abcde",
                    "token_type": "Bearer",
                })
                .to_string(),
            )
            .create();

        let mut client = super::Client::new(Some("aaa".to_string()), Some("bbb".to_string()));
        let url = &MockServer::url(&server);
        client.set_login_endpoint(url);
        client.login_with_credential("u".to_string(), "p".to_string())?;
        let token = client.access_token.unwrap();
        assert_eq!("this_is_access_token", token.value);
        assert_eq!("Bearer", token.token_type);
        assert_eq!("2019-10-01 00:00:00", token.issued_at);
        assert_eq!("https://ap.salesforce.com", client.instance_url.unwrap());

        Ok(())
    }

    #[test]
    fn query() -> Result<(), Error> {
        let mut server = MockServer::new_with_port(0);
        let _m = server
            .mock("GET", "/services/data/v56.0/query/")
            .match_query(mockito::Matcher::UrlEncoded(
                "q".into(),
                "SELECT Id, Name FROM Account".into(),
            ))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "totalSize": 123,
                    "done": true,
                    "records": vec![
                        Account {
                            id: "123".to_string(),
                            name: "foo".to_string(),
                        },
                    ]
                })
                .to_string(),
            )
            .create();

        let client = create_test_client(&server);
        let r: QueryResponse<Account> = client.query("SELECT Id, Name FROM Account")?;
        assert_eq!(123, r.total_size);
        assert_eq!(true, r.done);
        assert_eq!("123", r.records[0].id);
        assert_eq!("foo", r.records[0].name);

        Ok(())
    }

    #[test]
    fn insert() -> Result<(), Error> {
        let mut server = MockServer::new_with_port(0);
        let _m = server
            .mock("POST", "/services/data/v56.0/sobjects/Account")
            .with_status(201)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "id": "12345",
                    "success": true,
                })
                .to_string(),
            )
            .create();

        let client = create_test_client(&server);
        let r = client.insert("Account", [("Name", "foo"), ("Abc__c", "123")])?;
        assert_eq!("12345", r.id);
        assert_eq!(true, r.success);

        Ok(())
    }

    #[test]
    fn update() -> Result<(), Error> {
        let mut server = MockServer::new_with_port(0);
        let _m = server
            .mock("PATCH", "/services/data/v56.0/sobjects/Account/123")
            .with_status(204)
            .with_header("content-type", "application/json")
            .create();

        let client = create_test_client(&server);
        let r = client.update("Account", "123", [("Name", "foo"), ("Abc__c", "123")]);
        assert_eq!(true, r.is_ok());

        Ok(())
    }

    #[test]
    fn upsert_201() -> Result<(), Error> {
        let mut server = MockServer::new_with_port(0);
        let _m = server
            .mock(
                "PATCH",
                "/services/data/v56.0/sobjects/Account/ExKey__c/123",
            )
            .with_status(201)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "id": "12345",
                    "success": true,
                })
                .to_string(),
            )
            .create();

        let client = create_test_client(&server);
        let r = client
            .upsert(
                "Account",
                "ExKey__c",
                "123",
                [("Name", "foo"), ("Abc__c", "123")],
            )
            .unwrap();
        assert_eq!(true, r.is_some());
        let res = r.unwrap();
        assert_eq!("12345", res.id);
        assert_eq!(true, res.success);

        Ok(())
    }

    #[test]
    fn upsert_204() -> Result<(), Error> {
        let mut server = MockServer::new_with_port(0);
        let _m = server
            .mock(
                "PATCH",
                "/services/data/v56.0/sobjects/Account/ExKey__c/123",
            )
            .with_status(204)
            .with_header("content-type", "application/json")
            .create();

        let client = create_test_client(&server);
        let r = client
            .upsert(
                "Account",
                "ExKey__c",
                "123",
                [("Name", "foo"), ("Abc__c", "123")],
            )
            .unwrap();
        assert_eq!(true, r.is_none());

        Ok(())
    }

    #[test]
    fn delete() -> Result<(), Error> {
        let mut server = MockServer::new_with_port(0);
        let _m = server
            .mock("DELETE", "/services/data/v56.0/sobjects/Account/123")
            .with_status(204)
            .with_header("content-type", "application/json")
            .create();

        let client = create_test_client(&server);
        let r = client.delete("Account", "123")?;
        println!("{:?}", r);

        Ok(())
    }

    #[test]
    fn versions() -> Result<(), Error> {
        let mut server = MockServer::new_with_port(0);
        let _m = server
            .mock("GET", "/services/data/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                json!([{
                    "label": "Winter '19",
                    "url": "https://ap.salesforce.com/services/data/v56.0/",
                    "version": "v56.0",
                }])
                .to_string(),
            )
            .create();

        let client = create_test_client(&server);
        let r = client.versions()?;
        assert_eq!("Winter '19", r[0].label);
        assert_eq!("https://ap.salesforce.com/services/data/v56.0/", r[0].url);
        assert_eq!("v56.0", r[0].version);

        Ok(())
    }

    #[test]
    fn find_by_id() -> Result<(), Error> {
        let mut server = MockServer::new_with_port(0);
        let _m = server
            .mock("GET", "/services/data/v56.0/sobjects/Account/123")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "Id": "123",
                    "Name": "foo",
                })
                .to_string(),
            )
            .create();

        let client = create_test_client(&server);
        let r: Account = client.find_by_id("Account", "123")?;
        assert_eq!("foo", r.name);

        Ok(())
    }

    fn create_test_client(server: &MockServer) -> super::Client {
        let mut client = super::Client::new(Some("aaa".to_string()), Some("bbb".to_string()));
        let url = MockServer::url(&server);
        client.set_instance_url(&url);
        client.set_access_token("this_is_access_token");
        return client;
    }
}
