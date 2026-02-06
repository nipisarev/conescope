use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Question {
    pub id: String,
    pub instance_id: String,
    pub question_text: String,
    pub context: Option<String>,
    pub asked_at: String,
    pub answered_at: Option<String>,
    pub answer: Option<String>,
    pub snoozed_until: Option<String>,
}

/// Question joined with instance/project context for display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionWithContext {
    #[serde(flatten)]
    pub question: Question,
    pub instance_title: Option<String>,
    pub project_name: Option<String>,
    pub project_color: Option<String>,
}
