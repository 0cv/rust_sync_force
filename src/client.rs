use crate::errors::Error;
use crate::response::{
    AccessToken, CreateResponse, DescribeGlobalResponse, ErrorResponse,
    QueryResponse, SearchResponse, TokenErrorResponse, TokenResponse, VersionResponse,
};
use crate::utils::substring_before;

use regex::Regex;
use ureq::Response;
use serde::de::DeserializeOwned;
use serde::Serialize;


/// Represents a Salesforce Client
pub struct Client {
    http_client: ureq::Agent,
    client_id: Option<String>,
    client_secret: Option<String>,
    login_endpoint: String,
    instance_url: Option<String>,
    access_token: Option<AccessToken>,
    version: String,
}

impl Client {
    /// Creates a new client when passed a Client ID and Client Secret. These
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

        match self
            .http_client
            .post(&token_url)
            .send_form(&params) {

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
                        message: error_response.error_description,
                        error_code: error_response.error,
                        fields: None,
                    }])
                })
            }
            Err(ureq::Error::Transport(transport)) => {
                Err(Error::SfdcError {
                    status: 0,
                    url: transport.url().unwrap().to_string(),
                    transport_error: Some(transport.to_string()),
                    sfdc_errors: None
                })
            }
        }
    }

    pub fn login_by_soap(&mut self, username: String, password: String) -> Result<&mut Self, Error> {
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
            .send_string(&body) {
            
            Ok(res) => {
                let body_response = res.into_string()?;
                let re_access_token = Regex::new(r"<sessionId>([^<]+)</sessionId>").unwrap();
                let re_instance_url = Regex::new(r"<serverUrl>([^<]+)</serverUrl>").unwrap();
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
                let re_message = Regex::new(r"<faultstring>([^<]+)</faultstring>").unwrap();
                let re_error_code = Regex::new(r"<faultcode>([^<]+)</faultcode>").unwrap();
                Err(Error::SfdcError {
                    status: code,
                    url: url,
                    transport_error: None,
                    sfdc_errors: Some(vec![ErrorResponse {
                        message: String::from(
                            re_message
                                .captures(body_response.as_str())
                                .unwrap()
                                .get(1)
                                .unwrap()
                                .as_str(),
                        ),
                        error_code: String::from(
                            re_error_code
                                .captures(body_response.as_str())
                                .unwrap()
                                .get(1)
                                .unwrap()
                                .as_str(),
                        ),
                        fields: None,
                    }])
                })
            }
            Err(ureq::Error::Transport(transport)) => {
                Err(Error::SfdcError {
                    status: 0,
                    url: transport.url().unwrap().to_string(),
                    transport_error: Some(transport.to_string()),
                    sfdc_errors: None
                })
            }
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

    fn query_with<T: DeserializeOwned>(&self, query: &str, query_with: &str) -> Result<QueryResponse<T>, Error> {
        // Recursive query starts with /services/data/
        let res = if query.starts_with("/services/data/") {
            let query_url = format!("{}{}", self.instance_url.as_ref().unwrap(), query.to_string());
            self.get(query_url, None)?
        } else {
            let query_url = format!("{}/{}/", self.base_path(), query_with);
            self.get(query_url, Some(("q", query)))?
        };

        let mut json: QueryResponse<T> = res.into_json()?;
        if !json.done {
            let next_records_url = json.next_records_url.as_ref().unwrap();
            let mut recursive_json: QueryResponse<T> = self.query(&next_records_url)?;
            recursive_json.records.append(&mut json.records);
            Ok(recursive_json)
        } else {
            Ok(json)
        }
    }

    /// Find records using SOSL
    pub fn search(&self, query: &str) -> Result<SearchResponse, Error> {
        let query_url = format!("{}/search/", self.base_path());
        let res = self.get(query_url, Some(("q", query)))?;
        Ok(res.into_json()?)
    }

    /// Get all supported API versions
    pub fn versions(&self) -> Result<Vec<VersionResponse>, Error> {
        let versions_url = format!(
            "{}/services/data/",
            self.instance_url.as_ref().ok_or(Error::NotLoggedIn)?
        );
        let res = self.get(versions_url, None)?;
        Ok(res.into_json()?)
    }

    /// Finds a record by ID
    pub fn find_by_id<T: DeserializeOwned>(
        &self,
        sobject_name: &str,
        id: &str,
    ) -> Result<T, Error> {
        let resource_url = format!("{}/sobjects/{}/{}", self.base_path(), sobject_name, id);
        let res = self.get(resource_url, None)?;
        Ok(res.into_json()?)
    }

    /// Creates an SObject
    pub fn create<T: Serialize>(
        &self,
        sobject_name: &str,
        params: T,
    ) -> Result<CreateResponse, Error> {
        let resource_url = format!("{}/sobjects/{}", self.base_path(), sobject_name);
        let res = self.post(resource_url, params)?;
        Ok(res.into_json()?)
    }

    /// Updates an SObject
    pub fn update<T: Serialize>(
        &self,
        sobject_name: &str,
        id: &str,
        params: T,
    ) -> Result<(), Error> {
        let resource_url = format!("{}/sobjects/{}/{}", self.base_path(), sobject_name, id);
        self.patch(resource_url, params)?;
        Ok(())
    }

    /// Upserts an SObject with key
    pub fn upsert<T: Serialize>(
        &self,
        sobject_name: &str,
        key_name: &str,
        key: &str,
        params: T,
    ) -> Result<Option<CreateResponse>, Error> {
        let resource_url = format!(
            "{}/sobjects/{}/{}/{}",
            self.base_path(),
            sobject_name,
            key_name,
            key
        );
        let res = self.patch(resource_url, params)?;

        match res.status() {
            201 => Ok(res.into_json()?),
            _ => Ok(None),
        }
    }

    /// Deletes an SObject
    pub fn destroy(&self, sobject_name: &str, id: &str) -> Result<(), Error> {
        let resource_url = format!("{}/sobjects/{}/{}", self.base_path(), sobject_name, id);
        self.delete(resource_url)?;
        Ok(())
    }

    /// Describes all objects
    pub fn describe_global(&self) -> Result<DescribeGlobalResponse, Error> {
        let resource_url = format!("{}/sobjects/", self.base_path());
        let res = self.get(resource_url, None)?;
        Ok(res.into_json()?)
    }

    /// Describes specific object
    pub fn describe(&self, sobject_name: &str) -> Result<String, Error> {
        let resource_url = format!("{}/sobjects/{}/describe", self.base_path(), sobject_name);
        let res = self.get(resource_url, None)?;
        Ok(res.into_string()?)
    }

    pub fn rest_get(
        &self,
        path: String,
        params: Option<(&str, &str)>,
    ) -> Result<Response, Error> {
        let url = format!("{}{}", self.instance_url.as_ref().unwrap(), path);
        let req = self
            .http_client
            .get(url.as_str())
            .set("Authorization", &self.get_auth()?);
        
        let req = if let Some(params) = params {
            req.to_owned().query(&params.0, &params.1)
        } else {
            req
        };
        Ok(req.call()?)
    }

    pub fn rest_post<T: Serialize>(
        &self,
        path: String,
        params: T,
    ) -> Result<Response, Error> {
        let url = format!("{}{}", self.instance_url.as_ref().unwrap(), path);
        let res = self
            .http_client
            .post(url.as_str())
            .set("Authorization", &self.get_auth()?)
            .send_json(&params)?;
        Ok(res)
    }

    pub fn rest_patch<T: Serialize>(
        &self,
        path: String,
        params: T,
    ) -> Result<Response, Error> {
        let url = format!("{}{}", self.instance_url.as_ref().unwrap(), path);
        let res = self
            .http_client
            .patch(url.as_str())
            .set("Authorization", &self.get_auth()?)
            .send_json(&params)?;
        Ok(res)
    }

    pub fn rest_put<T: Serialize>(&self, path: String, params: T) -> Result<Response, Error> {
        let url = format!("{}{}", self.instance_url.as_ref().unwrap(), path);
        let res = self
            .http_client
            .put(url.as_str())
            .set("Authorization", &self.get_auth()?)
            .send_json(&params)?;
        Ok(res)
    }

    pub fn rest_delete(&self, path: String) -> Result<Response, Error> {
        let url = format!("{}{}", self.instance_url.as_ref().unwrap(), path);
        let res = self
            .http_client
            .delete(url.as_str())
            .set("Authorization", &self.get_auth()?)
            .call()?;
        Ok(res)
    }

    fn get(&self, url: String, params: Option<(&str, &str)>) -> Result<Response, Error> {
        ureq::AgentBuilder::new().build();
        let req = self
            .http_client
            .get(url.as_str())
            .set("Authorization", &self.get_auth()?);
        
        let req = if let Some(params) = params {
            req.to_owned().query(&params.0, &params.1)
        } else {
            req
        };
        Ok(req.call()?)
    }

    fn post<T: Serialize>(&self, url: String, params: T) -> Result<Response, Error> {
        let res = self
            .http_client
            .post(url.as_str())
            .set("Authorization", &self.get_auth()?)
            .send_json(&params)?;
        Ok(res)
    }

    fn patch<T: Serialize>(&self, url: String, params: T) -> Result<Response, Error> {
        let res = self
            .http_client
            .patch(url.as_str())
            .set("Authorization", &self.get_auth()?)
            .send_json(&params)?;
        Ok(res)
    }

    fn delete(&self, url: String) -> Result<Response, Error> {
        let res = self
            .http_client
            .delete(url.as_str())
            .set("Authorization", &self.get_auth()?)
            .call()?;
        Ok(res)
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
    use mockito::mock;
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
        let _m = mock("POST", "/services/oauth2/token")
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
        let url = &mockito::server_url();
        client.set_login_endpoint(url);
        client
            .login_with_credential("u".to_string(), "p".to_string())?;
        let token = client.access_token.unwrap();
        assert_eq!("this_is_access_token", token.value);
        assert_eq!("Bearer", token.token_type);
        assert_eq!("2019-10-01 00:00:00", token.issued_at);
        assert_eq!("https://ap.salesforce.com", client.instance_url.unwrap());

        Ok(())
    }

    #[test]
    fn query() -> Result<(), Error> {
        let _m = mock(
            "GET",
            "/services/data/v56.0/query/?q=SELECT+Id%2C+Name+FROM+Account",
        )
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

        let client = create_test_client();
        let r: QueryResponse<Account> = client.query("SELECT Id, Name FROM Account")?;
        assert_eq!(123, r.total_size);
        assert_eq!(true, r.done);
        assert_eq!("123", r.records[0].id);
        assert_eq!("foo", r.records[0].name);

        Ok(())
    }

    #[test]
    fn create() -> Result<(), Error> {
        let _m = mock("POST", "/services/data/v56.0/sobjects/Account")
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

        let client = create_test_client();
        let r = client
            .create("Account", [("Name", "foo"), ("Abc__c", "123")])?;
        assert_eq!("12345", r.id);
        assert_eq!(true, r.success);

        Ok(())
    }

    #[test]
    fn update() -> Result<(), Error> {
        let _m = mock("PATCH", "/services/data/v56.0/sobjects/Account/123")
            .with_status(204)
            .with_header("content-type", "application/json")
            .create();

        let client = create_test_client();
        let r = client
            .update("Account", "123", [("Name", "foo"), ("Abc__c", "123")]);
        assert_eq!(true, r.is_ok());

        Ok(())
    }

    #[test]
    fn upsert_201() -> Result<(), Error> {
        let _m = mock(
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

        let client = create_test_client();
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
        let _m = mock(
            "PATCH",
            "/services/data/v56.0/sobjects/Account/ExKey__c/123",
        )
        .with_status(204)
        .with_header("content-type", "application/json")
        .create();

        let client = create_test_client();
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
    fn destroy() -> Result<(), Error> {
        let _m = mock("DELETE", "/services/data/v56.0/sobjects/Account/123")
            .with_status(204)
            .with_header("content-type", "application/json")
            .create();

        let client = create_test_client();
        let r = client.destroy("Account", "123")?;
        println!("{:?}", r);

        Ok(())
    }

    #[test]
    fn versions() -> Result<(), Error> {
        let _m = mock("GET", "/services/data/")
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

        let client = create_test_client();
        let r = client.versions()?;
        assert_eq!("Winter '19", r[0].label);
        assert_eq!("https://ap.salesforce.com/services/data/v56.0/", r[0].url);
        assert_eq!("v56.0", r[0].version);

        Ok(())
    }

    #[test]
    fn find_by_id() -> Result<(), Error> {
        let _m = mock("GET", "/services/data/v56.0/sobjects/Account/123")
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

        let client = create_test_client();
        let r: Account = client.find_by_id("Account", "123")?;
        assert_eq!("foo", r.name);

        Ok(())
    }

    fn create_test_client() -> super::Client {
        let mut client = super::Client::new(Some("aaa".to_string()), Some("bbb".to_string()));
        let url = &mockito::server_url();
        client.set_instance_url(url);
        client.set_access_token("this_is_access_token");
        return client;
    }
}
