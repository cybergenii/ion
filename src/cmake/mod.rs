pub mod config_gen;
pub mod generator;

pub use config_gen::generate_config_file;
pub use generator::CmakeGenerator;
