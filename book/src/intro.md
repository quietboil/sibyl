# Introduction

Sibyl is an [OCI][1]-based interface (a.k.a. a driver) between Rust applications and Oracle databases. It supports both sync (blocking) and async (nonblocking) API.

## Example

Assuming an [HR sample schema][2] is installed, the following example program would report median salaries for each country in the specified region.

### Blocking Mode Version

```rust,noplayground
{{#include ../../examples/book_intro.rs:5:}}
```

When executed it prints:

```plaintext
Germany                  : 10000
United Kingdom           :  8800
```

### Nonblocking (async) Mode Version

```rust,noplayground
{{#include ../../examples/async_book_intro.rs:5:}}
```
> Note the only difference between this and the blocking mode program is that `async` method calls need to be `await`ed. Otherwise the `async` version of the program is a verbatim copy of the non-async one.

[1]: https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/index.html
[2]: https://docs.oracle.com/en/database/oracle/oracle-database/19/comsc/installing-sample-schemas.html#GUID-1E645D09-F91F-4BA6-A286-57C5EC66321D
