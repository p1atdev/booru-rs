use anyhow::Result;
use regex::{Regex, RegexBuilder};

/// build a regex from tags
pub fn build_tags_regex(tags: &[&str]) -> Result<Regex> {
    let tags = tags
        .iter()
        .map(|t| regex::escape(t))
        .collect::<Vec<_>>()
        .join("|");
    let regex = format!(r"{}", tags);
    let regex = RegexBuilder::new(&regex).case_insensitive(true).build()?;
    Ok(regex)
}

/// TagMatcher trait
pub trait TagMatcher {
    /// create a new TagRegex
    fn new(tags: Vec<&str>) -> Self
    where
        Self: Sized;

    fn tags(&self) -> Vec<String>;

    /// whether the matcher has a provided tag
    fn has(&self, tag: &str) -> bool {
        self.tags().contains(&tag.to_string())
    }

    /// whether a text contains at least one tag
    fn any_in(&self, text: &str) -> bool {
        self.tags().iter().any(|t| text.contains(t))
    }

    /// classify tags into two groups: matched and unmatched
    fn classify_has(&self, tags: Vec<String>) -> (Vec<String>, Vec<String>) {
        let mut matched = vec![];
        let mut unmatched = vec![];

        for tag in tags {
            if self.has(&tag) {
                matched.push(tag);
            } else {
                unmatched.push(tag);
            }
        }

        (matched, unmatched)
    }

    /// classify tags into two groups: matched and unmatched
    fn classify_any_in(&self, tags: Vec<String>) -> (Vec<String>, Vec<String>) {
        let mut matched = vec![];
        let mut unmatched = vec![];

        for tag in tags {
            if self.any_in(&tag) {
                matched.push(tag);
            } else {
                unmatched.push(tag);
            }
        }

        (matched, unmatched)
    }
}

/// TagNormalizer trait
pub trait TagNormalizer {
    fn new() -> Self
    where
        Self: Sized;

    /// normalize a tag
    fn normalize_text(&self, text: &str) -> String;

    /// normalize tags
    fn normalize(&self, tags: Vec<String>) -> Vec<String> {
        tags.into_iter()
            .filter(|t| !t.trim().is_empty())
            .map(|t| self.normalize_text(&t))
            .collect::<Vec<_>>()
    }
}

/// place tags with commas instead of spaces
pub fn replace_whitespace_to_comma(text: &str) -> String {
    text.trim()
        .split_whitespace()
        .filter(|t| !t.trim().is_empty())
        .collect::<Vec<_>>()
        .join(", ")
}

/// split by one or more spaces
pub fn split_whitespaces(text: &str) -> Vec<String> {
    text.trim()
        .split_whitespace()
        .filter(|t| !t.trim().is_empty())
        .map(|t| t.to_string())
        .collect::<Vec<_>>()
}
