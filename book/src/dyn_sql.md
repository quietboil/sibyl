# Dynamic SQL

In cases when the code for an SQL statement is being constructed at runtime as a string, it might be difficult to express the SQL arguments as a tuple. The following approaches allow construction of SQL arguments dynamically with the SQL code and passing them to Sibyl for SQL execution.

## Slice of Same Type Arguments

Quite often, when the SQL is built dynamically, argument values are provided out of the process. This way they most likely will arrive as strings. These strings can be collected into a vector and then passed to Sibyl as a slice of strings.

> For the sake of clarity the example below does not show the actual process of building the SQL and the vector of arguments. One would have to feel in the gaps.

```rust,noplayground
{{#include ../../examples/dyn_args_slice.rs:11:45}}
```

> Note that the example above uses only IN parameters. When some of them are OUT or INOUT the slice passed to Sibyl must be mutable, i.e. extracted via `as_mut_slice()`, and the strings at appropriate positions sized, i.e. created via `with_capacity()`, to accommodate the maximum length of the expected output.

While this woks, it has a certain, albeit minor, limitation - the SQL code has to expect only arguments of the same type and apply appropriate explicit conversion where appropriate. Sometimes this makes SQL a bit cluttered and somewhat more difficult to read.

## `Vec` of Any Type

More strictly - a vector of any type that implements Sibyl's `ToSql` trait.

```rust,noplayground
{{#include ../../examples/dyn_args.rs:12:52}}
```

While this allows working with arguments that have appropriate types, it has a limitation on its own - in order to be able to accept arguments for OUT or INOUT parameters this method requires all arguments to be mutable. In a way it treats all of them as INOUT even when that is not actually needed.