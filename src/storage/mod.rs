pub(crate) mod cozo;
mod migrations;

pub use self::cozo::CozoStorage;
pub(crate) use migrations::SCHEMA_MIGRATIONS;
