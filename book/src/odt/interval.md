# Interval

There are 2 types of intervals:
- `IntervalYM` which is eqivalent to Oracle's INTERVAL YEAR TO MONTH,
- `IntervalDS` - INTERVAL DAY TO SECOND

```rust,ignore
use sibyl::{ TimestampTZ, IntervalDS };

let launch  = TimestampTZ::with_date_and_time(1969, 7, 16, 13, 32,  0, 0, "UTC", &oracle)?;
let landing = TimestampTZ::with_date_and_time(1969, 7, 24, 16, 50, 35, 0, "UTC", &oracle)?;
let duration : IntervalDS = landing.subtract(&launch)?;

assert_eq!(duration.to_string(1,3)?, "+8 03:18:35.000");
```
