//! Mikołaj Depta 328690
//!
//! Abstractions for working with server resources.

use crate::util;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum LoadResourceError {
    NotFound(PathBuf),
}

pub trait ResourceLoader {
    type LoadError;

    fn load(&self, resource: &Path) -> Result<Box<[u8]>, Self::LoadError>;
}

pub struct StaticLoader {
    catalog: Rc<Path>,
}

impl StaticLoader {
    pub fn new(catalog: Rc<Path>) -> Self {
        Self { catalog }
    }
}

impl ResourceLoader for StaticLoader {
    type LoadError = LoadResourceError;

    fn load(&self, resource: &Path) -> Result<Box<[u8]>, Self::LoadError> {
        use std::io::ErrorKind;
        match fs::read(self.catalog.join(resource)) {
            Ok(data) => Ok(data.into_boxed_slice()),
            Err(err) if err.kind() == ErrorKind::NotFound => {
                Err(LoadResourceError::NotFound(resource.to_owned()))
            }
            Err(err) => util::fail_with_message(err.to_string().as_ref()),
        }
    }
}

#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum ValidationResourceError {
    OutdatedResourcePath(PathBuf),
    UnauthorizedResourceAccess(PathBuf),
}

pub trait ResourceValidator {
    type ValidationError;

    fn validate(&self, resource_path: &Path) -> Result<(), Self::ValidationError>;
}

pub type Domains = Rc<HashSet<PathBuf>>;

pub struct StaticValidator {
    catalog: Rc<Path>,
    domains: Domains,
}

impl StaticValidator {
    pub fn new(catalog: Rc<Path>, domains: Domains) -> Self {
        Self { catalog, domains }
    }

    pub fn default_config(catalog: Rc<Path>) -> Self {
        let mut base_dir = PathBuf::from(&catalog);
        let directories = HashSet::from(
            ["localhost", "lab108-18"].map(|domain| {
                let mut domain_dir = base_dir.clone();
                domain_dir.push(domain);
                domain_dir
            })
        );
        Self { catalog, domains: Rc::new(directories) }
    }
}

impl ResourceValidator for StaticValidator {
    type ValidationError = ValidationResourceError;

    fn validate(&self, resource_path: &Path) -> Result<(), Self::ValidationError>  {
        if self.domains.contains(resource_path) {
            return Err(ValidationResourceError::OutdatedResourcePath(
                resource_path.to_owned(),
            ));
        }
        return match resource_path.canonicalize() {
            Ok(absolute_path) => {
                for domain in self.domains {
                    let mut domain_path = self.catalog.clone();
                    domain_path.push(domain);
                    if absolute_path.starts_with(domain_path) {
                        return Ok(());
                    }
                }
                Err(ValidationResourceError::UnauthorizedResourceAccess(
                    resource_path.to_owned(),
                ))
            }
            Err(_) => {
                Err(ValidationResourceError::UnauthorizedResourceAccess(
                    resource_path.to_owned(),
                ))
            }
        }
    }
}
