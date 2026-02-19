use std::time::Instant;

// --- Public types ---

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PromptType {
    YesNo,
    NumberedList,
    ArrowSelect,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SessionEvent {
    Question {
        text: String,
        choices: Vec<String>,
        prompt_type: PromptType,
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
    event: Option<SessionEvent>,
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
            event: None,
            trigger_pending: true,
            last_output_at: Some(Instant::now()),
        }
    }

    #[must_use]
    pub fn status(&self) -> SessionStatus {
        self.status
    }

    #[must_use]
    pub fn event(&self) -> Option<&SessionEvent> {
        self.event.as_ref()
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
        if self.trigger_pending {
            return self
                .last_output_at
                .is_some_and(|t| t.elapsed().as_millis() >= IDLE_THRESHOLD_MS);
        }
        // Re-analyze if Working but silent for 2+ seconds
        // (screen may have transitioned to idle after trigger was consumed)
        self.status == SessionStatus::Working
            && self
                .last_output_at
                .is_some_and(|t| t.elapsed().as_secs() >= 2)
    }

    pub fn analyze_screen(&mut self, screen_text: &str) -> Option<SessionEvent> {
        self.trigger_pending = false;
        let lower = screen_text.to_lowercase();

        // Phase 2: status bar signals (highest priority)
        if lower.contains("esc to interrupt") {
            self.status = SessionStatus::Working;
            self.event = None;
            #[cfg(debug_assertions)]
            log_screen_dump(screen_text, self.status);
            return None;
        }

        // Claude CLI permission prompt (esc to cancel / tab to amend in status bar)
        if lower.contains("esc to cancel") || lower.contains("tab to amend") {
            let (question, choices) = extract_permission_prompt(screen_text);
            self.status = SessionStatus::Question;
            let evt = SessionEvent::Question {
                text: question,
                choices,
                prompt_type: PromptType::ArrowSelect,
                screen_snapshot: screen_text.to_string(),
            };
            self.event = Some(evt.clone());
            #[cfg(debug_assertions)]
            log_screen_dump(screen_text, self.status);
            return Some(evt);
        }

        if lower.contains("enter to select") {
            let (question, choices) = extract_question_and_choices(screen_text);
            self.status = SessionStatus::Question;
            let evt = SessionEvent::Question {
                text: question,
                choices,
                prompt_type: PromptType::ArrowSelect,
                screen_snapshot: screen_text.to_string(),
            };
            self.event = Some(evt.clone());
            #[cfg(debug_assertions)]
            log_screen_dump(screen_text, self.status);
            return Some(evt);
        }

        // Claude idle state (? for shortcuts hint)
        if lower.contains("? for shortcuts") {
            self.status = SessionStatus::Waiting;
            let evt = SessionEvent::WaitingForInput;
            self.event = Some(evt.clone());
            #[cfg(debug_assertions)]
            log_screen_dump(screen_text, self.status);
            return Some(evt);
        }

        // Yes/No prompt
        if has_yes_no_prompt(&lower) {
            let question = extract_yn_question(screen_text);
            self.status = SessionStatus::Question;
            let evt = SessionEvent::Question {
                text: question,
                choices: vec!["Yes".into(), "No".into()],
                prompt_type: PromptType::YesNo,
                screen_snapshot: screen_text.to_string(),
            };
            self.event = Some(evt.clone());
            #[cfg(debug_assertions)]
            log_screen_dump(screen_text, self.status);
            return Some(evt);
        }

        // Numbered options (1. 2. 3.)
        let options = extract_numbered_options(screen_text);
        if options.len() >= 2 {
            let question = extract_question_before_options(screen_text);
            self.status = SessionStatus::Question;
            let evt = SessionEvent::Question {
                text: question,
                choices: options,
                prompt_type: PromptType::NumberedList,
                screen_snapshot: screen_text.to_string(),
            };
            self.event = Some(evt.clone());
            #[cfg(debug_assertions)]
            log_screen_dump(screen_text, self.status);
            return Some(evt);
        }

        // Claude CLI idle state: diff stats line "N files +X -Y" on screen
        let has_diff_stats = screen_text
            .lines()
            .any(|line| is_diff_stats_line(line.trim()));
        if has_diff_stats {
            self.status = SessionStatus::Waiting;
            let evt = SessionEvent::WaitingForInput;
            self.event = Some(evt.clone());
            #[cfg(debug_assertions)]
            log_screen_dump(screen_text, self.status);
            return Some(evt);
        }

        // No recognized pattern — stay in current status
        None
    }

    pub fn request_analysis(&mut self) {
        self.trigger_pending = true;
        self.last_output_at = Some(Instant::now());
    }

    pub fn reset_to_working(&mut self) {
        self.status = SessionStatus::Working;
        self.event = None;
        self.trigger_pending = false;
    }

    pub fn mark_stopped(&mut self) {
        self.status = SessionStatus::Stopped;
        self.event = None;
        self.trigger_pending = false;
    }

    pub fn mark_finished(&mut self) {
        self.status = SessionStatus::Finished;
        self.event = None;
        self.trigger_pending = false;
    }
}

