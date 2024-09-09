use std::collections::HashMap;

use booru::{
    board::danbooru,
    tags::{split_whitespaces, TagMatcher, TagNormalizer},
};

pub struct TagManager {
    normalizer: danbooru::tags::Normalizer,
    people_matcher: danbooru::tags::Matcher,
    ooc_meta_matcher: danbooru::tags::Matcher,
}

impl TagManager {
    pub fn new() -> Self {
        TagManager {
            normalizer: danbooru::tags::Normalizer::new(),
            people_matcher: danbooru::tags::Matcher::new(danbooru::tags::PEOPLE_TAGS.to_vec()),
            ooc_meta_matcher: danbooru::tags::Matcher::new(
                danbooru::tags::OUT_OF_CONTEXT_META_TAG_PARTS.to_vec(),
            ),
        }
    }

    pub fn normalize(&self, tags: Vec<String>) -> Vec<String> {
        self.normalizer.normalize(tags)
    }

    pub fn replace_with_tags(&self, template: &str, tags: HashMap<&str, Vec<String>>) -> String {
        let mut result = template.to_string();
        for (key, value) in tags {
            result = result.replace(
                key,
                &self
                    .normalize(value)
                    .iter()
                    .map(|t| t.trim())
                    .filter(|t| !t.is_empty())
                    .collect::<Vec<_>>()
                    .join(", "),
            );
        }

        result
    }

    pub fn format_template(&self, template: &str, post: &danbooru::response::Post) -> String {
        let general_tags = split_whitespaces(&post.tag_string_general);
        let character_tags = split_whitespaces(&post.tag_string_character);
        let copyright_tags = split_whitespaces(&post.tag_string_copyright);
        let artist_tags = split_whitespaces(&post.tag_string_artist);
        let meta_tags = split_whitespaces(&post.tag_string_meta);

        let (people_tags, general_tags) = self.people_matcher.classify_has(general_tags);
        let (_ooc_meta_tags, meta_tags) = self.ooc_meta_matcher.classify_any_in(meta_tags);

        let result = self.replace_with_tags(
            template,
            [
                ("{people}", people_tags),
                ("{general}", general_tags),
                ("{character}", character_tags),
                ("{copyright}", copyright_tags),
                ("{artist}", artist_tags),
                ("{meta}", meta_tags),
            ]
            .iter()
            .cloned()
            .collect(),
        );

        result
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_tag_manager_replace_tags() {
        let manager = TagManager::new();

        let general = split_whitespaces("1girl cat_ears");
        let character = split_whitespaces("hatsune_miku");
        let copyright = split_whitespaces("vocaloid");
        let artist = split_whitespaces("example_(artist)");
        let meta = split_whitespaces("absurdres commentary_request photo_(medium)");

        let template = "{character}, {copyright}, {general}, {artist}, {meta}";
        let result = manager.replace_with_tags(
            template,
            [
                ("{general}", general),
                ("{character}", character),
                ("{copyright}", copyright),
                ("{artist}", artist),
                ("{meta}", meta),
            ]
            .iter()
            .cloned()
            .collect(),
        );

        assert_eq!(
            result,
            "hatsune miku, vocaloid, 1girl, cat ears, example (artist), absurdres, commentary request, photo (medium)"
        );
    }

    #[test]
    fn test_tag_manager_replace_tags_empty_category() {
        let manager = TagManager::new();

        let general = split_whitespaces("1girl cat_ears");
        let character = split_whitespaces("");
        let copyright = split_whitespaces("original");
        let artist = split_whitespaces("example_(artist)");
        let meta = split_whitespaces("");

        let template = "{character}, {copyright}, {general}, {artist}, {meta}";
        let result = manager.replace_with_tags(
            template,
            [
                ("{general}", general),
                ("{character}", character),
                ("{copyright}", copyright),
                ("{artist}", artist),
                ("{meta}", meta),
            ]
            .iter()
            .cloned()
            .collect(),
        );

        assert_eq!(result, "original, 1girl, cat ears, example (artist)");
    }
}
