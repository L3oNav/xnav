use std::net::SocketAddr;
use std::{fmt::Debug};

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use crate::sched::{self, Scheduler};
use std::future::Future;
use std::pin::Pin;
use http::{HeaderMap, Extensions, header, request::Parts};
use hyper::{body::{self, Incoming, Body}, Request, Response};

/// This struct represents the entire configuration file,
/// which describes a list of servers and their particular configuration options.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    #[serde(rename = "server")]
    pub servers: Vec<Server>,
}

/// Description of a single server instance in the config file.
#[derive(Serialize, Debug, Clone)]
pub struct Server {
    pub listen: Vec<SocketAddr>,
    pub patterns: Vec<Pattern>,
    #[serde(default = "default::max_connections")]
    pub max_connections: usize,
    pub name: Option<String>,
    pub log_name: String,
}

/// A pattern describes how to process requests with certain URIs,
/// and optionally includes request and response header configurations.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Pattern {
    #[serde(default = "default::uri")]
    pub uri: String,
    #[serde(flatten)]
    pub action: Action,
    pub request: Option<RequestHeaderConfig>,
    pub response: Option<ResponseHeaderConfig>,
}

/// Request header configurations for manipulating headers before forwarding.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RequestHeaderConfig {
    pub headers: RequestHeaders,
}

/// Response header configurations for manipulating headers before sending back.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ResponseHeaderConfig {
    pub headers: ResponseHeaders,
}

/// Request headers defined in the configuration.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RequestHeaders {
    pub forwarded: Option<ForwardedHeaderConfig>,
}

/// Response headers defined in the configuration.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ResponseHeaders {
    pub via: Option<CommonHeaderConfig>,
    pub server: Option<ServerHeaderConfig>,
}

/// Configuration for the `Forwarded` request header.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ForwardedHeaderConfig {
    pub extend: Option<bool>,
    pub by: Option<String>,
}

/// Common header configuration used for multiple headers.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CommonHeaderConfig {
    pub extend: bool,
}

/// Configuration for the `Server` response header.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServerHeaderConfig {
    pub override: bool,
    pub version: bool,
}

/// Describes what should be done when a request matches a pattern.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    Forward(Forward),
    Serve(String),
}

/// Proxy-specific forwarding configuration.
#[derive(Serialize, Deserialize)]
#[serde(from = "ForwardOption")]
pub struct Forward {
    pub backends: Vec<Backend>,
    pub algorithm: Algorithm,
    #[serde(skip)]
    pub scheduler: Box<dyn Scheduler + Sync + Send>,
}

impl Debug for Forward {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Forward")
            .field("backends", &self.backends)
            .field("algorithm", &self.algorithm)
            .finish()
    }
}

impl Clone for Forward {
    fn clone(&self) -> Self {
        Self {
            backends: self.backends.clone(),
            algorithm: self.algorithm,
            scheduler: sched::make(self.algorithm, &self.backends),
        }
    }
}

/// One element in the "forward" list, representing an upstream server.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(from = "BackendOption")]
pub struct Backend {
    pub address: SocketAddr,
    pub weight: usize,
}

/// Algorithm that should be used for load balancing.
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum Algorithm {
    #[serde(rename = "WRR")]
    Wrr,
}

mod default {
    pub fn uri() -> String {
        String::from("/")
    }

    pub fn max_connections() -> usize {
        1024
    }
}

/// Helper for deserializing any type `T` into [`Vec<T>`].
fn one_or_many<'de, T, D>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    T: Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum OneOrMany<T> {
        One(T),
        Many(Vec<T>),
    }

    let helper = OneOrMany::deserialize(deserializer)?;
    Ok(match helper {
        OneOrMany::One(t) => vec![t],
        OneOrMany::Many(vec) => vec,
    })
}

/// Allows specifying the upstream servers in multiple formats.
#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
enum BackendOption {
    Simple(SocketAddr),
    Weighted { address: SocketAddr, weight: usize },
}

impl From<BackendOption> for Backend {
    fn from(value: BackendOption) -> Self {
        let (address, weight) = match value {
            BackendOption::Simple(address) => (address, 1),
            BackendOption::Weighted { address, weight } => (address, weight),
        };

        Self { address, weight }
    }
}

/// Forward can be written as a single socket, list of sockets, list of objects with weights, or an object with load balancing algorithm.
#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
enum ForwardOption {
    #[serde(deserialize_with = "one_or_many")]
    Simple(Vec<Backend>),
    WithAlgorithm {
        algorithm: Algorithm,
        backends: Vec<Backend>,
    },
}

