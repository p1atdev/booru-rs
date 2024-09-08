# gather

## Installation 

```bash
cargo install --git https://github.com/p1atdev/booru-rs gather
```

## Example usage

```bash
gather "shinosawa_hiro solo" \
    --output-path ./output/hiro \
    --connections 4 \
    --threads 16 \
    --num-posts 20 \
    --tag-template "{people}|||{character}|||{copyright}|||{general}, {meta}|||{artist} style"
```

