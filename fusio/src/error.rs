use std::io;

use thiserror::Error;

#[derive(Debug, Error)]
#[error(transparent)]
pub enum Error {
    Io(#[from] io::Error),
    #[cfg(feature = "http")]
    Http(#[from] http::Error),
    #[cfg(feature = "object_store")]
    ObjectStore(#[from] object_store::Error),
    Path(#[from] crate::path::Error),
    #[error("unsupported operation")]
    Unsupported,
    #[error(transparent)]
    Other(BoxError),
}

#[allow(unused)]
pub type BoxError = Box<dyn std::error::Error + Send + Sync + 'static>;
