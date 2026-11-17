pub mod catalogue;
pub mod exclusions;
pub mod libraries;
pub mod log;

pub enum Screen {
    Catalogue(catalogue::Catalogue),
    Unavailable(String),
}
