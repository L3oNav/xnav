//! This module contains the configuration structures used for deserializing
//! TOML configuration files, along with custom deserialization logic.

use crate::threading::{self, Scheduler};
use serde::{Deserialize, Deserializer, Serialize};
use std::{net::SocketAddr, os::unix::thread};

/// Main configuration structs based on TOML config file.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    /// List of all servers.
    #[serde(rename = "server")]
    pub servers: Vec<Server>,
}

#[derive(Serialize, Debug, Clone)]
pub struct Server {
    pub listen: Vec<SocketAddr>,
    #[serde(rename = "match")]
    pub patterns: Vec<Pattern>,
    #[serde(default = "default::max_connections")]
    pub max_connections: usize,
    pub name: Option<String>,
    #[serde(skip)]
    pub log_name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Pattern {
    #[serde(default = "default::uri")]
    pub uri: String,
    #[serde(flatten)]
    pub action: Action,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(from = "BackendOption")]
pub struct Backend {
    pub address: SocketAddr,
    pub weight: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum Algorithm {
    #[serde(rename = "WRR")]
    Wrr,
}

#[derive(Serialize, Deserialize)]
#[serde(from = "ForwardOption")]
pub struct Forward {
    pub backends: Vec<Backend>,
    pub algorithm: Algorithm,
    #[serde(skip)]
    pub scheduler: Box<dyn Scheduler + Sync + Send>,
}

impl std::fmt::Debug for Forward {
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
            algorithm: self.algorithm.clone(),
            scheduler: threading::make(self.algorithm, &self.backends),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    Forward(Forward),
    Serve(String),
}

mod default {
    //! Default values for some configuration options.

    pub fn uri() -> String {
        String::from("/")
    }

    pub fn max_connections() -> usize {
        1024
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
enum OneOrMany<T> {
    One(T),
    Many(Vec<T>),
}

impl<T> From<OneOrMany<T>> for Vec<T> {
    fn from(value: OneOrMany<T>) -> Self {
        match value {
            OneOrMany::One(item) => vec![item],
            OneOrMany::Many(items) => items,
        }
    }
}

fn one_or_many<'de, T, D>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    Ok(OneOrMany::deserialize(deserializer)?.into())
}

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
        let scheduler = threading::make(algorithm, &backends);
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
}

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

impl<'de> serde::de::Visitor<'de> for ServerVisitor {
    type Value = Server;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("at least 'listen' and 'forward' or 'serve' fields")
    }

    fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
    where
        M: serde::de::MapAccess<'de>,
    {
        let mut listen = vec![];
        let mut patterns = vec![];
        let mut simple_pattern: Option<Pattern> = None;
        let mut name = None;
        let mut max_connections = default::max_connections();
        let mut uri = default::uri();

        while let Some(key) = map.next_key()? {
            match key {
                Field::Listen => {
                    if !listen.is_empty() {
                        return Err(serde::de::Error::duplicate_field("listen"));
                    }
                    listen = map.next_value::<OneOrMany<SocketAddr>>()?.into();
                }
                Field::Match => {
                    if !patterns.is_empty() {
                        return Err(serde::de::Error::duplicate_field("match"));
                    }
                    if simple_pattern.is_some() {
                        return Err(serde::de::Error::custom(Error::MixedSimpleAndMatch));
                    }
                    patterns = map.next_value()?;
                }
                Field::Forward => {
                    if !patterns.is_empty() {
                        return Err(serde::de::Error::custom(Error::MixedSimpleAndMatch));
                    }
                    if let Some(pattern) = simple_pattern.take() {
                        match pattern.action {
                            Action::Forward(_) => {
                                return Err(serde::de::Error::duplicate_field("forward"))
                            }
                            Action::Serve(_) => {
                                return Err(serde::de::Error::custom(Error::MixedActions))
                            }
                        }
                    }
                    simple_pattern = Some(Pattern {
                        uri: default::uri(),
                        action: Action::Forward(map.next_value()?),
                    });
                }
                Field::Serve => {
                    if !patterns.is_empty() {
                        return Err(serde::de::Error::custom(Error::MixedSimpleAndMatch));
                    }
                    if let Some(pattern) = simple_pattern.take() {
                        match pattern.action {
                            Action::Forward(_) => {
                                return Err(serde::de::Error::custom(Error::MixedActions))
                            }
                            Action::Serve(_) => {
                                return Err(serde::de::Error::duplicate_field("serve"))
                            }
                        }
                    }
                    simple_pattern = Some(Pattern {
                        uri: default::uri(),
                        action: Action::Serve(map.next_value()?),
                    });
                }
                Field::Uri => {
                    if !patterns.is_empty() {
                        return Err(serde::de::Error::custom(Error::MixedSimpleAndMatch));
                    }
                    uri = map.next_value()?;
                }
                Field::Name => {
                    if name.is_some() {
                        return Err(serde::de::Error::duplicate_field("name"));
                    }
                    name = Some(map.next_value()?);
                }
                Field::Connections => {
                    max_connections = map.next_value()?;
                }
            }
        }

        if let Some(mut pattern) = simple_pattern.take() {
            pattern.uri = uri;
            patterns.push(pattern);
        }

        if patterns.is_empty() {
            return Err(serde::de::Error::custom(Error::MissingConfig));
        }

        if listen.is_empty() {
            return Err(serde::de::Error::missing_field("listen"));
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
