# crawl

## Example usage

```bash
cargo run --release -- \
    --domain safebooru \
    --year-start 2024 --year-end 2024 \
    --month-start 7 --month-end 8 \
    --output-path ./output \
    --prefix safebooru \
    --write-concurrency 8 \
    --overwrite
```

