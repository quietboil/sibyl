# Character Sets

When a database character set is anything other than AL32UTF8, UTF8, or US7ASCII, the reported by Oracle character column data size most likely will be smaller than what is required to store the retrieved text encoded as UTF-8 in Rust. To address this issue Sibyl applies a *database character set to UTF-8 worst case conversion factor* when it allocates memory for column buffers. By default this factor is 1, which works with AL32UTF8, UTF8, and US7ASCII.

In cases when conversion of the database character set to UTF-8 requires more bytes for each character, in the worst case, than is used by the original encoding this conversion factor should be provided to the application via environment variable `ORACLE_UTF8_CONV_FACTOR`. The factor is an unsigned integer. However, the most likely values for it would be 2, 3, or 4.

## Example

A Thai Ko Kai "ก" character in TH8TISASCII encoding is stored as `0xA1`. However, it is encoded as `0xE0 0xB8 0x81` in UTF-8. Thus, an application that is connected to the database that uses TH8TISASCII character set needs to use a conversion factor 3. To run such an application one would need to set `ORACLE_UTF8_CONV_FACTOR` before executing it:

```sh
export ORACLE_UTF8_CONV_FACTOR=3
```
