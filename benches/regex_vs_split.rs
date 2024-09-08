use anyhow::Result;
use booru::board::danbooru;
use booru::tags::{build_tags_regex, TagMatcher};
use criterion::*;
use rand::distributions::Alphanumeric;
use rand::Rng;
use regex::Regex;
use std::iter;
use std::sync::LazyLock;

/// regex for tags about people
const PEOPLE_TAGS_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| build_tags_regex(&danbooru::tags::PEOPLE_TAGS).unwrap());

/// one or more spaces
const SPACES_PATTERN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+").unwrap());

// generates random word
fn random_word(len: usize) -> String {
    // `Alphanumeric` を使ってランダムなアルファベットと数字を生成
    let mut rng = rand::thread_rng();
    iter::repeat_with(|| rng.sample(Alphanumeric))
        .take(len)
        .map(char::from)
        .collect()
}

// generates random spaces
fn random_spaces(min: usize, max: usize) -> String {
    let mut rng = rand::thread_rng();
    let space_len = rng.gen_range(min..=max);
    " ".repeat(space_len)
}

// generates long string using random words and spaces
fn generate_long_string(
    word_count: usize,
    min_word_len: usize,
    max_word_len: usize,
    min_space_len: usize,
    max_space_len: usize,
) -> String {
    let mut rng = rand::thread_rng();
    let mut result = String::new();

    for _ in 0..word_count {
        let word_len = rng.gen_range(min_word_len..=max_word_len);
        let word = random_word(word_len);

        let spaces = random_spaces(min_space_len, max_space_len);

        result.push_str(&word);
        result.push_str(&spaces);
    }

    result
}

fn generate_random_people_tags(
    count: usize,
    min_word_len: usize,
    max_word_len: usize,
) -> Vec<String> {
    let mut rng = rand::thread_rng();
    let tags = danbooru::tags::PEOPLE_TAGS.to_vec();
    let mut result = vec![];

    for _ in 0..count {
        let tag = tags[rng.gen_range(0..tags.len())].to_string();
        let word_len = rng.gen_range(min_word_len..=max_word_len);
        let word = random_word(word_len);

        result.push(tag);
        result.push(word);
    }

    result
}

// replace using split and join
fn split_and_join_ws(text: &str) -> String {
    text.trim()
        .split_whitespace()
        .filter(|t| !t.trim().is_empty())
        .collect::<Vec<&str>>()
        .join(", ")
}

// replace using regex
fn regex_replace_ws(text: &str) -> String {
    LazyLock::force(&SPACES_PATTERN)
        .replace_all(text.trim(), ", ")
        .to_string()
}

fn contains_match_people(tags: Vec<String>) -> Result<()> {
    let matcher = danbooru::tags::Matcher::new(danbooru::tags::PEOPLE_TAGS.to_vec());

    let mut people_tags = vec![];
    let mut other_tags = vec![];

    for tag in tags {
        if matcher.has(&tag.as_ref()) {
            people_tags.push(tag);
        } else {
            other_tags.push(tag);
        }
    }

    Ok(())
}

fn regex_match_people(tags: Vec<String>) -> Result<()> {
    let regex = LazyLock::force(&PEOPLE_TAGS_PATTERN).clone();

    let mut people_tags = vec![];
    let mut other_tags = vec![];

    for tag in tags {
        if regex.is_match(&tag) {
            people_tags.push(tag);
        } else {
            other_tags.push(tag);
        }
    }

    Ok(())
}

// benchmark
fn criterion_benchmark(c: &mut Criterion) {
    let text = generate_long_string(100_000, 3, 10, 1, 10);

    // check if both functions return the same result
    assert_eq!(split_and_join_ws(&text), regex_replace_ws(&text));

    c.bench_function("replace spaces to commas; split and join", |b| {
        b.iter(|| split_and_join_ws(&text))
    });
    c.bench_function("replace spaces to commas; regex replace", |b| {
        b.iter(|| regex_replace_ws(&text))
    });

    let tags = generate_random_people_tags(100_000, 3, 10);

    assert_eq!(
        contains_match_people(tags.clone()).unwrap(),
        regex_match_people(tags.clone()).unwrap()
    );

    c.bench_function("match tags; contains match", |b| {
        b.iter(|| contains_match_people(tags.clone()).unwrap())
    });
    c.bench_function("match tags; regex match", |b| {
        b.iter(|| regex_match_people(tags.clone()).unwrap())
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
