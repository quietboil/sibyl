# Number

```rust,ignore
use sibyl::Number;

let oracle = sibyl::env()?;

let pi = Number::pi(&oracle);
let two = Number::from_int(2, &oracle);
let two_pi = pi.mul(&two)?;
let h = Number::from_string("6.62607004E-34", "9D999999999EEEE", &oracle)?;
let hbar = h.div(&two_pi)?;

assert_eq!(
    hbar.to_string("TME")?,
    "1.05457180013911265115394106872506677375E-34"
);
```
