use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Type, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Spawning,
    Running,
    NeedsInput,
    Idle,
    Exited,
}
