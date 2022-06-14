//! Mikołaj Depta 328690


use std::cmp::Ordering;
use std::io;
use std::io::{Read, Write, BufWriter, BufReader};
use std::net::{TcpListener, TcpStream, SocketAddr};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Duration;
use crate::http::common::Body;
use crate::http::headers::{general_header::GeneralHeader, Headers, response_header::ResponseHeader};
use crate::http::request::{Request, RequestMetaData};
use crate::http::response::{Response, StatusCode, StatusLine};
use crate::http::entity::Entity;
use crate::http::headers::entity_header::ContentType;
use crate::http::headers::response_header::ResponseHeaders;

use crate::resources::{StaticValidator, StaticLoader, ResourceLoader, ResourceValidator, ValidationResourceError};
use crate::registry::{Registry, TimeoutDuration};
use crate::util::OrFailWithMessage;


pub struct HttpServer<D, S, L = StaticLoader, V = StaticValidator,>
where
    D: Downloader,
    S: Sender,
    L: ResourceLoader,
    V: ResourceValidator,
{
    address: SocketAddr,
    loader: L,
    validator: V,
    listener: TcpListener,
    registry: Registry,
    catalog: Rc<Path>,
    connections: Vec<Connection<D, S>>,
}

impl<D, S, L, V> HttpServer<D, S, L, V>
where
    D: Downloader,
    S: Sender,
    L: ResourceLoader,
    V: ResourceValidator,
{
    const MAX_CONNECTIONS: usize = 1;

    pub fn new(address: SocketAddr, dir: Rc<Path>) -> Self {
        let listener = TcpListener::bind(address)
            .or_fail_with_message(format!("could not bind tcp socket to {}", address).as_str());
        let loader = StaticLoader::new(dir.clone());
        let validator = StaticValidator::default_config(dir.clone());
        let registry = Registry::new()
            .or_fail_with_message("could not create an epoll event queue");
        Self { address, loader, validator, listener, registry, catalog: dir, connections: Vec::new() }
    }

    fn connection_limit_exceeded(&self) -> bool {
        self.connections.len() < Self::MAX_CONNECTIONS
    }

    fn handle_request(&mut self, request: &Request) -> Response {
        let domain = request.host();
        let resource_path = request.start_line().url();
        let mut full_resource_path = PathBuf::from(&self.catalog);

        let http_version = request.start_line().version().clone();

        full_resource_path.push(domain);
        full_resource_path.push(resource_path);
        match self.validator.validate(&full_resource_path) {
            Ok(_) => {
                match self.loader.load(&full_resource_path) {
                    Ok(data) => {
                        let status_line = StatusLine::new(http_version, StatusCode::Ok);
                        let entity = Entity::new(data, request.headers().content_type().unwrap_or_default());
                        let headers = Headers::new(
                            request.headers().general_headers(),
                            None,
                            None,
                            Some(entity.headers()),
                        );
                        Response::new(status_line, headers, Some(Body::SingleSource(entity)))
                    }
                    Err(_) => {
                        let status_line = StatusLine::new(http_version, StatusCode::NotFound);
                        let entity = Entity::not_found();
                        let headers = Headers::new(
                            request.headers().general_headers(),
                            None,
                            None,
                            Some(entity.headers()),
                        );
                        Response::new(status_line, headers, Some(Body::SingleSource(entity)))
                    }
                }
            }
            Err(ValidationResourceError::UnauthorizedResourceAccess(_)) => {
                // prepare 403 message
                let status_line = StatusLine::new(http_version, StatusCode::Forbidden);
                let entity = Entity::morbidden();
                let headers = Headers::new(
                    request.headers().general_headers(),
                    None,
                    None,
                    Some(entity.headers()),
                );
                Response::new(status_line, headers, Some(Body::SingleSource(entity)))
            }
            Err(ValidationResourceError::OutdatedResourcePath(path)) => {
                // prepare 301 message
                let status_line = StatusLine::new(http_version, StatusCode::MovedPermanently);
                let entity = Entity::redirect();
                let mut new_path = PathBuf::from(path);
                new_path.push("/index.html");
                let headers = Headers::new(
                    request.headers().general_headers(),
                    None,
                    Some(ResponseHeaders::from([ResponseHeader::Location(new_path)])),
                    Some(entity.headers()),
                );
                Response::new(status_line, headers, Some(Body::SingleSource(entity)))
            }
            other => { panic!("some wierd edge case") }
        }
    }
    
    pub fn process_connections(&mut self) { 
        for connection in self.connections {

        }
    }

    pub fn start(&mut self) { }
}


