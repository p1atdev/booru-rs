use crate::tags::{split_whitespaces, TagMatcher, TagNormalizer};

// tags which has underscore in them
#[rustfmt::skip]
pub const UNDERSCORE_TAGS: [&str; 19] = [
    ">_<",
    ">_o",
    "0_0",
    "o_o",
    "3_3",
    "6_9",
    "@_@",
    "u_u",
    "x_x",
    "^_^",
    "|_|",
    "=_=",
    "+_+",
    "+_-",
    "._.",
    "<o>_<o>",
    "<|>_<|>",
    // ↓ deprecated
    "||_||",
    "(o)_(o)",
];

// tags about people
#[rustfmt::skip]
pub const PEOPLE_TAGS: [&str; 21] = [
    "1girl",
    "2girls",
    "3girls",
    "4girls",
    "5girls",
    "6+girls",
    "multiple girls",
    "1boy",
    "2boys",
    "3boys",
    "4boys",
    "5boys",
    "6+boys",
    "multiple boys",
    "1other",
    "2others",
    "3others",
    "4others",
    "5others",
    "6+others",
    "multiple others",
];

// tags about focusing
#[rustfmt::skip]
pub const FOCUS_TAGS: [&str; 3] = [
    "solo focus",
    "male focus",
    "other focus",
];

// tags about duplication
#[rustfmt::skip]
pub const DUPLICATATION_META_TAGS: [&str; 2] = [
    "duplicate",
    "pixel-perfect duplicate",
];

// tag parts which are out of context to the image
#[rustfmt::skip]
pub const OUT_OF_CONTEXT_META_TAG_PARTS: [&str; 23] = [
    "commentary",
    "commision",
    "translat",
    "request",
    "mismatch",
    "bad",
    "has",
    "resize",
    "scale",
    "edit",
    "source",
    "available",
    "sample",
    "upload",
    "link",
    "paid",
    "reward",
    "check",
    "variant",
    "text",
    "gift",
    "guest",
    "artist collaboration",
];

/// Tag Matcher
pub struct Matcher {
    tags: Vec<String>,
}

impl TagMatcher for Matcher {
    fn new(tags: Vec<&str>) -> Self {
        Matcher {
            tags: tags.iter().map(|t| t.to_string()).collect(),
        }
    }

    fn tags(&self) -> Vec<String> {
        self.tags.clone()
    }
}

pub struct Normalizer {
    keep_tags: Vec<String>,
}

impl TagNormalizer for Normalizer {
    fn new() -> Self {
        Normalizer {
            keep_tags: UNDERSCORE_TAGS
                .to_vec()
                .iter()
                .map(|t| t.to_string())
                .collect(),
        }
    }

    fn normalize_text(&self, text: &str) -> String {
        let tags = split_whitespaces(text);
        tags.into_iter()
            .map(|t| {
                if self.keep_tags.contains(&t) {
                    t
                } else {
                    t.replace("_", " ")
                }
            })
            .collect::<Vec<_>>()
            .join(", ")
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_matcher_people() {
        let matcher = Matcher::new(PEOPLE_TAGS.to_vec());

        for tag in &PEOPLE_TAGS {
            assert!(matcher.has(tag));
        }

        assert!(!matcher.has("solo focus"));
        assert!(!matcher.has("upper body"));
    }

    #[test]
    fn test_matcher_ooc_meta() {
        let matcher = Matcher::new(OUT_OF_CONTEXT_META_TAG_PARTS.to_vec());

        for tag in &OUT_OF_CONTEXT_META_TAG_PARTS {
            assert!(matcher.any_in(tag));
        }

        assert!(matcher.any_in("english commentary"));
        assert!(matcher.any_in("paid reward"));
        assert!(matcher.any_in("translation request"));
        assert!(matcher.any_in("bad id"));
        assert!(matcher.any_in("third-party edit"));
        assert!(!matcher.any_in("1girl"));
    }

    #[test]
    fn test_normalizer() {
        let normalizer = Normalizer::new();
        assert_eq!(normalizer.normalize_text("1girl"), "1girl");
        assert_eq!(normalizer.normalize_text("cat_ears"), "cat ears");
        assert_eq!(
            normalizer.normalize(vec!["1girl".to_string(), "cat_ears".to_string()]),
            vec!["1girl".to_string(), "cat ears".to_string()]
        );
        assert_eq!(
            normalizer.normalize_text(
                "1girl cat_ears     upper_body   looking_at_viewer  >_< <|>_<|> :3"
            ),
            "1girl, cat ears, upper body, looking at viewer, >_<, <|>_<|>, :3"
        );
    }
}
