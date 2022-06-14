//! Mikołaj Depta 328690
//!
//! This module exposes http headers.

use std::collections::HashSet;
use std::path::Path;
use std::fmt::{Display, Formatter};
use std::rc::Rc;

// region Errors
#[derive(Debug)]
pub enum InvalidHeaderFormatError {
    ColonMissing,
    CrlfMissing,
}

impl Display for InvalidHeaderFormatError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "invalid HTTP header format: {}",
            match self {
                Self::ColonMissing => "colon missing",
                Self::CrlfMissing => "no trailing CRLF byte sequence",
            }
        )
    }
}

#[derive(Debug)]
pub enum UnsupportedHeaderError {
    UnsupportedName(String),
    UnsupportedValue(String, String),
}

impl Display for UnsupportedHeaderError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedName(name) => write!(f, "unsupported HTTP header {name}"),
            Self::UnsupportedValue(name, value) => {
                write!(f, "unsupported HTTP header value {value} for header {name}")
            }
        }
    }
}

#[derive(Debug)]
pub enum ParseHeaderError {
    InvalidFormat(InvalidHeaderFormatError),
    Unsupported(UnsupportedHeaderError),
}

impl From<InvalidHeaderFormatError> for ParseHeaderError {
    fn from(err: InvalidHeaderFormatError) -> Self {
        Self::InvalidFormat(err)
    }
}

impl From<UnsupportedHeaderError> for ParseHeaderError {
    fn from(err: UnsupportedHeaderError) -> Self {
        Self::Unsupported(err)
    }
}

impl Display for ParseHeaderError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidFormat(err) => write!(f, "{err}"),
            Self::Unsupported(err) => write!(f, "{err}"),
        }
    }
}
// endregion

pub mod response_header {
    use crate::http::headers::{ParseHeaderError, UnsupportedHeaderError};
    use std::fmt::{Display, Formatter};
    use std::hash::{Hash, Hasher};
    use std::path::{PathBuf};
    use std::rc::Rc;

    pub type ResponseHeaders = Rc<[ResponseHeader]>;

    #[non_exhaustive]
    #[derive(Debug, Clone, Hash, Eq, PartialEq)]
    pub enum ResponseHeader {
        Location(PathBuf),
    }

    impl ResponseHeader {
        const LOCATION_REPR: &'static str = "location";
        const SUPPORTED_HEADERS: [&'static str; 1] = [Self::LOCATION_REPR];

        fn is_supported(header_name: &str) -> bool {
            Self::SUPPORTED_HEADERS.contains(&header_name)
        }

        pub fn parse(name: &str, value: &str) -> Result<Self, ParseHeaderError> {
            match name.to_lowercase().as_str() {
                Self::LOCATION_REPR => Ok(Self::Location(value.into())),
                _ => Err(ParseHeaderError::from(
                    UnsupportedHeaderError::UnsupportedName(name.to_owned()),
                )),
            }
        }
    }

    impl Display for ResponseHeader {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            match self {
                ResponseHeader::Location(location) => write!(f, "{}", location.display()),
            }
        }
    }
}

pub mod entity_header {
    use std::ffi::OsStr;
    use std::fmt::{Display, Formatter};
    use std::path::Path;
    use std::rc::Rc;

    pub type EntityHeaders = Rc<[EntityHeader]>;

    #[non_exhaustive]
    #[derive(Debug, Clone, Hash, Eq, PartialEq)]
    pub enum EntityHeader {
        ContentLength(usize),
        ContentType(ContentType),
    }

    impl EntityHeader {
        const CONTENT_LENGTH_REPR: &'static str = "Content-Length";
        const CONTENT_TYPE_REPR: &'static str = "Content-Type";
    }