/// Abstraction of action that can be performed by `HttpConnection`.
///
/// Action is will be injected into `HttpConnection` and will control the process of
/// downloading a Request and sending a Response.
///
/// `advance` will be called continuously until `is_finished` returns `true`.
pub trait Action {
    type Output;
    
    fn advance(&mut self) -> io::Result<Self::Output>;

    fn is_finished(&self) -> bool;
    
    fn timeout(&self) -> &TimeoutDuration;
}

pub trait Downloader : Action<Output=Option<Request>> { }

pub trait Sender : Action<Output=()> { }


// region Downloader
/// Provides functionality of downloading HTTP Request until end of header section.
/// HTTP Entity event if present will be ignored.
struct HttpDownloader<R> where R: Read {
    reader: BufReader<R>,
    timeout: TimeoutDuration,
    store: Vec<u8>,
    download_buffer: Vec<u8>,
    is_finished: bool,
    request_metadata: Option<RequestMetaData>,
    content_length: Option<usize>,
    body: Option<Body>,
}

impl<R> HttpDownloader<R> where R: Read {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            timeout: TimeoutDuration::Infinite,
            store: Vec::new(),
            download_buffer: Vec::new(),
            is_finished: false,
            request_metadata: None,
            content_length: None,
            body: None
        }
    }

    pub fn reset(&mut self, reader: R) {
        self.reader = BufReader::new(reader);
        self.download_buffer.clear();
        self.store.clear();
        self.is_finished = false;
        self.request_metadata = None;
        self.content_length = None;
        self.body = None;
    }
}

impl<R> HttpDownloader<R> where R: Read {
    fn download_metadata(&mut self) -> io::Result<()> {
        loop {
            self.download_buffer.clear();
            match self.reader.read(&mut self.download_buffer)? {
                0 => Ok(()),
                bytes_read=> {
                    // section separator can be downloaded in two separate messages eg:
                    // [.., b"\r", b"\n", b"\r"]
                    // [b"\n", ..]
                    // so we must first add data to the store buffer, check if stored section sep
                    // and move excessive data.
                    let prev_store_len = self.store.len();
                    let sep_range = prev_store_len.saturating_sub(3)..prev_store_len + bytes_read;
                    self.store.extend_from_slice(&self.download_buffer);
                    if let Some(pos) = Request::section_sep_pos(&self.store[sep_range]) {
                        /* addition of Request::SECTION_SEP.len() / 2 adds CRLF at the end, final header wouldn't be valid otherwise.  */
                        self.store.resize(prev_store_len.saturating_sub(3) + pos + Request::SECTION_SEP.len() / 2, 0);
                        self.download_buffer.drain(..pos + Request::SECTION_SEP.len() - 3);
                        self.request_metadata = Some(RequestMetaData::try_from(&self.store)?);
                        
                        let metadata_ref = &self.request_metadata.unwrap();
                        if let Some(content_length) = metadata_ref.headers.content_length() {
                            /* once metadata section was parsed store can be reused for payload download. */
                            self.store.clear();
                            /* move all the remaining bytes from download buffer into store */
                            let payload_range = 0..self.download_buffer.len().min(content_length);
                            self.store.extend(self.download_buffer.drain(payload_range));
                        } else {
                            /* if no content-type information is present request is ready */
                            self.is_finished = true;
                        }
                        Ok(())
                    }
                }
            }
        }
    }

