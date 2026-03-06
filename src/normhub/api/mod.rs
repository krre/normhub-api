pub mod endpoint;
pub mod error;
pub mod extract;
pub mod middleware;
pub mod router;

pub use error::Error;

pub type Result<T> = std::result::Result<T, Error>;
