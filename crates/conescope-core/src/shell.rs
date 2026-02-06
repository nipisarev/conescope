use std::path::PathBuf;

/// Returns the user's default shell, falling back through common options.
#[must_use]
pub fn default_shell() -> PathBuf {
    if let Ok(shell) = std::env::var("SHELL") {
        if !shell.is_empty() {
            return PathBuf::from(shell);
        }
    }
    for candidate in &["/bin/zsh", "/bin/bash", "/bin/sh"] {
        let p = PathBuf::from(candidate);
        if p.exists() {
            return p;
        }
    }
    PathBuf::from("/bin/sh")
}

/// Returns PATH augmented with common tool directories.
#[must_use]
pub fn augmented_path() -> String {
    let current = std::env::var("PATH").unwrap_or_default();
    let home = std::env::var("HOME").unwrap_or_default();
    let extra_dirs = [
        "/opt/homebrew/bin".to_string(),
        "/opt/homebrew/sbin".to_string(),
        "/usr/local/bin".to_string(),
        format!("{home}/.cargo/bin"),
        format!("{home}/.local/bin"),
    ];
    let mut parts: Vec<&str> = Vec::new();
    for d in &extra_dirs {
        if !current.contains(d.as_str()) {
            parts.push(d);
        }
    }
    if parts.is_empty() {
        return current;
    }
    parts.push(&current);
    parts.join(":")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_shell_returns_something() {
        let shell = default_shell();
        assert!(
            shell.to_str().unwrap().contains("sh"),
            "Expected a shell path, got: {shell:?}"
        );
    }

    #[test]
    fn augmented_path_includes_homebrew() {
        let path = augmented_path();
        assert!(
            path.contains("/opt/homebrew/bin"),
            "Expected /opt/homebrew/bin in PATH"
        );
    }
}
