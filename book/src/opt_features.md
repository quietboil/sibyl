# Optional Features

Sibyl provides one optional opt-in feature - `unsafe-direct-binds`.

By default Sibyl creates shadow buffers for arguments that are bound to `IN` parameter placeholders. With `unsafe-direct-binds` Sibyl instead binds arguments directly. This, of course, is somewhat more performant and conserves memory. However, `unsafe-direct-binds` makes it possible to violate Rust's immutability of references when a reference is mistakenly bound to the `OUT` or `INOUT` placeholder.

## Example

```rust,noplayground
{{#include ../../examples/binding.rs:11:34}}
```

### Default (Safe) Binding

```rust,noplayground
{{#include ../../examples/binding.rs:35:37}}
```

The OCI actually did change the value as values bound to OUT placeholders are always changed. However, that has happened in the shadow buffer that Sibyl created to bind the value, thus actual value in Rust was not affected.

## Unsafe Direct Binding

The binding mistake allows unacceptable mutation of the bound value:

```rust,noplayground
{{#include ../../examples/binding.rs:38:39}}
```

Note also that because the string was bound via a (read-only) reference Sibyl used read-only binding for it and thus the code that sets the `String` length to match the loaded value was not executed. As the result the new name still has the last 3 characters from the original name.