impl From<ForwardOption> for Forward {
    fn from(value: ForwardOption) -> Self {
        let (backends, algorithm) = match value {
            ForwardOption::Simple(backends) => (backends, Algorithm::Wrr),

            ForwardOption::WithAlgorithm {
                algorithm,
                backends,
            } => (backends, algorithm),
        };

        let scheduler = sched::make(algorithm, &backends);

        Self {
            backends,
            algorithm,
            scheduler,
        }
    }
}

impl<'de> Deserialize<'de> for Server {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_struct("Server", &["listen", "patterns"], ServerVisitor)
    }
}

struct ServerVisitor;

/// Possible fields of a server instance in the config file.
#[derive(Deserialize)]
#[serde(field_identifier, rename_all = "lowercase")]
enum Field {
    Listen,
    Match,
    Forward,
    Serve,
    Uri,
    Name,
    Connections,
    Request,
    Response,
}

/// Custom errors that can happen during deserialization.
#[derive(Debug)]
enum Error {
    MixedSimpleAndMatch,
    MixedActions,
    MissingConfig,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Error::MixedSimpleAndMatch => {
                "either use 'match' for multiple patterns or describe a single pattern"
            }
            Error::MixedActions => {
                "use either 'forward' or 'serve', if you need multiple patterns use 'match'"
            }
            Error::MissingConfig => "missing 'match' or simple configuration",
        };

        f.write_str(message)
    }
}

impl<'de> Visitor<'de> for ServerVisitor {
    type Value = Server;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("at least 'listen' and 'forward' or 'serve' fields")
    }

    fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
    where
        M: de::MapAccess<'de>,
    {
        let mut listen: Vec<SocketAddr> = vec![];
        let mut patterns: Vec<Pattern> = vec![];
        let mut simple_pattern: Option<Pattern> = None;
        let mut name = None;
        let mut max_connections = default::max_connections();
        let mut uri = default::uri();

        while let Some(key) = map.next_key()? {
            match key {
                Field::Listen => {
                    if !listen.is_empty() {
                        return Err(de::Error::duplicate_field("listen"));
                    }

                    listen = map.next_value::<OneOrMany<SocketAddr>>()?.into();
                }

                Field::Match => {
                    if !patterns.is_empty() {
                        return Err(de::Error::duplicate_field("match"));
                    }

                    if simple_pattern.is_some() {
                        return Err(de::Error::custom(Error::MixedSimpleAndMatch));
                    }

                    patterns = map.next_value()?;
                }

                Field::Forward => {
                    if !patterns.is_empty() {
                        return Err(de::Error::custom(Error::MixedSimpleAndMatch));
                    }

                    if let Some(pattern) = simple_pattern {
                        return match pattern.action {
                            Action::Forward(_) => Err(de::Error::duplicate_field("forward")),
                            Action::Serve(_) => Err(de::Error::custom(Error::MixedActions)),
                        };
                    }

                    simple_pattern = Some(Pattern {
                        uri: default::uri(),
                        action: Action::Forward(map.next_value()?),
                        request: None,
                        response: None,
                    });
                }

                Field::Serve => {
                    if !patterns.is_empty() {
                        return Err(de::Error::custom(Error::MixedSimpleAndMatch));
                    }

                    if let Some(pattern) = simple_pattern {
                        return match pattern.action {
                            Action::Forward(_) => Err(de::Error::custom(Error::MixedActions)),
                            Action::Serve(_) => Err(de::Error::duplicate_field("serve")),
                        };
                    }

                    simple_pattern = Some(Pattern {
                        uri: default::uri(),
                        action: Action::Serve(map.next_value()?),
                        request: None,
                        response: None,
                    });
                }

                Field::Uri => {
                    if !patterns.is_empty() {
                        return Err(de::Error::custom(Error::MixedSimpleAndMatch));
                    }

                    uri = map.next_value()?;
                }

                Field::Name => {
                    if name.is_some() {
                        return Err(de::Error::duplicate_field("name"));
                    }

                    name = Some(map.next_value()?);
                }

                Field::Connections => max_connections = map.next_value()?,

                Field::Request => {
                    if let Some(pattern) = simple_pattern.as_mut() {
                        pattern.request = Some(map.next_value()?);
                    } else {
                        return Err(de::Error::missing_field("action"));
                    }
                }

                Field::Response => {
                    if let Some(pattern) = simple_pattern.as_mut() {
                        pattern.response = Some(map.next_value()?);
                    } else {
                        return Err(de::Error::missing_field("action"));
                    }
                }
            }
        }

        if let Some(mut pattern) = simple_pattern {
            pattern.uri = uri;
            patterns.push(pattern);
        }

        if patterns.is_empty() {
            return Err(de::Error::custom(Error::MissingConfig));
        }

        if listen.is_empty() {
            return Err(de::Error::missing_field("listen"));
        }

        Ok(Server {
            listen,
            patterns,
            max_connections,
            name,
            log_name: String::from("unnamed"),
        })
    }
}

