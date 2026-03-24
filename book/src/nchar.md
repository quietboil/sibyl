# Using National Character Set

By default when character arguments are passed to a prepared statement they are encoded using database character set. Which is a non-issue, even if they need to be converted later to/from the national character set, if the database character set is [Unicode](https://docs.oracle.com/en/database/oracle/oracle-database/26/nlspg/supporting-multilingual-databases-with-unicode.html#GUID-CD422E4F-C5C6-4E22-B95F-CA9CABBCB543). However, when the database character is is not Unicode, then character data conversion to the database character set might [lose data](https://docs.oracle.com/en/database/oracle/oracle-database/19/nlspg/programming-with-unicode.html#GUID-337FC5E5-9A3F-4E49-B6C5-A94D82607BB9). For these environments Sibyl provides an `NChar` type indicates during parameter binding that the value should be encoded using the national character set.

## Example

```rust,noplayground
{{#include ../../examples/nchar.rs:36:45}}
```

