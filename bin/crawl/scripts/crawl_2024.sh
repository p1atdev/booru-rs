#!/bin/bash

OUTPUT_DIR="output/2024"
TAG_NAME="2024"

carogo build --release

# firstly, crawl from safebooru (only general)
cargo run --release -- \
    --domain safebooru \
    --year-start 2005 --year-end 2023 \
    --month-start 1 --month-end 12 \
    --output-path $OUTPUT_DIR \
    --prefix safebooru \
    --write-concurrency 8 \
    --overwrite

cargo run --release -- \
    --domain safebooru \
    --year-start 2024 --year-end 2024 \
    --month-start 1 --month-end 12 \
    --output-path $OUTPUT_DIR \
    --prefix safebooru \
    --write-concurrency 8 \
    --overwrite

echo "safebooru done"

# then crawl from danbooru (sensitive, questionable, explicit)
cargo run --release -- \
    --domain danbooru \
    --tags "rating:s,q,e" \
    --year-start 2005 --year-end 2023 \
    --month-start 1 --month-end 12 \
    --output-path $OUTPUT_DIR \
    --prefix danbooru \
    --write-concurrency 8 \
    --overwrite

cargo run --release -- \
    --domain danbooru \
    --tags "rating:s,q,e" \
    --year-start 2024 --year-end 2024 \
    --month-start 1 --month-end 12 \
    --output-path $OUTPUT_DIR \
    --prefix danbooru \
    --write-concurrency 8 \
    --overwrite

echo "danbooru done"

echo "all done"