    fn download_payload(&mut self) -> io::Result<()> {
        loop {
            self.download_buffer.clear();
            match self.reader.read(&mut self.download_buffer)? {
                0 => Ok(()),
                _ => {
                    self.store.extend_from_slice(&self.download_buffer);
                    match (self.store.len()).cmp(&self.content_length.unwrap()) {
                        Ordering::Less => {
                            continue;
                        }
                        _ => {
                            let overhead = self.store.len() - self.content_length.unwrap();
                            self.body = Some(Body::SingleSource(
                                Entity::new(
                                    Vec::from(&self.store[..overhead]).into_boxed_slice(),
                                    self.request_metadata.unwrap().headers.content_type().unwrap_or_default(),
                                )
                            ));
                            self.is_finished = true;
                        }
                    }
                    Ok(())
                }
            }
        }
    }
}

impl<R> Action for HttpDownloader<R> where R: Read {
    type Output = Option<Request>;
    
    fn advance(&mut self) -> io::Result<Self::Output> {
        if !self.is_finished {
            if self.request_metadata.is_none() {
                self.download_metadata()?;
            } else {
                self.download_payload()?;
            }
        } else {
            let RequestMetaData { start_line, headers } = self.request_metadata.take().unwrap();
            Ok(Some(Request::new(start_line, headers, self.body.take())))
        }
        Ok(None)
    }

    fn is_finished(&self) -> bool {
        self.is_finished
    }

    fn timeout(&self) -> &TimeoutDuration {
        &self.timeout
    }
}

impl<R> Downloader for HttpDownloader<R> where R: Read { }
// endregion


// region Sender
struct HttpSender<W> where W: Write {
    writer: BufWriter<W>,
    data: Box<[u8]>,
    timeout: TimeoutDuration,
    bytes_sent: usize,
    is_finished: bool,
}

impl<W> HttpSender<W> where W: Write {
    pub fn new(writer: W, data: Box<[u8]>) -> Self {
        Self {
            writer,
            data,
            timeout: TimeoutDuration::Infinite,
            bytes_sent: 0,
            is_finished: false,
        }
    }
}

impl<W> Action for HttpSender<W> where W: Write {
    type Output = ();
    
    fn advance(&mut self) -> io::Result<Self::Output> {
        while self.bytes_sent < self.data.len() {
            let bytes_written = self.writer.write(self.bytes_sent[self.bytes_sent..])?;
            self.bytes_sent += bytes_written;
            if self.bytes_sent == self.data.len() {
                self.is_finished = true;
            }
        }
        Ok(())
    }

    fn is_finished(&self) -> bool {
        self.is_finished
    }

    fn timeout(&self) -> &TimeoutDuration {
        &self.timeout
    }
}

impl<W> Sender for HttpSender<W> where W: Write { }
// endregion


// region Connection
pub enum ActionStatus {
    DownloadPending,
    DownloadFinished,
    SendPending,
    SendFinished,
}

pub struct Connection<D, S>
where
    D: Downloader,
    S: Sender,
{
    tcp_stream: TcpStream,
    status: ActionStatus,
    pub downloader: D,
    pub sender: S,
}

impl<D, S> Connection<D, S>
where
    D: Downloader,
    S: Sender,
{
    const STALE_CONNECTION_TIMEOUT: TimeoutDuration = TimeoutDuration::Finite(Duration::from_millis(500));

    pub fn new(mut tcp_stream: TcpStream, downloader: D, sender: S) -> Self {
        tcp_stream.set_nonblocking(true).unwrap();
        Self {
            tcp_stream,
            status: ActionStatus::DownloadPending,
            downloader,
            sender
        }
    }

    pub fn yield_resources(self) -> (D, S) {
        let Self { downloader, sender, .. } = self;
        (downloader, sender)
    }

    pub fn timeout(&self) -> &TimeoutDuration {
        match self.status {
            ActionStatus::Pending => self.action.timeout(),
            ActionStatus::Finished => &Self::STALE_CONNECTION_TIMEOUT
        }
    }

    pub fn advance_send(&mut self) -> io::Result<()> {
        self.sender.advance()
    }

    pub fn advance_download(&mut self) -> io::Result<Option<Request>> {
        self.downloader.advance()
    }
}
// endregion
