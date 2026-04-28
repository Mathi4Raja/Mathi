use regex::Regex;

#[derive(Debug, Clone)]
pub struct Redactor {
    patterns: Vec<Regex>,
}

impl Default for Redactor {
    fn default() -> Self {
        Self {
            patterns: vec![
                Regex::new(r"(?i)bearer\s+[a-z0-9._\-]+").expect("valid bearer regex"),
                Regex::new(r#"(?i)(api[_\-]?key|token|secret)\s*[:=]\s*['"]?[a-z0-9_\-]{8,}['"]?"#).expect("valid credential regex"),
                Regex::new(r"(?i)[a-z0-9._%+\-]+@[a-z0-9.\-]+\.[a-z]{2,}").expect("valid email regex"),
            ],
        }
    }
}

impl Redactor {
    pub fn redact_text(&self, input: &str) -> String {
        self.patterns.iter().fold(input.to_string(), |acc, pattern| {
            pattern.replace_all(&acc, "[REDACTED]").to_string()
        })
    }
}