    impl Display for EntityHeader {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            match self {
                EntityHeader::ContentLength(len) => {
                    write!(f, "{}: {}", Self::CONTENT_LENGTH_REPR, len)
                }
                EntityHeader::ContentType(content_type) => {
                    write!(f, "{}: {}\r\n", Self::CONTENT_TYPE_REPR, content_type)
                }
            }
        }
    }

    // region Content-Type
    #[non_exhaustive]
    #[derive(Debug, Clone, Hash, Eq, PartialEq)]
    pub enum ContentType {
        Txt,
        Html,
        Css,
        Jpg,
        Jpeg,
        Png,
        Pdf,
        OctetSteam,
    }

    impl Display for ContentType {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "{}",
                match self {
                    ContentType::Txt => "text/plain; charset=utf-8",
                    ContentType::Html => "text/html; charset=utf-8",
                    ContentType::Css => "text/css; charset=utf-8",
                    ContentType::Jpg => "image/jpeg",
                    ContentType::Jpeg => "image/jpeg",
                    ContentType::Png => "image/png",
                    ContentType::Pdf => "application/pdf",
                    ContentType::OctetSteam => "application/octet-stream",
                }
            )
        }
    }

    impl Default for ContentType {
        fn default() -> Self {
            Self::OctetSteam
        }
    }
    
    impl TryFrom<&Path> for ContentType {
        type Error = ();

        fn try_from(file: &Path) -> Result<Self, Self::Error> {
            if file.metadata().unwrap().is_file() {
                match file.extension() {
                    OsStr::new("txt") => Ok(Self::Txt),
                    OsStr::new("html") => Ok(Self::Html),
                    OsStr::new("css") => Ok(Self::Css),
                    OsStr::new("jpg") => Ok(Self::Jpg),
                    OsStr::new("jpeg") => Ok(Self::Jpeg),
                    OsStr::new("png") => Ok(Self::Png),
                    OsStr::new("pdf") => Ok(Self::Pdf),
                    _ => Ok(Self::OctetSteam),
                }
            } else {
                Err(()) // NotFound
            }
        }
    }
    // endregion
}

pub mod general_header {
    use super::{ParseHeaderError, UnsupportedHeaderError};
    use std::fmt::{Display, Formatter};
    use std::rc::Rc;
    use std::str::FromStr;

    pub type GeneralHeaders = Rc<[GeneralHeader]>;

    mod representation {
        pub(super) const CONNECTION: &str = "Connection";
    }

    mod patterns {
        pub(super) const CONNECTION: &str = "connection";
    }

    #[non_exhaustive]
    #[derive(Debug, Clone, Hash, Eq, PartialEq)]
    pub enum GeneralHeader {
        Connection(ConnectionType),
    }

    impl GeneralHeader {
        pub const SUPPORTED_HEADERS: [&'static str; 1] = [patterns::CONNECTION];

        pub fn connection(&self) -> &ConnectionType {
            match self {
                GeneralHeader::Connection(ct) => { ct }
            }
        }

        pub fn parse(name: &str, value: &str) -> Result<Self, ParseHeaderError> {
            if name.trim().to_lowercase() == patterns::CONNECTION {
                let connection_type = value.parse().map_err(|_| {
                    UnsupportedHeaderError::UnsupportedValue(name.to_owned(), value.to_owned())
                })?;
                Ok(Self::Connection(connection_type))
            } else {
                Err(ParseHeaderError::from(
                    UnsupportedHeaderError::UnsupportedName(name.to_owned()),
                ))
            }
        }
    }

    impl Display for GeneralHeader {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::Connection(ct) => write!(f, "{}: {}", representation::CONNECTION, ct),
            }
        }
    }

    // region Connection-Type
    #[non_exhaustive]
    #[derive(Debug, Clone, Hash, Eq, PartialEq)]
    pub enum ConnectionType {
        KeepAlive,
        Close,
    }

    impl ConnectionType {
        const KEEP_ALIVE_REPR: &'static str = "KeepAlive";
        const CLOSE_REPR: &'static str = "Close";
    }

    impl FromStr for ConnectionType {
        type Err = ();

        fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
            match s {
                Self::KEEP_ALIVE_REPR => Ok(Self::KeepAlive),
                Self::CLOSE_REPR => Ok(Self::Close),
                _ => Err(()),
            }
        }
    }

    impl Display for ConnectionType {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "{}",
                match self {
                    ConnectionType::KeepAlive => Self::KEEP_ALIVE_REPR,
                    ConnectionType::Close => Self::CLOSE_REPR,
                }
            )
        }
    }

    impl Default for ConnectionType {
        fn default() -> Self {
            Self::KeepAlive
        }
    }
    // endregion
}

