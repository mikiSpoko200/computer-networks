//! Mikołaj Depta 328690

use super::common::{
    Body, Method, ParseBodyError, ParseMethodError, ParseVersionError, Version, CRLF
};
use super::headers::{
    ParseHeaderError,
    HeaderParser,
    Headers
};
use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};
use std::str;
use std::str::{FromStr, Utf8Error};
use crate::http::common;


pub struct StartLine {
    method: Method,
    url: PathBuf,
    version: Version,
}

impl StartLine {
    pub fn new(method: Method, url: &Path, version: Version) -> Self {
        Self {
            method,
            url: url.to_owned(),
            version,
        }
    }

    pub fn method(&self) -> &Method {
        &self.method
    }

    pub fn url(&self) -> &Path {
        &self.url
    }

    pub fn version(&self) -> &Version {
        &self.version
    }

}

#[non_exhaustive]
pub enum ParseUrlError {
    InvalidUtf8(Box<[u8]>),
}

pub enum ParseStartLineError {
    InvalidFormatError(String),
    ParseMethodError(ParseMethodError),
    ParseUrlError(ParseUrlError),
    ParseVersionError(ParseVersionError),
}

impl From<ParseMethodError> for ParseStartLineError {
    fn from(err: ParseMethodError) -> Self {
        Self::ParseMethodError(err)
    }
}

impl From<ParseVersionError> for ParseStartLineError {
    fn from(err: ParseVersionError) -> Self {
        Self::ParseVersionError(err)
    }
}

impl FromStr for StartLine {
    type Err = ParseStartLineError;

    fn from_str(line: &str) -> Result<Self, Self::Err> {
        let mut tags = line.split_whitespace();
        let method = tags
            .next()
            .ok_or_else(|| Self::Err::InvalidFormatError(line.to_owned()))?
            .parse()?;
        let url = tags
            .next()
            .ok_or_else(|| Self::Err::InvalidFormatError(line.to_owned()))?
            .as_ref();
        let version = tags
            .next()
            .ok_or_else(|| Self::Err::InvalidFormatError(line.to_owned()))?
            .parse()?;

        Ok(Self::new(method, url, version))
    }
}

impl Display for StartLine {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {} {}{}", self.method, self.url.display(), self.version, common::CRLF)
    }
}


pub enum ParseRequestError {
    InvalidUtf8Error(Utf8Error),
    MissingStartLineError,
    InvalidFormatError(InvalidRequestFormatError),
    ParseStartLineError(ParseStartLineError),
    ParseHeaderError(ParseHeaderError),
    ParseBodyError(ParseBodyError),
}

/// Enumeration of `Header`s required for `Request` to be created.
pub enum RequiredHeaders {
    Host,
}

/// Request parameter validation Error
pub enum ValidateRequestParamsError {
    RequiredHeaderMissing(RequiredHeaders)
}

pub trait RequestValidator {
    fn validate(&self, request: &Request) -> Result<(), ValidateRequestParamsError>;
}

/// Validator that checks if `GeneralHeader::Host` instance is present in `Request`'s `headers`.
pub struct SimpleRequestValidator;

impl RequestValidator for SimpleRequestValidator {
    fn validate(&self, request: &Request) -> Result<(), ValidateRequestParamsError> {
        if request.headers.host().is_none() {
            Err(ValidateRequestParamsError::RequiredHeaderMissing(RequiredHeaders::Host))
        } else {
            Ok(())
        }
    }
}

pub struct RequestMetaData {
    pub start_line: StartLine,
    pub headers: Headers
}


/// Try parsing array of bytes into valid HTTP start line and headers.
///
/// It is assumed that data all headers are nonempty bytes sequences that end with CRLF.
/// This means that in particular last header should also contain CRLF.
/// If given slice was to come from some buffer in which HTTP message separation sequence was detected
/// the slice should contain bytes up to and including the first two bytes of that separation sequence.
///
/// # Example:
///
/// ```
/// buffer = String::from("GET / HTTP/1.1\r\nHost: www.rust-lang.org\r\n\r\n");
/// let separator = buffer.find("\r\n\r\n").unwrap();
/// 
/// // _ = RequestMetaData::try_from(&buffer[..separator+2].as_bytes());
/// ```
impl TryFrom<&[u8]> for RequestMetaData {
    type Error = ParseRequestError;

    fn try_from(raw_metadata: &[u8]) -> Result<Self, Self::Error> {
        let metadata = str::from_utf8(raw_metadata)?;
        let sep = metadata
            .find(CRLF)
            .ok_or_else(|| Self::Error::ParseStartLineError(ParseStartLineError::InvalidFormatError(metadata.to_owned())))?;
        let (start_line, [_, _, headers_repr @ ..]) = metadata.split_at(sep);
        let start_line = start_line.parse()?;
        let headers = Headers::parse(headers_repr)?;

        Ok(Self { start_line, headers })
    }
}

pub struct Request {
    start_line: StartLine,
    headers: Headers,
    body: Option<Body>,
}

impl Request {
    pub const MAX_GET_SIZE: usize = 8192;
    pub const SECTION_SEP: &'static [u8] = b"\r\n\r\n";

    pub fn new(start_line: StartLine, headers: Headers, body: Option<Body>) -> Self {
        Self { start_line, headers, body }
    }

    pub fn headers(&self) -> &Headers {
        &self.headers
    }
    
    pub fn start_line(&self) -> &StartLine {
        &self.start_line
    }

    pub fn section_sep_pos(data: &[u8]) -> Option<usize> {
        data.windows(Self::SECTION_SEP.len()).position(|wind| wind == Self::SECTION_SEP)
    }
}

#[non_exhaustive]
pub enum InvalidRequestFormatError {
    SectionSeparatorMissing,
}

impl From<ParseStartLineError> for ParseRequestError {
    fn from(err: ParseStartLineError) -> Self {
        Self::ParseStartLineError(err)
    }
}

impl From<Utf8Error> for ParseRequestError {
    fn from(err: Utf8Error) -> Self {
        Self::InvalidUtf8Error(err)
    }
}
