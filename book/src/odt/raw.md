# RAW

```rust,noplayground
# fn main() -> sibyl::Result<()> {
use sibyl::Raw;

let env = sibyl::env()?;

let raw = Raw::from_bytes(&[1u8,2,3,4,5], &env)?;

assert_eq!(raw.as_bytes(), &[1u8,2,3,4,5]);
# }
```
