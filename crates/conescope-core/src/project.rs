use serde::{Deserialize, Serialize};

pub const PROJECT_COLORS: &[&str] = &[
    "#E57373", "#64B5F6", "#81C784", "#FFB74D", "#BA68C8", "#4DD0E1", "#FFD54F", "#A1887F",
    "#90A4AE",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub path: String,
    pub display_name: String,
    pub color: String,
    pub created_at: String,
    pub last_used_at: String,
}
