//! Mikołaj Depta 328690

use super::entity::Entity;
use std::fmt::{Display, Formatter};
use std::str::FromStr;

pub const CRLF: &str = "\r\n";

/// Versions of HTTP protocol.
#[non_exhaustive]
#[derive(Copy, Clone)]
pub enum Version {
    V1,
    V1_1,
    V2,
    V3,
}

impl Version {
    const PREFIX_REPR: &'static str = "HTTP/";
    const V1_REPR: &'static str = "1";
    const V1_1_REPR: &'static str = "1.1";
    const V2_REPR: &'static str = "2";
    const V3_REPR: &'static str = "3";
}

impl Display for Version {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let version = match self {
            Version::V1 => Self::V1_REPR,
            Version::V1_1 => Self::V1_1_REPR,
            Version::V2 => Self::V2_REPR,
            Version::V3 => Self::V3_REPR,
        };
        write!(f, "{}{}", Self::PREFIX_REPR, version)
    }
}

pub struct ParseVersionError(String);

impl Display for ParseVersionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid http version representation: {}", self.0)
    }
}

impl FromStr for Version {
    type Err = ParseVersionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let version = s.trim_start_matches(Self::PREFIX_REPR);
        match version {
            Self::V1_REPR => Ok(Self::V1),
            Self::V1_1_REPR => Ok(Self::V1_1),
            Self::V2_REPR => Ok(Self::V2),
            Self::V3_REPR => Ok(Self::V3),
            _ => Err(ParseVersionError(version.to_owned())),
        }
    }
}

#[non_exhaustive]
pub enum Body {
    SingleSource(Entity),
}

pub struct ParseBodyError;

impl AsRef<[u8]> for Body {
    fn as_ref(&self) -> &[u8] {
        match self {
            Body::SingleSource(entity) => entity.as_ref(),
        }
    }
}

impl TryFrom<&[u8]> for Body {
    type Error = ParseBodyError;

    fn try_from(_: &[u8]) -> Result<Self, Self::Error> {
        Err(ParseBodyError)
    }
}

/// Type of http method.
#[non_exhaustive]
#[derive(Debug)]
#[allow(non_camel_case_types)]
pub enum Method {
    GET,
}

impl Method {
    const GET_REPR: &'static str = "GET";
}

impl Display for Method {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let repr = match self {
            Method::GET => Self::GET_REPR,
        };
        write!(f, "{repr}")
    }
}

pub struct ParseMethodError(String);

impl FromStr for Method {
    type Err = ParseMethodError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            Self::GET_REPR => Ok(Self::GET),
            _ => Err(ParseMethodError(s.to_owned())),
        }
    }
}
