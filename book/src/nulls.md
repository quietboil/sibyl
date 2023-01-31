# NULLs as Statement Arguments

There are several, sometimes complementary, ways to pass nulls into and receive them from statements.

## Empty Strings Are Nulls

This is idiosyncratic Oracle's way to treat empty strings as nulls.

### Example

```rust,noplayground
{{#include ../../examples/null_empty_string.rs:11:19}}
```

When used as an OUT or an INOUT arguments a `String` or a `Varchar` does not even need to be wrapped into `Option` as its length by itself indicates whether Oracle sees it as null or returned its value as null.

### Example

```rust,noplayground
{{#include ../../examples/null_empty_string.rs:21:39}}
```

## Using `Option` to Represent Null

`Option` allows the maximum flexibility in representing input and output null values.

### Example

```rust,noplayground
{{#include ../../examples/null_option.rs:11:26}}
```

Unfortunately, there are a few cases, specifically with INOUT parameters, when an `Option` cannot represent arguments that are null values as inputs and concrete values on output. To support these Sibyl offers `Nvl` type.

## Using `Nvl` to Represent Null

`Nvl` wraps a value that would provide a storage for an output value, but it binds it to the parameter placeholder as null.

### Example

```rust,noplayground
{{#include ../../examples/null_nvl.rs:11:31}}
```

Similarly INOUT `Interval` must be wrapped in `Nvl` if the input value is null while output is expected to be an actual interval.
