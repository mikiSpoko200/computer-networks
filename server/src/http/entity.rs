//! Mikołaj Depta 328690

use std::rc::Rc;
use super::headers::entity_header::{ContentType, EntityHeader, EntityHeaders};

pub struct Entity {
    data: Box<[u8]>,
    headers: EntityHeaders,
}

impl Entity {
    pub fn new(data: Box<[u8]>, content_type: ContentType) -> Self {
        let headers = Rc::from([
            EntityHeader::ContentType(content_type),
            EntityHeader::ContentLength(data.len()),
        ]).into_boxed_slice();
        Self { data, headers }
    }

    pub fn headers(&self) -> EntityHeaders {
        self.headers.clone()
    }

    pub fn not_found() -> Self {
        let data = String::from("Page not found").into_boxed_bytes();
        Self {
            data,
            headers: Rc::from([EntityHeader::ContentType(ContentType::Txt), EntityHeader::ContentLength(data.len())]).into_boxed_slice()
        }
    }

    pub fn morbidden() -> Self {
        let data = String::from("Access denied").into_boxed_bytes();
        Self { data, headers: Rc::from([EntityHeader::ContentType(ContentType::Txt), EntityHeader::ContentLength(data.len())]).into_boxed_slice() }
    }

    pub fn redirect() -> Self {
        let data = String::from("Redirecting...").into_boxed_bytes();
        Self { data, headers: Rc::from([EntityHeader::ContentType(ContentType::Txt), EntityHeader::ContentLength(data.len())]).into_boxed_slice() }
    }

    pub fn not_implemented() -> Self {
        let data = String::from("Unrecognized http message").into_boxed_bytes();
        Self { data, headers: Rc::from([EntityHeader::ContentType(ContentType::Txt), EntityHeader::ContentLength(data.len())]).into_boxed_slice() }
    }
}

impl AsRef<[u8]> for Entity {
    fn as_ref(&self) -> &[u8] {
        self.data.as_ref()
    }
}
