use std::time::Instant;

// --- Public types ---

#[derive(Debug, Clone, PartialEq)]
pub enum SessionEvent {
    Question {
        text: String,
        choices: Vec<String>,
        screen_snapshot: String,
    },
    WaitingForInput,
    Finished,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionStatus {
    Working,
    Question,
    Waiting,
    Finished,
    Stopped,
}

impl SessionStatus {
    #[must_use]
    pub fn needs_attention(self) -> bool {
        matches!(self, Self::Question | Self::Waiting)
    }

    #[must_use]
    pub fn is_pulsing(self) -> bool {
        matches!(self, Self::Question | Self::Waiting)
    }
}

// --- Detector ---

#[derive(Debug)]
pub struct SessionDetector {
    status: SessionStatus,
    trigger_pending: bool,
    last_output_at: Option<Instant>,
}

const IDLE_THRESHOLD_MS: u128 = 200;

impl Default for SessionDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionDetector {
    #[must_use]
    pub fn new() -> Self {
        Self {
            status: SessionStatus::Working,
            trigger_pending: false,
            last_output_at: None,
        }
    }

    #[must_use]
    pub fn status(&self) -> SessionStatus {
        self.status
    }

    pub fn on_output(&mut self, data: &[u8]) {
        self.last_output_at = Some(Instant::now());

        let stripped = strip_ansi_escapes::strip(data);
        let text = String::from_utf8_lossy(&stripped);
        let lower = text.to_lowercase();

        if scan_triggers(&lower) {
            self.trigger_pending = true;
        }
    }

    #[must_use]
    pub fn should_analyze(&self) -> bool {
        if !self.trigger_pending {
            return false;
        }
        match self.last_output_at {
            Some(t) => t.elapsed().as_millis() >= IDLE_THRESHOLD_MS,
            None => false,
        }
    }

    pub fn analyze_screen(&mut self, screen_text: &str) -> Option<SessionEvent> {
        self.trigger_pending = false;
        let lower = screen_text.to_lowercase();

        // Phase 2: status bar signals (highest priority)
        if lower.contains("esc to interrupt") {
            self.status = SessionStatus::Working;
            return None;
        }

        if lower.contains("enter to select") {
            let (question, choices) = extract_question_and_choices(screen_text);
            self.status = SessionStatus::Question;
            return Some(SessionEvent::Question {
                text: question,
                choices,
                screen_snapshot: screen_text.to_string(),
            });
        }

        // Yes/No prompt
        if has_yes_no_prompt(&lower) {
            let question = extract_yn_question(screen_text);
            self.status = SessionStatus::Question;
            return Some(SessionEvent::Question {
                text: question,
                choices: vec!["Yes".into(), "No".into()],
                screen_snapshot: screen_text.to_string(),
            });
        }

        // Numbered options (1. 2. 3.)
        let options = extract_numbered_options(screen_text);
        if options.len() >= 2 {
            let question = extract_question_before_options(screen_text);
            self.status = SessionStatus::Question;
            return Some(SessionEvent::Question {
                text: question,
                choices: options,
                screen_snapshot: screen_text.to_string(),
            });
        }

        // Fallback: idle without recognizable pattern = waiting
        self.status = SessionStatus::Waiting;
        Some(SessionEvent::WaitingForInput)
    }

    pub fn reset_to_working(&mut self) {
        self.status = SessionStatus::Working;
        self.trigger_pending = false;
    }

    pub fn mark_stopped(&mut self) {
        self.status = SessionStatus::Stopped;
        self.trigger_pending = false;
    }

