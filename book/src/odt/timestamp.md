# Timestamp

There are 3 types of timestamps:
- `Timestamp` which is equivalent to Oracle TIMESTAMP data type,
- `TimestampTZ` - TIMESTAMP WITH TIME ZONE, and
- `TimestampLTZ` - TIMESTAMP WITH LOCAL TIME ZONE

```rust,ignore
use sibyl::TimestampTZ;

let ts = oracle::TimestampTZ::from_string(
    "July 20, 1969 8:18:04.16 pm UTC",
    "MONTH DD, YYYY HH:MI:SS.FF PM TZR",
    &oracle
)?;
assert_eq!(
    ts.to_string("YYYY-MM-DD HH24:MI:SS.FF TZR", 3)?,
    "1969-07-20 20:18:04.160 UTC"
);
```

> Note that if you are getting `ORA-01805` when timestamp with time zone is used, then most likely your local client and the server it is connected to are using different versions of the time zone file. This [stackoverflow answer][1] should help you in setting up your local client with the correct time zone file.

[1]: https://stackoverflow.com/questions/69381749/where-is-the-oracle-instant-client-timezone-file-located