pub mod request_header {
    use crate::http::headers::{ParseHeaderError, UnsupportedHeaderError};
    use std::rc::Rc;

    pub type RequestHeaders = Rc<[RequestHeader]>;

    #[non_exhaustive]
    #[derive(Debug, Clone, Hash, Eq, PartialEq)]
    pub enum RequestHeader {
        Host(String, Option<u16>),
    }

    mod representation {
        pub(super) const HOST: &str = "Host";
    }

    mod patterns {
        pub(super) const HOST: &str = "host";
    }

    impl RequestHeader {
        pub const SUPPORTED_HEADERS: [&'static str; 1] = [patterns::HOST];

        pub fn parse(name: &str, value: &str) -> Result<Self, ParseHeaderError> {
            if name.trim().to_lowercase() == patterns::HOST {
                return if let Some(sep_index) = value.find(':') {
                    let (domain, port) = value.split_at(sep_index);
                    let port = port.parse().map_err(|_| {
                        UnsupportedHeaderError::UnsupportedValue(name.to_owned(), value.to_owned())
                    })?;
                    Ok(Self::Host(domain.into(), Some(port)))
                } else {
                    Ok(Self::Host(value.into(), None))
                };
            }
            Err(ParseHeaderError::from(
                UnsupportedHeaderError::UnsupportedName(name.to_owned()),
            ))
        }
    }
}

use entity_header::{EntityHeaders, EntityHeader, ContentType};
use general_header::{GeneralHeaders, GeneralHeader, ConnectionType};
use request_header::{RequestHeaders, RequestHeader};
use response_header::{ResponseHeaders, ResponseHeader};

pub enum Header {
    General(GeneralHeader),
    Request(RequestHeader),
    Response(ResponseHeader),
    Entity(EntityHeader),
}

pub struct HeadersBuilder {
    general_headers: GeneralHeaders,
    request_headers: Option<RequestHeaders>,
    response_headers: Option<ResponseHeaders>,
    entity_headers: Option<EntityHeaders>,
}

impl HeadersBuilder {
    pub fn new(general_headers: GeneralHeaders) -> Self {
        Self { general_headers, request_headers: None, response_headers: None, entity_headers: None }
    }

    pub fn with_request_headers(mut self, request_headers: RequestHeaders) -> Self {
        self.request_headers = Some(request_headers);
        self
    }

    pub fn with_response_headers(mut self, response_headers: ResponseHeaders) -> Self {
        self.response_headers = Some(response_headers);
        self
    }

    pub fn with_entity_headers(mut self, entity_headers: EntityHeaders) -> Self {
        self.entity_headers = Some(entity_headers);
        self
    }

    pub fn build(self) -> Headers {
        Headers::new(
            self.general_headers,
            self.request_headers,
            self.response_headers,
            self.entity_headers
        )
    }
}


pub struct Headers {
    general_headers: GeneralHeaders,
    request_headers: Option<RequestHeaders>,
    response_headers: Option<ResponseHeaders>,
    entity_headers: Option<EntityHeaders>,
}

impl Headers {
    pub fn new(
        general_headers: GeneralHeaders,
        request_headers: Option<RequestHeaders>,
        response_headers: Option<ResponseHeaders>,
        entity_headers: Option<EntityHeaders>
    ) -> Self {
        Self { general_headers, request_headers, response_headers, entity_headers }
    }

    pub fn general_headers(&self) -> GeneralHeaders {
        self.general_headers.clone()
    }

    pub fn request_headers(&self) -> Option<RequestHeaders> {
        self.request_headers.clone()
    }

    pub fn response_headers(&self) -> Option<ResponseHeaders> {
        self.response_headers.clone()
    }

    pub fn entity_headers(&self) -> Option<EntityHeaders> {
        self.entity_headers.clone()
    }

