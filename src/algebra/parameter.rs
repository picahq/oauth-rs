use handlebars::Handlebars;
use osentities::{
    connection_oauth_definition::{Computation, ComputeRequest, ConnectionOAuthDefinition},
    error::PicaError as Error,
    oauth_secret::OAuthSecret,
    InternalError,
};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde_json::Value;
use std::{
    collections::{BTreeMap, HashMap},
    str::FromStr,
};
use tracing::warn;

pub trait ParameterExt {
    fn headers(&self, computation: Option<&Computation>) -> Result<Option<HeaderMap>, Error>;
    fn body(&self, secret: &OAuthSecret) -> Result<Option<Value>, Error>;
    fn query(&self, computation: Option<&Computation>) -> Result<Option<Value>, Error>;
}

impl ParameterExt for ConnectionOAuthDefinition {
    fn headers(&self, computation: Option<&Computation>) -> Result<Option<HeaderMap>, Error> {
        headers(self, computation)
    }

    fn body(&self, secret: &OAuthSecret) -> Result<Option<Value>, Error> {
        body(secret, &self.compute.refresh)
    }

    fn query(&self, computation: Option<&Computation>) -> Result<Option<Value>, Error> {
        query(self, computation)
    }
}

fn body(secret: &OAuthSecret, refresh: &ComputeRequest) -> Result<Option<Value>, Error> {
    let payload = serde_json::to_value(secret).map_err(|e| {
        warn!("Failed to serialize secret: {}", e);
        InternalError::encryption_error("Failed to serialize secret", None)
    })?;
    let computation = refresh
        .computation
        .clone()
        .map(|computation| computation.compute::<Computation>(&payload))
        .transpose()
        .map_err(|e| {
            warn!("Failed to compute oauth payload: {}", e);
            InternalError::encryption_error("Failed to parse computation payload", None)
        })?;
    computation
        .clone()
        .map(|computation| computation.body)
        .map(|body| {
            let handlebars = Handlebars::new();

            let body_str = serde_json::to_string_pretty(&body).map_err(|e| {
                warn!("Failed to serialize body: {}", e);
                InternalError::encryption_error("Failed to serialize body", None)
            })?;

            let body = handlebars
                .render_template(&body_str, &payload)
                .map_err(|e| {
                    warn!("Failed to render body: {}", e);
                    InternalError::encryption_error("Failed to render body template", None)
                })?;

            serde_json::from_str(&body).map_err(|e| {
                warn!("Failed to deserialize body: {}", e);
                InternalError::encryption_error("Failed to deserialize body", None)
            })
        })
        .transpose()
        .map_err(|e| {
            warn!("Failed to compute body: {}", e);
            InternalError::encryption_error("Failed to compute body", None)
        })
}

fn query(
    definition: &ConnectionOAuthDefinition,
    computation: Option<&Computation>,
) -> Result<Option<Value>, Error> {
    let query_params = definition
        .configuration
        .refresh
        .query_params
        .as_ref()
        .map(|query_params| {
            let mut map = HashMap::new();
            for (key, value) in query_params {
                let key = key.to_string();
                let value = value.as_str();

                map.insert(key, value.to_string());
            }
            map
        });

    match query_params {
        Some(query_params) => {
            let payload = computation.and_then(|computation| computation.clone().query_params);
            let handlebars = handlebars::Handlebars::new();

            let query_params_str = serde_json::to_string_pretty(&query_params).map_err(|e| {
                warn!("Failed to serialize query params: {}", e);
                InternalError::encryption_error("Failed to serialize query params", None)
            })?;

            let query_params = handlebars
                .render_template(&query_params_str, &payload)
                .map_err(|e| {
                    warn!("Failed to render query params: {}", e);
                    InternalError::encryption_error("Failed to render query params template", None)
                })?;

            let query_params: BTreeMap<String, String> = serde_json::from_str(&query_params)
                .map_err(|e| {
                    warn!("Failed to deserialize query params: {}", e);
                    InternalError::encryption_error("Failed to deserialize query params", None)
                })?;

            let query_params: Result<Value, Error> = Ok(serde_json::to_value(query_params)
                .map_err(|e| {
                    warn!("Failed to serialize query params: {}", e);
                    InternalError::encryption_error("Failed to serialize query params", None)
                })?);

            query_params.map(Some)
        }
        None => Ok(None),
    }
}

fn headers(
    definition: &ConnectionOAuthDefinition,
    computation: Option<&Computation>,
) -> Result<Option<HeaderMap>, Error> {
    let headers = definition
        .configuration
        .refresh
        .headers
        .as_ref()
        .and_then(|headers| {
            let mut map = HashMap::new();
            for (key, value) in headers {
                let key = key.to_string();
                let value = value.to_str().ok()?;

                map.insert(key, value.to_string());
            }
            Some(map)
        });

    match headers {
        Some(headers) => {
            let payload = computation.and_then(|computation| computation.clone().headers);
            let handlebars = handlebars::Handlebars::new();

            let headers_str = serde_json::to_string_pretty(&headers).map_err(|e| {
                warn!("Failed to serialize headers: {}", e);
                InternalError::encryption_error("Failed to serialize headers", None)
            })?;

            let headers = handlebars
                .render_template(&headers_str, &payload)
                .map_err(|e| {
                    warn!("Failed to render headers: {}", e);
                    InternalError::encryption_error("Failed to render headers template", None)
                })?;

            let headers: BTreeMap<String, String> =
                serde_json::from_str(&headers).map_err(|e| {
                    warn!("Failed to deserialize headers: {}", e);
                    InternalError::encryption_error("Failed to deserialize headers", None)
                })?;

            let headers: Result<HeaderMap, Error> =
                headers
                    .iter()
                    .try_fold(HeaderMap::new(), |mut header_map, (key, value)| {
                        let key = HeaderName::from_str(key).map_err(|e| {
                            warn!("Failed to parse header name: {}", e);
                            InternalError::encryption_error("Failed to parse header name", None)
                        })?;

                        let value = HeaderValue::from_str(value).map_err(|e| {
                            warn!("Failed to parse header value: {}", e);
                            InternalError::encryption_error("Failed to parse header value", None)
                        })?;

                        header_map.insert(key, value);

                        Ok(header_map)
                    });

            headers.map(Some)
        }
        None => Ok(None),
    }
}
