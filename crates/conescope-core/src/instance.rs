use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InstanceStatus {
    Starting,
    Working,
    Waiting,
    Paused,
    Stopped,
}

impl InstanceStatus {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Starting => "starting",
            Self::Working => "working",
            Self::Waiting => "waiting",
            Self::Paused => "paused",
            Self::Stopped => "stopped",
        }
    }

    #[must_use]
    pub fn from_str_opt(s: &str) -> Option<Self> {
        match s {
            "starting" => Some(Self::Starting),
            "working" => Some(Self::Working),
            "waiting" => Some(Self::Waiting),
            "paused" => Some(Self::Paused),
            "stopped" => Some(Self::Stopped),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InstanceType {
    Project,
    Terminal,
}

impl InstanceType {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Project => "project",
            Self::Terminal => "terminal",
        }
    }

    #[must_use]
    pub fn from_str_opt(s: &str) -> Option<Self> {
        match s {
            "project" => Some(Self::Project),
            "terminal" => Some(Self::Terminal),
            _ => None,
        }
    }
}

/// Which terminal tab is active in focus mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TerminalTab {
    /// Claude CLI terminal (default for Project instances) or the primary terminal.
    #[default]
    Primary,
    /// Secondary shell terminal (spawned on demand for Project instances).
    Shell,
}

/// Partial update for an instance. Only non-None fields are written.
#[derive(Debug, Clone, Default)]
pub struct InstanceUpdate {
    pub title: Option<String>,
    pub status: Option<InstanceStatus>,
    pub tokens_used: Option<i64>,
    pub cost_estimate: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instance {
    pub id: String,
    pub project_id: Option<String>,
    pub title: Option<String>,
    pub status: InstanceStatus,
    pub instance_number: Option<i64>,
    pub tokens_used: i64,
    pub cost_estimate: f64,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub instance_type: InstanceType,
    pub color: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_roundtrip() {
        let statuses = [
            InstanceStatus::Starting,
            InstanceStatus::Working,
            InstanceStatus::Waiting,
            InstanceStatus::Paused,
            InstanceStatus::Stopped,
        ];
        for s in statuses {
            let str_val = s.as_str();
            let parsed = InstanceStatus::from_str_opt(str_val).unwrap();
            assert_eq!(s, parsed);
        }
    }

    #[test]
    fn type_roundtrip() {
        let types = [InstanceType::Project, InstanceType::Terminal];
        for t in types {
            let str_val = t.as_str();
            let parsed = InstanceType::from_str_opt(str_val).unwrap();
            assert_eq!(t, parsed);
        }
    }

    #[test]
    fn unknown_status_returns_none() {
        assert!(InstanceStatus::from_str_opt("invalid").is_none());
    }

    #[test]
    fn unknown_type_returns_none() {
        assert!(InstanceType::from_str_opt("invalid").is_none());
    }
}
