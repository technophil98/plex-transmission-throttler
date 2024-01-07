use std::fmt::{Display, Formatter};

use serde::Deserialize;
use serde_enum_str::Deserialize_enum_str;
use strum::{AsRefStr, Display};

pub const UNTHROTTLED_STREAM_LOCATIONS: &[StreamLocation] = &[StreamLocation::Lan];

#[derive(Debug, PartialEq, Deserialize, Display)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    Play,
    Pause,
    Resume,
    Stop,
}

#[derive(Debug, PartialEq, Deserialize_enum_str, AsRefStr)]
#[serde(rename_all = "lowercase")]
pub enum StreamLocation {
    Lan,
    Wan,
    Cellular,
    #[serde(other)]
    Other(String),
}

impl Display for StreamLocation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            StreamLocation::Other(o) => write!(f, "{}", o),
            location => write!(f, "{}", location.as_ref()),
        }
    }
}
