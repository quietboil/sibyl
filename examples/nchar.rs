/*!
This example is demonstrates the use of `NChar` type to specify explicit use of the
national character set to encode the bound value. 

It is, however, primarily exists as a test that is written as an example becuase it
has to connect to a different database than what all other Sibyl's tests and examples
use. The regular database that Sibyl uses to run tests has Unicode database character
set (AL32UTF8). The target database for this test/example uses a non-Unicode database
chacracter set, for example US7ASCII.

This example expects that the following test table has been manually created:
```sql
CREATE TABLE nchar_test_data (
    id  INTEGER GENERATED ALWAYS AS IDENTITY,
    txt NVARCHAR2(100)
)
```

Note that some other binding cases not used by this example can be found in the
`data_types.character_datatypes` test.
*/
#[cfg(feature="blocking")]
fn main() -> sibyl::Result<()> {
    use sibyl as oracle;
    use sibyl::{NChar, Varchar};

    let oracle = oracle::env()?;

    let dbname = std::env::var("DBNAME").expect("database name");
    let dbuser = std::env::var("DBUSER").expect("user name");
    let dbpass = std::env::var("DBPASS").expect("password");

    let session = oracle.connect(&dbname, &dbuser, &dbpass)?;
    let mut ids = [0u32;6];

    let stmt = session.prepare("
        INSERT INTO nchar_test_data (txt) VALUES (:TXT)
        RETURNING id INTO :ID
    ")?;
    
    let old_pond = "古池や 蛙飛び込む 水の音";
    stmt.execute((
        ("TXT", NChar(old_pond)),
        ("ID", &mut ids[0])
    ))?;

    let spring_ocean = String::from("春の海 ひねもすのたり のたりかな");
    stmt.execute((
        ("TXT", NChar(&spring_ocean)),
        ("ID", &mut ids[1])
    ))?;

    let canola_flowers = String::from("菜の花や 月は東に 日は西に");
    stmt.execute((
        ("TXT", NChar(canola_flowers)),
        ("ID", &mut ids[2])
    ))?;

    let tranquility = Varchar::from("閑けさや 岩にしみいる 蝉の声", &session)?;
    stmt.execute((
        ("TXT", NChar(tranquility)),
        ("ID", &mut ids[3])
    ))?;

    let mut opt: Option<&str> = Some("梅一輪 一輪ほどの 暖かさ");
    stmt.execute((
        ("TXT", NChar(opt)),
        ("ID", &mut ids[4])
    ))?;
    
    opt.take();
    stmt.execute((
        ("TXT", NChar(opt)),
        ("ID", &mut ids[5])
    ))?;

    // Retrieve and print
    let stmt = session.prepare("
        SELECT id, txt FROM nchar_test_data WHERE id >= :ID ORDER BY id
    ")?;
    let rows = stmt.query(ids[0])?;
    while let Some(row) = rows.next()? {
        let id: usize = row.get(0)?;
        let txt: Option<&str> = row.get(1)?;
        if let Some(val) = txt {
            println!("{id:>8}: {val}");
        } else {
            println!("{id:>8}: -----");
        }
    }

    let mut hids = [0u32;6];
    let mut haikus = [
        Some(String::with_capacity(100)), 
        Some(String::with_capacity(100)), 
        Some(String::with_capacity(100)), 
        Some(String::with_capacity(100)), 
        Some(String::with_capacity(100)), 
        Some(String::with_capacity(100))
    ];
    let stmt = session.prepare("
        DECLARE
            TYPE td_tab_t IS TABLE OF nchar_test_data%ROWTYPE;
            tds td_tab_t := td_tab_t();
        BEGIN
            SELECT * BULK COLLECT INTO tds
              FROM nchar_test_data 
             WHERE id IN (:I1, :I2, :I3, :I4, :I5, :I6);

            -- Assignment of outputs is arranged line this
            -- to allow binding to slices
            :O1 := tds(1).id;
            :O2 := tds(2).id;
            :O3 := tds(3).id;
            :O4 := tds(4).id;
            :O5 := tds(5).id;
            :O6 := tds(6).id;

            :N1 := tds(1).txt;
            :N2 := tds(2).txt;
            :N3 := tds(3).txt;
            :N4 := tds(4).txt;
            :N5 := tds(5).txt;
            :N6 := tds(6).txt;
        END;
    ")?;
    stmt.execute((
        ("I1", &ids),
        ("O1", &mut hids),
        ("N1", NChar(&mut haikus))
    ))?;
    println!("---8<---");
    for i in 0..6 {
        let id = hids[i];
        if let Some(val) = &haikus[i] {
            println!("{id:>8}: '{val}'");
        } else {
            println!("{id:>8}: -----");
        }
    }

    Ok(())
}

#[cfg(feature="nonblocking")]
fn main() {}