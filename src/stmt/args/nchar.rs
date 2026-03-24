use super::{Params, ToSql};
use crate::{Result, Varchar, oci::*};

/**
    Types that can be encoded using national character set.
 */
pub trait NCharForm {}

impl NCharForm for &str {}
impl NCharForm for &&str {}
impl NCharForm for String {}
impl NCharForm for &String {}
impl NCharForm for &mut String {}
impl NCharForm for Varchar<'_> {}
impl NCharForm for &Varchar<'_> {}
impl NCharForm for &mut Varchar<'_> {}

impl<T: NCharForm> NCharForm for &[T] {}
impl<T: NCharForm> NCharForm for &mut [T] {}

impl<T: NCharForm> NCharForm for Vec<T> {}
impl<T: NCharForm> NCharForm for &Vec<T> {}
impl<T: NCharForm> NCharForm for &mut Vec<T> {}

impl<T: NCharForm, const N: usize> NCharForm for [T; N] {}
impl<T: NCharForm, const N: usize> NCharForm for &[T; N] {}
impl<T: NCharForm, const N: usize> NCharForm for &mut [T; N] {}

impl<T: NCharForm> NCharForm for Option<T> {}
impl<T: NCharForm> NCharForm for &Option<T> {}
impl<T: NCharForm> NCharForm for &mut Option<T> {}

/**
    Represents a value that will be encoded using national character set.

    This type is used to indicate that a value bound to the parameter placeholder
    should be encoded using national character set. This is normally not the case
    as bound "[data is converted to the database character set before converting to
    or from the national character set](https://docs.oracle.com/en/database/oracle/oracle-database/19/nlspg/programming-with-unicode.html#GUID-337FC5E5-9A3F-4E49-B6C5-A94D82607BB9)."

    The use of this type is unnecessary if the database character set is Unicode
    (AL32UTF8, UTF8, UTFE). In this case the conversion to the database character
    set does not lose any data and thus the subsequent conversion to the national
    character set preserves the original data.

    However, if the database character set is not Unicode, then, depending on the
    data, the conversion to the database character set might lose characters that
    cannot be represented in the current database character set.

    This type alters the binding of the argument value to the statement parameter
    placeholder to indicate that the value should be encoded using the national
    character set directly.

    # Example

    This example assumes that the following test table exists in the database:
    ```sql
    CREATE TABLE test_character_data (
        id      NUMBER GENERATED ALWAYS AS IDENTITY,
        text    VARCHAR2(97),
        ntext   NVARCHAR2(99)
    )
    ```

    ```rust
    # #[cfg(feature="blocking")]
    # fn main() -> sibyl::Result<()> {
    # let session = sibyl::test_env::get_session()?;
    use sibyl::NChar;

    let stmt = session.prepare("
        INSERT INTO test_character_data (ntext) VALUES (:TEXT)
        RETURNING id INTO :ID
    ")?;
    let mut id = 0;
    let spring_ocean = "春の海 ひねもすのたり のたりかな";
    stmt.execute((
        ("TEXT", NChar(spring_ocean)),
        ("ID", &mut id)
    ))?;

    let stmt = session.prepare("
        BEGIN
            SELECT ntext INTO :TEXT FROM test_character_data WHERE id = :ID;            
        END;
    ")?;
    let mut haiku = String::with_capacity(99);
    stmt.execute((
        ("TEXT", NChar(&mut haiku)),
        ("ID", id)
    ))?;

    assert_eq!(haiku.as_str(), spring_ocean);
    # Ok(())
    # }
    # #[cfg(feature="nonblocking")]
    # fn main() {}
    ```
 */
pub struct NChar<T> (pub T) where T: ToSql + NCharForm;

impl<T> ToSql for NChar<T> where T: ToSql + NCharForm {
    fn bind_to(&mut self, pos: usize, params: &mut Params, stmt: &OCIStmt, err: &OCIError) -> Result<usize> {
        let next_pos = self.0.bind_to(pos, params, stmt, err)?;
        for i in pos..next_pos {
            params.mark_as_nchar(i, err)?;
        }
        Ok(next_pos)
    }

    fn update_from_bind(&mut self, pos: usize, params: &Params) -> Result<usize> {
        self.0.update_from_bind(pos, params)
    }
}