    // region header getters
    pub fn parse<P: HeaderParser>(headers: &str) -> Result<Self, ParseHeaderError> {
        let parser = P::default();

        let mut general_headers = Vec::new();
        let mut request_headers = Vec::new();
        let mut response_headers = Vec::new();
        let mut entity_headers = Vec::new();

        for header in headers.split("\r\n").map(|line |parser.parse(line)?) {
            match header {
                Header::General(general) => { general_headers.push(general); }
                Header::Request(request) => { request_headers.push(request); }
                Header::Response(response) => { response_headers.push(response); }
                Header::Entity(entity) => { entity_headers.push(entity); }
            }
        }

        let request_headers = if request_headers.is_empty() { None } else { Some(Rc::from(request_headers.into_boxed_slice())) };
        let response_headers = if response_headers.is_empty() { None } else { Some(Rc::from(response_headers.into_boxed_slice())) };
        let entity_headers = if entity_headers.is_empty() { None } else { Some(Rc::from(entity_headers.into_boxed_slice())) };

        Ok(Self::new(
            Rc::from(general_headers.into_boxed_slice()),
            response_headers,
            request_headers,
            entity_headers,
        ))
    }

    //noinspection ALL
    pub fn location(&self) -> Option<&Path> {
        self.response_headers
            .iter()
            .filter_map(|header| if let ResponseHeader::Location(path) = header {
                Some(path.as_path())
            } else {
                None
            })
            .next()
    }

    //noinspection ALL
    pub fn host(&self) -> Option<(&str, Option<usize>)> {
        self.request_headers
            .iter()
            .filter_map(|header| if let RequestHeader::Host(host, port) = header {
                Some((host.as_str(), port.clone()))
            } else {
                None
            })
            .next()
    }

    pub fn content_length(&self) -> Option<usize> {
        self.entity_headers
            .iter()
            .filter_map(|header| if let EntityHeader::ContentLength(length) = header {
                Some(length)
            } else {
                None
            })
            .next()
    }

    pub fn content_type(&self) -> Option<ContentType> {
        self.entity_headers
            .iter()
            .filter_map(|header| if let EntityHeader::ContentType(ct) = header {
                Some(ct)
            } else {
                None
            })
            .next()
    }

    //noinspection ALL
    pub fn connection(&self) -> Option<ConnectionType> {
        self.general_headers
            .iter()
            .filter_map(|header| if let GeneralHeader::Connection(connection) = header {
                Some(connection)
            } else {
                None
            })
            .next()
    }
    // endregion
}

impl Display for Headers {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for header in self.general_headers {
            write!(f, "{}", header)?;
        }
        if let Some(headers) = &self.response_headers {
            for header in headers {
                write!(f, "{}", header)?;
            }
        }
        if let Some(headers) = &self.entity_headers {
            for header in headers {
                write!(f, "{}", header)?;
            }
        }
        std::fmt::Result::Ok(())
    }
}

// region Header Parser
pub trait HeaderParser : Default {
    fn parse(&self, line: &str) -> Result<Header, ParseHeaderError>;

    fn generic_parse(line: &str) -> Result<(&str, &str), InvalidHeaderFormatError> {
        if !line.ends_with("\r\n") {
            return Err(InvalidHeaderFormatError::CrlfMissing);
        }
        if let Some(index) = line.find(':') {
            return Ok(line.split_at(index));
        }
        Err(InvalidHeaderFormatError::ColonMissing)
    }
}

pub struct SimpleHeaderParser {
    supported_request_headers: HashSet<&'static str>,
    supported_general_headers: HashSet<&'static str>,
}

impl HeaderParser for SimpleHeaderParser {
    fn parse(&self, line: &str) -> Result<Header, ParseHeaderError> {
        let (name, value) = HeaderParser::generic_parse(line).map_err(ParseHeaderError::from)?;
        if self.supported_request_headers.contains(name) {
            return Ok(Header::Request(request_header::RequestHeader::parse(
                name, value,
            )?));
        }
        if self.supported_general_headers.contains(name) {
            return Ok(Header::General(general_header::GeneralHeader::parse(
                name, value,
            )?));
        }
        Err(ParseHeaderError::Unsupported(
            UnsupportedHeaderError::UnsupportedName(name.to_owned()),
        ))
    }
}

impl Default for SimpleHeaderParser {
    fn default() -> Self {
        let supported_request_headers =
            HashSet::from_iter(request_header::RequestHeader::SUPPORTED_HEADERS);
        let supported_general_headers =
            HashSet::from_iter(general_header::GeneralHeader::SUPPORTED_HEADERS);
        Self {
            supported_request_headers,
            supported_general_headers,
        }
    }
}
// endregion
