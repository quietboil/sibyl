# Example Project

Let's say we need to write a program to generate a report for the [demo HR schema][1]. This report shall list median salaries for each country of the specified region. The name of the region and the connection parameters shall be specified as environment variables (maybe because our program will be executed in a container).

## Project Setup

```shell
cargo new median-salary
```

Then edit `Cargo.toml` and include Sibyl as a dependency:

```toml
{{#include ../../examples/median-salary/Cargo.toml:6:}}
```

We will be writing a single threaded program, thus we will use Sibyl's `blocking` API.

## Implement the Solution

```rust,noplayground
{{#include ../../examples/median-salary/src/main.rs}}
```

## Build and Run

```shell
cargo build
```

And then run it:

```shell
DBNAME=localhost/orcl DBUSER=sibyl DBPASS=Or4cl3 REGION=Americas cargo run
```

## Expected Output

```plaintext
Canada                   :  9500
United States of America :  3250
```

[1]: https://docs.oracle.com/en/database/oracle/oracle-database/19/comsc/installing-sample-schemas.html#GUID-1E645D09-F91F-4BA6-A286-57C5EC66321D
