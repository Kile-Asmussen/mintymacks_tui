use std::path::PathBuf;

use serde::{Deserialize, Serialize};


#[derive(Serialize, Deserialize, Debug)]
pub struct Settings {
    engines: Vec<(String, PathBuf)>,
}