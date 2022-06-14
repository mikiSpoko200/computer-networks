//! Mikołaj Depta 328690

#[allow(dead_code, unused)]

use super::common::{Body, Version};
use super::headers::{general_header::GeneralHeader, response_header::ResponseHeader};
use std::fmt::{Display, Formatter};
use crate::http::common;
use crate::http::headers::Headers;

pub struct StatusLine {
    version: Version,
    status_code: StatusCode,
}

impl StatusLine {
    pub fn new(version: Version, status_code: StatusCode) -> Self {
        Self { version, status_code }
    }
}

impl Display for StatusLine {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}{}", self.version, self.status_code, common::CRLF)
    }
}

#[non_exhaustive]
pub enum StatusCode {
    Ok,
    MovedPermanently,
    Forbidden,
    NotFound,
    NotImplemented,
}

impl StatusCode {
    const OK_CODE: usize = 200;
    const MOVED_PERMANENTLY_CODE: usize = 301;
    const FORBIDDEN_CODE: usize = 403;
    const NOT_FOUND_CODE: usize = 404;
    const NOT_IMPLEMENTED_CODE: usize = 501;

    const OK_MESSAGE: &'static str = "OK";
    const MOVED_PERMANENTLY_MESSAGE: &'static str = "Moved Permanently";
    const FORBIDDEN_MESSAGE: &'static str = "Forbidden";
    const NOT_FOUND_MESSAGE: &'static str = "Not Found";
    const NOT_IMPLEMENTED_MESSAGE: &'static str = "Not Implemented";
}

impl Display for StatusCode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let (code, message) = match &self {
            StatusCode::Ok => (Self::OK_CODE, Self::OK_MESSAGE),
            StatusCode::MovedPermanently => (
                Self::MOVED_PERMANENTLY_CODE,
                Self::MOVED_PERMANENTLY_MESSAGE,
            ),
            StatusCode::Forbidden => (Self::FORBIDDEN_CODE, Self::FORBIDDEN_MESSAGE),
            StatusCode::NotFound => (Self::NOT_FOUND_CODE, Self::NOT_FOUND_MESSAGE),
            StatusCode::NotImplemented => {
                (Self::NOT_IMPLEMENTED_CODE, Self::NOT_IMPLEMENTED_MESSAGE)
            }
        };
        write!(f, "{} {}", code, message)
    }
}

pub struct Response {
    status_line: StatusLine,
    headers: Headers,
    body: Option<Body>,
    buffer: Vec<u8>,
}

impl Response {
    const SECTION_SEP: &'static str = "\r\n\r\n";

    pub fn new(
        status_line: StatusLine,
        headers: Headers,
        body: Option<Body>,
    ) -> Self {
        let mut instance = Self {
            status_line,
            headers,
            body,
            buffer: Vec::new(),
        };
        instance.buffer.extend_from_slice(instance.to_string().as_bytes());
        if let Some(body) = &instance.body {
            instance.buffer.extend_from_slice(body.as_ref())
        }
        instance
    }
}

impl Display for Response {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}{}", self.status_line, self.headers, common::CRLF)
    }
}

impl AsRef<[u8]> for Response {
    fn as_ref(&self) -> &[u8] {
        self.buffer.as_ref()
    }
}
