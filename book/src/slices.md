# Using Slices as Statement Arguments

Sibyl accepts a collection of values (as a slice) as a single positional or a named argument. The primary target of this feature is to make passing arguments for IN-list parameters easier. However, as Sibyl simply unrolls a slice into consecutive arguments, this feature can also be "abused" :-) to pass multiple consecutive arguments of the same type when it is convenient.

## Example

```rust,noplayground
{{#include ../../examples/arg_as_slice.rs:17:40}}
```
