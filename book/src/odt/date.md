# Date

```rust,ignore
use sibyl::Date;

let mar28_1996 = Date::from_string("28-MAR-1996", "DD-MON-YYYY", &oracle)?;
let next_monday = mar28_1996.next_week_day("MONDAY")?;

assert_eq!(next_monday.to_string("DL")?, "Monday, April 01, 1996");
```
