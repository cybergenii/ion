pub mod config_gen;
pub mod generator;

pub use generator::CmakeGenerator;
pub use config_gen::generate_config_file;
