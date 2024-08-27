use std::collections::HashMap;

use indexmap::IndexMap;

use crate::board::BoardSearchTagsBuilder;

use super::{FileExt, Rating};

/// search condition for filtering by the score
#[derive(Debug, Clone)]
pub enum Score {
    MinMax { min: i32, max: i32 },
    Min(i32),
    Max(i32),
    Exact(i32),
}

impl ToString for Score {
    fn to_string(&self) -> String {
        match self {
            Score::MinMax { min, max } => format!("{}..{}", min, max),
            Score::Min(min) => format!("{}..", min),
            Score::Max(max) => format!("..{}", max),
            Score::Exact(exact) => exact.to_string(),
        }
    }
}

/// danbooru search tags builder
pub struct SearchTagsBuilder {
    tags: Vec<String>,
    metatags: IndexMap<String, Vec<String>>,
}

impl BoardSearchTagsBuilder for SearchTagsBuilder {
    fn new() -> Self {
        SearchTagsBuilder {
            tags: Vec::new(),
            metatags: IndexMap::new(),
        }
    }

    fn tags(&self) -> Vec<String> {
        self.tags.clone()
    }

    fn metatags(&self) -> HashMap<String, String> {
        self.metatags
            .iter()
            .map(|(k, v)| (k.clone(), v.join(",")))
            .collect()
    }

    fn add_tag(&mut self, tag: &str) {
        self.tags.push(tag.to_string());
    }

    fn set_metatag(&mut self, key: &str, value: Vec<String>) {
        self.metatags.insert(key.to_string(), value);
    }

    fn append_metatag(&mut self, key: &str, value: &str) {
        if let Some(v) = self.metatags.get_mut(key) {
            v.push(value.to_string());
        } else {
            self.set_metatag(key, vec![value.to_string()]);
        }
    }

    fn build(&self) -> String {
        let tags = self.tags.join(" ");
        let metatags = self
            .metatags
            .iter()
            .map(|(k, v)| format!("{}:{}", k, v.join(",")))
            .collect::<Vec<String>>()
            .join(" ");

        format!("{} {}", tags, metatags)
    }
}

impl SearchTagsBuilder {
    /// set filetypes metatag
    pub fn filetypes(&mut self, filetypes: Vec<FileExt>) {
        let filetypes = filetypes.iter().map(|f| f.to_string()).collect();
        self.set_metatag("filetype", filetypes);
    }

    /// set rating metatag
    pub fn ratings(&mut self, ratings: Vec<Rating>) {
        let ratings = ratings.iter().map(|r| r.to_string()).collect();
        self.set_metatag("rating", ratings);
    }

    /// set score metatag
    pub fn scores(&mut self, scores: Vec<Score>) {
        self.append_metatag(
            "score",
            &scores
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<String>>()
                .join(","),
        );
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_search_tags_builder() {
        let mut builder = SearchTagsBuilder::new();
        builder.add_tag("1girl");
        builder.add_tag("solo");
        builder.ratings(vec![Rating::General, Rating::Sensitive]);
        builder.filetypes(vec![FileExt::Jpg, FileExt::Png]);
        builder.scores(vec![Score::MinMax { min: 50, max: 100 }]);

        assert_eq!(builder.tags(), vec!["1girl", "solo"]);
        assert_eq!(builder.metatags().get("rating").unwrap(), "g,s");
        assert_eq!(builder.metatags().get("filetype").unwrap(), "jpg,png");
        assert_eq!(builder.metatags().get("score").unwrap(), "50..100");

        let tags = builder.build();

        assert_eq!(tags, "1girl solo rating:g,s filetype:jpg,png score:50..100");
    }
}
