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

    pub fn format_template(&self, template: &str, post: &danbooru::response::Post) -> String {
        let general_tags = split_whitespaces(&post.tag_string_general);
        let character_tags = split_whitespaces(&post.tag_string_character);
        let copyright_tags = split_whitespaces(&post.tag_string_copyright);
        let artist_tags = split_whitespaces(&post.tag_string_artist);
        let meta_tags = split_whitespaces(&post.tag_string_meta);

        let (people_tags, general_tags) = self.people_matcher.classify_has(general_tags);
        let (_ooc_meta_tags, meta_tags) = self.ooc_meta_matcher.classify_any_in(meta_tags);

        let mut result = template.to_string();
        result = result.replace("{people}", &self.normalize(people_tags).join(", "));
        result = result.replace("{general}", &self.normalize(general_tags).join(", "));
        result = result.replace("{character}", &self.normalize(character_tags).join(", "));
        result = result.replace("{copyright}", &self.normalize(copyright_tags).join(", "));
        result = result.replace("{artist}", &self.normalize(artist_tags).join(", "));
        result = result.replace("{meta}", &self.normalize(meta_tags).join(", "));

        result
    }
}

#[cfg(test)]
mod test {
    use danbooru::response::post::MediaAsset;

    use super::*;

    #[test]
    fn test_tag_manager() {
        let manager = TagManager::new();
        let post = danbooru::response::Post {
            id: 1,
            created_at: "2021-01-01".to_string(),
            updated_at: "2021-01-01".to_string(),
            score: 10,
            source: "https://example.com".to_string(),
            up_score: 10,
            down_score: 0,
            fav_count: 3,
            rating: danbooru::Rating::General,
            image_width: 512,
            image_height: 768,
            tag_count: 10,
            tag_string: "".to_string(),
            tag_string_general: "1girl cat_ears".to_string(),
            tag_string_character: "hatsune_miku".to_string(),
            tag_string_copyright: "vocaloid".to_string(),
            tag_string_artist: "example_(artist)".to_string(),
            tag_string_meta: "absurdres commentary_request photo_(medium)".to_string(),
            tag_count_general: 99,
            tag_count_artist: 99,
            tag_count_character: 99,
            tag_count_copyright: 99,
            tag_count_meta: 00,
            has_large: true,
            media_asset: MediaAsset {
                id: 1,
                created_at: "2021-01-01".to_string(),
                updated_at: "2021-01-01".to_string(),
                md5: None,
                file_ext: danbooru::FileExt::Png,
                file_size: 12344,
                image_width: 512,
                image_height: 768,
                duration: None,
                status: "active".to_string(),
                file_key: None,
                is_public: true,
                pixel_hash: "".to_string(),
                variants: None,
            },
            file_url: Some("https://example.com".to_string()),
            large_file_url: Some("https://example.com".to_string()),
            preview_file_url: Some("https://example.com".to_string()),
            parent_id: None,
            has_children: false,
            has_active_children: false,
            has_visible_children: false,
            last_commented_at: Some("2021-01-01".to_string()),
            last_comment_bumped_at: Some("2021-01-01".to_string()),
            last_noted_at: Some("2021-01-01".to_string()),
            file_size: 1234,
            file_ext: danbooru::FileExt::Png,
            md5: None,
            uploader_id: 2,
            approver_id: None,
            pixiv_id: None,
            is_pending: false,
            is_flagged: false,
            is_deleted: false,
            is_banned: false,
            bit_flags: 0,
        };

        let template = "{people}, {character}, {copyright}, {general}, {artist}, {meta}";
        let result = manager.format_template(template, &post);

        assert_eq!(
            result,
            "1girl, hatsune miku, vocaloid, cat ears, example (artist), absurdres, photo (medium)"
        );
    }
}
