# Cw20 Wrap

CosmWasm contract for wrapping native coins into wrapped cw20's; supporting deposits and withdrawals.

## Build

```
RUSTFLAGS='-C link-arg=-s' cargo wasm
```

## Optimize

```
docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/rust-optimizer:0.12.9
```
