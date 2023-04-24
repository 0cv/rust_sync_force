use crate::response::ErrorResponse;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("not logged in")]
    NotLoggedIn,

    #[error("Error from Salesforce status: {status:?}, url: {url:?}, body: {sfdc_errors:?}, transport_error: {transport_error:?}")]
    SfdcError {
        status: u16,
        url: String,
        sfdc_errors: Option<Vec<ErrorResponse>>,
        transport_error: Option<String>,
    },

    #[error("Input Output Error {0}")]
    IOError(#[from] ::std::io::Error),
}

impl From<ureq::Error> for Error {
    fn from(e: ureq::Error) -> Self {
        match e {
            ureq::Error::Status(status, response) => {
                let url = response.get_url().to_string();
                let response_string = format!("{:?}", response.into_string());
                let error_response = ErrorResponse {
                    message: response_string,
                    error_code: "".to_string(),
                    fields: None,
                };
                return Error::SfdcError {
                    status,
                    url,
                    sfdc_errors: Some(vec![error_response]),
                    transport_error: None,
                };
            }
            ureq::Error::Transport(transport) => Error::SfdcError {
                status: 0,
                url: transport.url().unwrap().to_string(),
                sfdc_errors: None,
                transport_error: Some(transport.to_string()),
            },
        }
    }
}