    pub fn mark_finished(&mut self) {
        self.status = SessionStatus::Finished;
        self.trigger_pending = false;
    }
}

// --- Pattern scanning (Phase 1) ---

fn scan_triggers(lower: &str) -> bool {
    lower.contains("[y/n]")
        || lower.contains("[yes/no]")
        || lower.contains("enter to select")
        || lower.contains("? ")
        || lower.contains("(y/n)")
}

// --- Text extraction helpers ---

fn has_yes_no_prompt(lower: &str) -> bool {
    lower.contains("[y/n]")
        || lower.contains("[yes/no]")
        || lower.contains("(y/n)")
        || lower.contains("(yes/no)")
}

fn extract_yn_question(screen_text: &str) -> String {
    let lower = screen_text.to_lowercase();
    for line in screen_text.lines().rev() {
        let ll = line.to_lowercase();
        if ll.contains("[y/n]")
            || ll.contains("[yes/no]")
            || ll.contains("(y/n)")
            || ll.contains("(yes/no)")
        {
            // Strip the prompt suffix to get the question text
            let trimmed = line.trim();
            for pat in &["[Y/n]", "[y/N]", "[y/n]", "[Yes/No]", "[yes/no]", "(y/n)", "(yes/no)"] {
                if let Some(pos) = trimmed.find(pat) {
                    let q = trimmed[..pos].trim();
                    if !q.is_empty() {
                        return q.to_string();
                    }
                }
            }
            // Case-insensitive fallback
            for pat in &["[y/n]", "[yes/no]", "(y/n)", "(yes/no)"] {
                if let Some(pos) = lower.find(pat) {
                    let q = screen_text[..pos].trim();
                    // Take just the last line
                    return q.lines().last().unwrap_or(q).trim().to_string();
                }
            }
            return trimmed.to_string();
        }
    }
    String::new()
}

fn extract_question_and_choices(screen_text: &str) -> (String, Vec<String>) {
    let mut choices = Vec::new();
    let mut question_lines = Vec::new();
    let mut in_choices = false;

    for line in screen_text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        // Detect choice lines: start with ">" or "○" or "●" or "[ ]" / "[x]"
        if is_choice_line(trimmed) {
            in_choices = true;
            let choice = clean_choice_text(trimmed);
            if !choice.is_empty() {
                choices.push(choice);
            }
        } else if !in_choices {
            question_lines.push(trimmed);
        }
    }

    let question = question_lines
        .last()
        .unwrap_or(&"")
        .trim_end_matches('?')
        .trim()
        .to_string()
        + if question_lines.last().is_some_and(|l| l.ends_with('?')) {
            "?"
        } else {
            ""
        };

    (question, choices)
}

fn is_choice_line(line: &str) -> bool {
    line.starts_with("> ")
        || line.starts_with("○ ")
        || line.starts_with("● ")
        || line.starts_with("◯ ")
        || line.starts_with("[ ] ")
        || line.starts_with("[x] ")
        || line.starts_with("[X] ")
}

fn clean_choice_text(line: &str) -> String {
    let prefixes = ["> ", "○ ", "● ", "◯ ", "[ ] ", "[x] ", "[X] "];
    for prefix in &prefixes {
        if let Some(rest) = line.strip_prefix(prefix) {
            return rest.trim().to_string();
        }
    }
    line.trim().to_string()
}

fn extract_numbered_options(screen_text: &str) -> Vec<String> {
    let mut options = Vec::new();
    for line in screen_text.lines() {
        let trimmed = line.trim();
        if let Some(rest) = try_strip_number_prefix(trimmed) {
            if !rest.is_empty() {
                options.push(rest.to_string());
            }
        }
    }
    options
}

fn try_strip_number_prefix(line: &str) -> Option<&str> {
    let mut chars = line.chars();
    let first = chars.next()?;
    if !first.is_ascii_digit() {
        return None;
    }
    // Consume remaining digits
    let rest = chars.as_str();
    let after_digits = rest.trim_start_matches(|c: char| c.is_ascii_digit());
    // Expect ". " or ") " after the number
    if let Some(stripped) = after_digits.strip_prefix(". ") {
        Some(stripped.trim())
    } else if let Some(stripped) = after_digits.strip_prefix(") ") {
        Some(stripped.trim())
    } else {
        None
    }
}

