# VARCHAR

```rust,noplayground
# fn main() -> sibyl::Result<()> {
use sibyl::Varchar;

let env = sibyl::env()?;

let txt = Varchar::from("Hello, World!", &env)?;

assert_eq!(txt.as_str(), "Hello, World!");
# }
```