// --- Debug screen dump logging ---

#[cfg(debug_assertions)]
fn log_screen_dump(screen_text: &str, status: SessionStatus) {
    use std::fs;
    use std::path::PathBuf;

    let home = std::env::var("HOME").unwrap_or_default();
    let dir = PathBuf::from(home).join(".conescope").join("screen_dumps");
    let _ = fs::create_dir_all(&dir);

    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let filename = format!("{ts}_{status:?}.txt");

    let last_lines: String = screen_text
        .lines()
        .rev()
        .take(2)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("\n");

    let content = format!(
        "=== Status: {status:?} ===\n\n--- Last 2 lines ---\n{last_lines}\n\n--- Full screen ---\n{screen_text}\n",
    );
    let _ = fs::write(dir.join(filename), content);
}

// --- Pattern scanning (Phase 1) ---

fn scan_triggers(lower: &str) -> bool {
    lower.contains("[y/n]")
        || lower.contains("[yes/no]")
        || lower.contains("enter to select")
        || lower.contains("(y/n)")
        || lower.contains("esc to cancel")
        || lower.contains("tab to amend")
        || lower.contains("? for shortcuts")
        || lower.contains("esc to interrupt")
        || lower.contains("file +")
        || lower.contains("files +")
}

fn is_diff_stats_line(line: &str) -> bool {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 4 {
        return false;
    }
    if !parts[0].chars().all(|c| c.is_ascii_digit()) {
        return false;
    }
    if parts[1] != "file" && parts[1] != "files" {
        return false;
    }
    if !parts[2].starts_with('+')
        || parts[2].len() < 2
        || !parts[2][1..].chars().all(|c| c.is_ascii_digit())
    {
        return false;
    }
    if !parts[3].starts_with('-')
        || parts[3].len() < 2
        || !parts[3][1..].chars().all(|c| c.is_ascii_digit())
    {
        return false;
    }
    true
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
            for pat in &[
                "[Y/n]", "[y/N]", "[y/n]", "[Yes/No]", "[yes/no]", "(y/n)", "(yes/no)",
            ] {
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
    let mut expected_num = 1u32;
    for line in screen_text.lines() {
        let trimmed = line.trim();
        if let Some((num, text)) = try_strip_number_prefix_with_num(trimmed) {
            if num == expected_num && !text.is_empty() {
                options.push(text.to_string());
                expected_num += 1;
            } else {
                // Non-sequential or unexpected number — not an options list
                break;
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

fn try_strip_number_prefix_with_num(line: &str) -> Option<(u32, &str)> {
    let mut chars = line.chars();
    let first = chars.next()?;
    if !first.is_ascii_digit() {
        return None;
    }
    // Collect all digits
    let rest = chars.as_str();
    let digit_end = rest
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(rest.len());
    let num_str = &line[..=digit_end];
    let num: u32 = num_str.parse().ok()?;
    let after_digits = &rest[digit_end..];
    if let Some(stripped) = after_digits.strip_prefix(". ") {
        Some((num, stripped.trim()))
    } else if let Some(stripped) = after_digits.strip_prefix(") ") {
        Some((num, stripped.trim()))
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

// --- Permission prompt extraction (Claude CLI) ---

fn strip_cursor_prefix(line: &str) -> &str {
    let trimmed = line.trim_start();
    if let Some(rest) = trimmed.strip_prefix("❯ ") {
        return rest;
    }
    if let Some(rest) = trimmed.strip_prefix("> ") {
        return rest;
    }
    trimmed
}

fn extract_question_above_choices_from_lines(lines: &[&str], first_choice_idx: usize) -> String {
    for i in (0..first_choice_idx).rev() {
        let trimmed = lines[i].trim();
        if !trimmed.is_empty() && trimmed.ends_with('?') {
            return trimmed.to_string();
        }
    }
    // Fallback: use the first non-empty line before choices
    for i in (0..first_choice_idx).rev() {
        let trimmed = lines[i].trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    String::new()
}

const STATUS_BAR_PHRASES: &[&str] = &["esc to cancel", "tab to amend", "ctrl+e", "enter to select"];

fn is_status_bar_fragment(s: &str) -> bool {
    let lower = s.to_lowercase();
    let trimmed = lower.trim();
    STATUS_BAR_PHRASES.iter().any(|p| trimmed.contains(p))
}

fn extract_permission_prompt(screen_text: &str) -> (String, Vec<String>) {
    let lines: Vec<&str> = screen_text.lines().collect();

    // Search bottom-up for a line containing numbered choices (inline or standalone)
    for (idx, line) in lines.iter().enumerate().rev() {
        let stripped = strip_cursor_prefix(line);
        if stripped.is_empty() {
            continue;
        }

        // Check for inline choices separated by " / " (e.g. "1. Yes / 2. No / Esc to cancel")
        if stripped.contains(" / ") {
            let parts: Vec<&str> = stripped.split(" / ").collect();
            let choices: Vec<String> = parts
                .iter()
                .filter_map(|p| {
                    let t = p.trim();
                    // Keep only numbered choices like "1. Yes"
                    if try_strip_number_prefix(t).is_some() && !is_status_bar_fragment(t) {
                        // Remove the "· " separator remnants
                        let clean = t.split('·').next().unwrap_or(t).trim();
                        Some(clean.to_string())
                    } else {
                        None
                    }
                })
                .collect();

            if choices.len() >= 2 {
                let question = extract_question_above_choices_from_lines(&lines, idx);
                return (question, choices);
            }
        }

        // Check for standalone numbered choice line
        if try_strip_number_prefix(stripped).is_some() {
            // Find the first numbered line in this run
            let mut first = idx;
            for i in (0..idx).rev() {
                let s = strip_cursor_prefix(lines[i]);
                if try_strip_number_prefix(s).is_some() {
                    first = i;
                } else {
                    break;
                }
            }
            let choices: Vec<String> = (first..=idx)
                .filter_map(|i| {
                    let s = strip_cursor_prefix(lines[i]);
                    try_strip_number_prefix(s).map(str::to_string)
                })
                .collect();
            if choices.len() >= 2 {
                let question = extract_question_above_choices_from_lines(&lines, first);
                return (question, choices);
            }
        }
    }

    // Fallback: no choices found, just return the whole text as question
    (screen_text.trim().to_string(), vec![])
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
    fn analyze_screen_fallback_stays_current_status() {
        let mut det = SessionDetector::new();
        det.trigger_pending = true;
        let result = det.analyze_screen("$ ");
        assert!(result.is_none());
        assert_eq!(det.status(), SessionStatus::Working);
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
        // new() starts with trigger_pending=true for startup analysis; reset to test on_output
        det.trigger_pending = false;
        det.last_output_at = None;

        det.on_output(b"proceed? [y/n]");
        assert!(det.trigger_pending);
        assert!(det.last_output_at.is_some());
    }

    #[test]
    fn on_output_no_trigger_for_plain_text() {
        let mut det = SessionDetector::new();
        // new() starts with trigger_pending=true for startup analysis; reset to test on_output
        det.trigger_pending = false;
        det.on_output(b"hello world");
        assert!(!det.trigger_pending);
    }

    #[test]
    fn scan_triggers_detects_question_mark_shortcut_hint() {
        assert!(scan_triggers("? for shortcuts"));
        assert!(!scan_triggers("press ? for help"));
    }

    #[test]
    fn extract_numbered_options_non_sequential_returns_empty() {
        let screen = "3. Third\n5. Fifth\n";
        let opts = extract_numbered_options(screen);
        assert!(opts.is_empty());
    }

    #[test]
    fn extract_numbered_options_not_starting_from_1_returns_empty() {
        let screen = "2. Second\n3. Third\n";
        let opts = extract_numbered_options(screen);
        assert!(opts.is_empty());
    }

    #[test]
    fn scan_triggers_detects_claude_cli_patterns() {
        assert!(scan_triggers("esc to cancel"));
        assert!(scan_triggers("tab to amend"));
        assert!(scan_triggers("esc to interrupt"));
    }

    #[test]
    fn analyze_screen_question_on_esc_to_cancel() {
        let mut det = SessionDetector::new();
        det.trigger_pending = true;
        let screen = "Do you want to proceed?\n❯ 1. Yes / 2. No\n Esc to cancel · Tab to amend";
        let result = det.analyze_screen(screen);
        assert!(matches!(
            result,
            Some(SessionEvent::Question {
                prompt_type: PromptType::ArrowSelect,
                ..
            })
        ));
        assert_eq!(det.status(), SessionStatus::Question);
        if let Some(SessionEvent::Question { text, choices, .. }) = result {
            assert!(
                text.contains("proceed"),
                "question should contain 'proceed', got: {text}"
            );
            assert!(
                choices.len() >= 2,
                "should have at least 2 choices, got: {choices:?}"
            );
        }
    }

    #[test]
    fn analyze_screen_waiting_on_shortcuts_hint() {
        let mut det = SessionDetector::new();
        det.trigger_pending = true;
        let result = det.analyze_screen("claude> \n? for shortcuts");
        assert!(matches!(result, Some(SessionEvent::WaitingForInput)));
        assert_eq!(det.status(), SessionStatus::Waiting);
    }

    #[test]
    fn analyze_screen_esc_to_interrupt_beats_esc_to_cancel() {
        let mut det = SessionDetector::new();
        det.trigger_pending = true;
        let result = det.analyze_screen("Working...\n esc to interrupt \n esc to cancel ");
        assert!(result.is_none());
        assert_eq!(det.status(), SessionStatus::Working);
    }

    #[test]
    fn extract_permission_prompt_basic() {
        let screen = "Do you want to proceed?\n❯ 1. Yes / 2. No\n Esc to cancel · Tab to amend";
        let (question, choices) = extract_permission_prompt(screen);
        assert!(question.contains("proceed"), "got: {question}");
        assert_eq!(choices.len(), 2);
        assert!(choices.iter().any(|c| c.contains("Yes")));
        assert!(choices.iter().any(|c| c.contains("No")));
    }

    #[test]
    fn strip_cursor_prefix_works() {
        assert_eq!(strip_cursor_prefix("❯ hello"), "hello");
        assert_eq!(strip_cursor_prefix("> hello"), "hello");
        assert_eq!(strip_cursor_prefix("  hello"), "hello");
        assert_eq!(strip_cursor_prefix("hello"), "hello");
    }

    #[test]
    fn diff_stats_line_detection() {
        assert!(is_diff_stats_line("13 files +649 -133"));
        assert!(is_diff_stats_line("1 file +10 -5"));
        assert!(!is_diff_stats_line("some random text"));
        assert!(!is_diff_stats_line("files +10 -5")); // missing number
        assert!(!is_diff_stats_line("13 files")); // incomplete
    }

    #[test]
    fn scan_triggers_detects_diff_stats() {
        assert!(scan_triggers("13 files +649 -133"));
        assert!(scan_triggers("1 file +10 -5"));
    }

    #[test]
    fn analyze_screen_waiting_on_diff_stats() {
        let mut det = SessionDetector::new();
        det.trigger_pending = true;
        let screen = "> Try \"create a util logging.py\"\n  13 files +649 -133\n";
        let result = det.analyze_screen(screen);
        assert!(matches!(result, Some(SessionEvent::WaitingForInput)));
        assert_eq!(det.status(), SessionStatus::Waiting);
    }

    #[test]
    fn should_analyze_re_triggers_when_working_and_silent() {
        let mut det = SessionDetector::new();
        det.trigger_pending = false;
        det.status = SessionStatus::Working;
        det.last_output_at = Some(
            Instant::now()
                .checked_sub(std::time::Duration::from_secs(3))
                .unwrap(),
        );
        assert!(
            det.should_analyze(),
            "should re-analyze when Working but silent for 2+ seconds"
        );
    }

    #[test]
    fn should_analyze_no_re_trigger_when_waiting() {
        let mut det = SessionDetector::new();
        det.trigger_pending = false;
        det.status = SessionStatus::Waiting;
        det.last_output_at = Some(
            Instant::now()
                .checked_sub(std::time::Duration::from_secs(3))
                .unwrap(),
        );
        assert!(
            !det.should_analyze(),
            "should NOT re-analyze when already Waiting"
        );
    }
}