fn extract_question_before_options(screen_text: &str) -> String {
    for line in screen_text.lines() {
        let trimmed = line.trim();
        if try_strip_number_prefix(trimmed).is_some() {
            break;
        }
        if !trimmed.is_empty() && (trimmed.ends_with('?') || trimmed.ends_with(':')) {
            return trimmed.to_string();
        }
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_needs_attention() {
        assert!(SessionStatus::Question.needs_attention());
        assert!(SessionStatus::Waiting.needs_attention());
        assert!(!SessionStatus::Working.needs_attention());
        assert!(!SessionStatus::Finished.needs_attention());
        assert!(!SessionStatus::Stopped.needs_attention());
    }

    #[test]
    fn scan_triggers_detects_yn() {
        assert!(scan_triggers("do you want to continue? [y/n]"));
        assert!(scan_triggers("proceed? (y/n)"));
        assert!(!scan_triggers("hello world"));
    }

    #[test]
    fn extract_yn_question_works() {
        let q = extract_yn_question("Do you want to proceed? [Y/n]");
        assert_eq!(q, "Do you want to proceed?");
    }

    #[test]
    fn extract_numbered_options_works() {
        let screen = "Pick one:\n1. Option A\n2. Option B\n3. Option C\n";
        let opts = extract_numbered_options(screen);
        assert_eq!(opts, vec!["Option A", "Option B", "Option C"]);
    }

    #[test]
    fn extract_numbered_options_parentheses() {
        let screen = "1) First\n2) Second\n";
        let opts = extract_numbered_options(screen);
        assert_eq!(opts, vec!["First", "Second"]);
    }

    #[test]
    fn extract_question_before_options_works() {
        let screen = "Which model to use?\n1. Claude\n2. GPT\n";
        let q = extract_question_before_options(screen);
        assert_eq!(q, "Which model to use?");
    }

    #[test]
    fn choice_line_detection() {
        assert!(is_choice_line("> Selected item"));
        assert!(is_choice_line("○ Unselected"));
        assert!(is_choice_line("● Selected"));
        assert!(is_choice_line("[ ] Unchecked"));
        assert!(is_choice_line("[x] Checked"));
        assert!(!is_choice_line("Normal text"));
    }

    #[test]
    fn analyze_screen_working_on_esc_to_interrupt() {
        let mut det = SessionDetector::new();
        det.trigger_pending = true;
        let result = det.analyze_screen("Some output\n esc to interrupt ");
        assert!(result.is_none());
        assert_eq!(det.status(), SessionStatus::Working);
    }

    #[test]
    fn analyze_screen_question_on_enter_to_select() {
        let mut det = SessionDetector::new();
        det.trigger_pending = true;
        let result = det.analyze_screen("Pick a tool:\n> Tool A\n○ Tool B\n enter to select ");
        assert!(matches!(result, Some(SessionEvent::Question { .. })));
        assert_eq!(det.status(), SessionStatus::Question);
    }

    #[test]
    fn analyze_screen_yn_prompt() {
        let mut det = SessionDetector::new();
        det.trigger_pending = true;
        let result = det.analyze_screen("Allow access? [Y/n]");
        match result {
            Some(SessionEvent::Question { text, choices, .. }) => {
                assert_eq!(text, "Allow access?");
                assert_eq!(choices, vec!["Yes", "No"]);
            }
            other => panic!("expected Question, got {other:?}"),
        }
        assert_eq!(det.status(), SessionStatus::Question);
    }

    #[test]
    fn analyze_screen_fallback_waiting() {
        let mut det = SessionDetector::new();
        det.trigger_pending = true;
        let result = det.analyze_screen("$ ");
        assert!(matches!(result, Some(SessionEvent::WaitingForInput)));
        assert_eq!(det.status(), SessionStatus::Waiting);
    }

    #[test]
    fn reset_to_working_clears_state() {
        let mut det = SessionDetector::new();
        det.status = SessionStatus::Question;
        det.trigger_pending = true;
        det.reset_to_working();
        assert_eq!(det.status(), SessionStatus::Working);
        assert!(!det.trigger_pending);
    }

    #[test]
    fn mark_stopped_sets_status() {
        let mut det = SessionDetector::new();
        det.mark_stopped();
        assert_eq!(det.status(), SessionStatus::Stopped);
    }

    #[test]
    fn mark_finished_sets_status() {
        let mut det = SessionDetector::new();
        det.mark_finished();
        assert_eq!(det.status(), SessionStatus::Finished);
    }

    #[test]
    fn on_output_sets_trigger_and_timestamp() {
        let mut det = SessionDetector::new();
        assert!(!det.trigger_pending);
        assert!(det.last_output_at.is_none());

        det.on_output(b"proceed? [y/n]");
        assert!(det.trigger_pending);
        assert!(det.last_output_at.is_some());
    }

    #[test]
    fn on_output_no_trigger_for_plain_text() {
        let mut det = SessionDetector::new();
        det.on_output(b"hello world");
        assert!(!det.trigger_pending);
    }
}
