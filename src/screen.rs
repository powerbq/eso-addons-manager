pub mod catalogue;
pub mod exclusions;
pub mod libraries;
pub mod log;
pub mod patches;

pub enum Screen {
    Catalogue(catalogue::Catalogue),
    Unavailable(String),
}
