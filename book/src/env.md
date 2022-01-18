# Environment

An OCI environment handle must be created before any other OCI function can be called. While there can be many environments - for example, they might be configured to have different languages and territories - usually one is sufficient. Sibyl initializes it to be the most compatible with Rust requirements - thread-safe using UTF8 (AL32UTF8) character encoding. That single environment handle can be created in `main` and then passed around:

```rust,ignore
fn main() {
    let oracle = sibyl::env().expect("Oracle OCI environment");
    // ...
}
```

Note however that some functions will need a direct reference to this handle, so instead of passing it around some applications might prefer to create it statically:

```rust,ignore
use sibyl::Environment;
use lazy_static::lazy_static;

lazy_static!{
    pub static ref ORACLE : Environment = sibyl::env().expect("Oracle OCI environment");
}
```

Then later one would be able to create, for example, a current timestamp as:

```rust,ignore
use sibyl::TimestampTZ;

let current_timestamp = TimestampTZ::from_systimestamp(&ORACLE)?;
```
