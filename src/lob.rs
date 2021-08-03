//! Functions for performing operations on large objects (LOBs).

use crate::*;
use crate::desc::{ Descriptor, DescriptorType };
use crate::conn::Conn;
use libc::c_void;
use std::{ cmp, mem, ptr, cell::Cell };

extern "C" {
    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-84EA4A66-27BF-470C-8464-3DE31937702A
    // fn OCIDurationBegin(
    //     envhp:      *mut OCIEnv,
    //     errhp:      *mut OCIError,
    //     svchp:      *const OCISvcCtx,
    //     parent:     u16,
    //     duration:   *mut u16
    // ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-AABC3F29-C91B-45A7-AF1E-D486C12E4962
    // fn OCIDurationEnd(
    //     envhp:      *mut OCIEnv,
    //     errhp:      *mut OCIError,
    //     svchp:      *const OCISvcCtx,
    //     duration:   u16
    // ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-5B43FC88-A649-4764-8C1E-6D792F05F7CE
    fn OCILobAppend(
        svchp:      *mut OCISvcCtx,
        errhp:      *mut OCIError,
        dst:        *mut OCILobLocator,
        src:        *const OCILobLocator,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-9B25760D-649E-4B83-A0AA-8C4F3C479BC8
    fn OCILobCharSetForm(
        envhp:      *mut OCIEnv,
        errhp:      *mut OCIError,
        src:        *const OCILobLocator,
        csform:     *mut u8
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-A243691D-8180-4AF6-AA6E-DF9333F8258B
    fn OCILobCharSetId(
        envhp:      *mut OCIEnv,
        errhp:      *mut OCIError,
        src:        *const OCILobLocator,
        csid:       *mut u16
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-CBEB9238-6B47-4A08-8C8D-FC2E5ED56557
    fn OCILobClose(
        svchp:      *mut OCISvcCtx,
        errhp:      *mut OCIError,
        loc:        *mut OCILobLocator,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-404C8A50-516F-4DFD-939D-646A232AF7DF
    fn OCILobCopy2(
        svchp:      *mut OCISvcCtx,
        errhp:      *mut OCIError,
        dst:        *mut OCILobLocator,
        src:        *mut OCILobLocator,
        amount:     u64,
        dst_off:    u64,
        src_off:    u64,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-63F75EC5-EB14-4E25-B593-270FF814615A
    fn OCILobCreateTemporary(
        svchp:      *mut OCISvcCtx,
        errhp:      *mut OCIError,
        loc:        *mut OCILobLocator,
        csid:       u16,
        csfrm:      u8,
        lob_type:   u8,
        cache:      u8,
        duration:   u16,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-264797B2-B3EA-4F6D-9A0E-BF8A4DDA13FA
    fn OCILobErase2(
        svchp:      *mut OCISvcCtx,
        errhp:      *mut OCIError,
        loc:        *mut OCILobLocator,
        amount:     *mut u64,
        offset:     u64,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-40AFA7A3-3A24-4DF7-A719-AECA7C1F522A
    fn OCILobFileClose(
        svchp:      *mut OCISvcCtx,
        errhp:      *mut OCIError,
        filep:      *mut OCILobLocator,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-977F905D-DAFB-4D88-8FE0-7A345837B147
    fn OCILobFileExists(
        svchp:      *mut OCISvcCtx,
        errhp:      *mut OCIError,
        filep:      *mut OCILobLocator,
        flag:       *mut u8
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-BF637A34-B18A-47EE-A060-93C4E79D1813
    fn OCILobFileGetName(
        envhp:      *mut OCIEnv,
        errhp:      *mut OCIError,
        loc:        *const OCILobLocator,
        dir:        *mut u8,
        dir_len:    *mut u16,
        filename:   *mut u8,
        name_len:   *mut u16,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-A662166C-DC74-40B4-9BFA-8D3ED216FDE7
    fn OCILobFileIsOpen(
        svchp:      *mut OCISvcCtx,
        errhp:      *mut OCIError,
        filep:      *mut OCILobLocator,
        flag:       *mut u8
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-2E933BBA-BCE3-41F2-B8A2-4F9485F0BCB0
    fn OCILobFileOpen(
        svchp:      *mut OCISvcCtx,
        errhp:      *mut OCIError,
        filep:      *mut OCILobLocator,
        mode:       u8
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-507AC0EF-4CAB-437E-BB94-1FD77EDC1B5C
    fn OCILobFileSetName(
        envhp:      *mut OCIEnv,
        errhp:      *mut OCIError,
        filepp:     *mut *mut OCILobLocator,
        dir:        *const u8,
        dir_len:    u16,
        filename:   *const u8,
        name_len:   u16,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-E0FBF017-1B08-410C-9E53-F6E14008813A
    fn OCILobFreeTemporary(
        svchp:      *mut OCISvcCtx,
        errhp:      *mut OCIError,
        loc:        *mut OCILobLocator,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-ABB71585-172E-4F3E-A0CF-F70D709F2072
    fn OCILobGetChunkSize(
        svchp:      *mut OCISvcCtx,
        errhp:      *mut OCIError,
        loc:        *mut OCILobLocator,
        size:       *mut u32,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-D62200EF-FA60-4788-950F-0C0686D807FD
    fn OCILobGetContentType(
        envhp:      *mut OCIEnv,
        svchp:      *mut OCISvcCtx,
        errhp:      *mut OCIError,
        loc:        *mut OCILobLocator,
        ctx_type:   *mut u8,
        len:        *mut u32,
        mode:       u32
    )-> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-9BC0A78A-37CB-432F-AE2B-22C905608C4C
    fn OCILobGetLength2(
        svchp:      *mut OCISvcCtx,
        errhp:      *mut OCIError,
        loc:        *mut OCILobLocator,
        len:        *mut u64,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-5142710F-03AD-43D5-BBAB-6732B874E52E
    fn OCILobIsEqual(
        envhp:      *mut OCIEnv,
        loc1:       *const OCILobLocator,
        loc2:       *const OCILobLocator,
        flag:       *mut u8,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-FFF883CE-3B99-4319-A81C-A11F8740209E
    fn OCILobIsOpen(
        svchp:      *mut OCISvcCtx,
        errhp:      *mut OCIError,
        loc:        *mut OCILobLocator,
        flag:       *mut u8,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-071D8134-F9E7-4C5A-8E63-E90831FA7AC3
    fn OCILobIsTemporary(
        svchp:      *mut OCISvcCtx,
        errhp:      *mut OCIError,
        loc:        *mut OCILobLocator,
        flag:       *mut u8,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-DA1CD18B-7044-4E40-B1F4-4FCC1FCAB6C4
    fn OCILobLoadFromFile2(
        svchp:      *mut OCISvcCtx,
        errhp:      *mut OCIError,
        dst:        *mut OCILobLocator,
        src:        *mut OCILobLocator,
        amount:     u64,
        dst_off:    u64,
        src_off:    u64,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-F7887376-4B3C-430C-94A3-11FE96E26627
    fn OCILobLocatorAssign(
        svchp:      *mut OCISvcCtx,
        errhp:      *mut OCIError,
        src:        *const OCILobLocator,
        dst:        *const *mut OCILobLocator,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-4CA17A83-795F-43B2-8B76-611B13E4C8DE
    fn OCILobLocatorIsInit(
        envhp:      *mut OCIEnv,
        errhp:      *mut OCIError,
        src:        *const OCILobLocator,
        flag:       *mut u8,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-B007A3C7-999B-4AD7-8BF7-C6D14572F470
    fn OCILobOpen(
        svchp:      *mut OCISvcCtx,
        errhp:      *mut OCIError,
        loc:        *mut OCILobLocator,
        mode:       u8,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-6AC6E6DA-236B-4BF9-942F-9FCC4178FEDA
    fn OCILobRead2(
        svchp:      *mut OCISvcCtx,
        errhp:      *mut OCIError,
        loc:        *mut OCILobLocator,
        byte_cnt:   *mut u64,
        char_cnt:   *mut u64,
        offset:     u64,
        buf:        *mut u8,
        buf_len:    u64,
        piece:      u8,
        ctx:        *mut c_void,
        read_cb:    *const c_void,
        csid:       u16,
        csfrm:      u8,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-789C0971-76D5-4439-9379-E3DCE7885528
    fn OCILobSetContentType(
        envhp:      *mut OCIEnv,
        svchp:      *mut OCISvcCtx,
        errhp:      *mut OCIError,
        loc:        *mut OCILobLocator,
        ctype:      *const u8,
        len:        u32,
        mode:       u32,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-ABDB1543-1782-4216-AD80-55FA82CFF733
    fn OCILobTrim2(
        svchp:      *mut OCISvcCtx,
        errhp:      *mut OCIError,
        loc:        *mut OCILobLocator,
        len:        u64,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-77F056CA-9EEE-4550-8A8E-0155DF994DBE
    fn OCILobWrite2(
        svchp:      *mut OCISvcCtx,
        errhp:      *mut OCIError,
        loc:        *mut OCILobLocator,
        byte_cnt:   *mut u64,
        char_cnt:   *mut u64,
        offset:     u64,
        buf:        *const u8,
        buf_len:    u64,
        piece:      u8,
        ctx:        *mut c_void,
        write_cb:   *const c_void,
        csid:       u16,
        csfrm:      u8,
    ) -> i32;

    // https://docs.oracle.com/en/database/oracle/oracle-database/19/lnoci/lob-functions.html#GUID-87D3275A-B042-4991-B261-AB531BB83CA2
    fn OCILobWriteAppend2(
        svchp:      *mut OCISvcCtx,
        errhp:      *mut OCIError,
        loc:        *mut OCILobLocator,
        byte_cnt:   *mut u64,
        char_cnt:   *mut u64,
        buf:        *const u8,
        buf_len:    u64,
        piece:      u8,
        ctx:        *mut c_void,
        write_cb:   *const c_void,
        csid:       u16,
        csfrm:      u8,
    ) -> i32;
}

const OCI_ATTR_LOBEMPTY     : u32 = 45;
const OCI_ATTR_LOB_REMOTE   : u32 = 520;
const OCI_ATTR_LOB_TYPE     : u32 = 591;

const OCI_TEMP_BLOB         : u8 = 1;
const OCI_TEMP_CLOB         : u8 = 2;

const OCI_FILE_READONLY     : u8 = 1;
const OCI_LOB_READONLY      : u8 = 1;
const OCI_LOB_READWRITE     : u8 = 2;

const OCI_ONE_PIECE         : u8 = 0;
const OCI_FIRST_PIECE       : u8 = 1;
const OCI_NEXT_PIECE        : u8 = 2;
const OCI_LAST_PIECE        : u8 = 3;

const OCI_LOB_CONTENTTYPE_MAXSIZE   : usize = 128;

/// A marker trait for internal LOB descriptors - CLOB, NCLOB and BLOB.
pub trait InternalLob {}
impl InternalLob for OCICLobLocator {}
impl InternalLob for OCIBLobLocator {}

/// LOB locator.
///
pub struct LOB<'a,T>
    where T: DescriptorType<OCIType=OCILobLocator>
{
    locator: Descriptor<T>,
    conn: &'a dyn Conn,
    chunk_size: Cell<u32>,
}

impl<'a,T> Drop for LOB<'a,T>
    where T: DescriptorType<OCIType=OCILobLocator>
{
    fn drop(&mut self) {
        let mut is_open = mem::MaybeUninit::<u8>::uninit();
        let res = unsafe {
            OCILobIsOpen(self.conn.svc_ptr(), self.conn.err_ptr(), self.as_mut_ptr(), is_open.as_mut_ptr())
        };
        if res == OCI_SUCCESS && unsafe { is_open.assume_init() } != 0 {
            unsafe {
                OCILobClose(self.conn.svc_ptr(), self.conn.err_ptr(), self.as_mut_ptr());
            }
        }
        let mut is_temp = mem::MaybeUninit::<u8>::uninit();
        let res = unsafe {
            OCILobIsTemporary(self.conn.svc_ptr(), self.conn.err_ptr(), self.as_mut_ptr(), is_temp.as_mut_ptr())
        };
        if res == OCI_SUCCESS && unsafe { is_temp.assume_init() } != 0 {
            unsafe {
                OCILobFreeTemporary(self.conn.svc_ptr(), self.conn.err_ptr(), self.as_mut_ptr());
            }
        }
    }
}

impl<'a,T> LOB<'a,T>
    where T: DescriptorType<OCIType=OCILobLocator>
{
    pub(crate) fn as_ptr(&self) -> *const OCILobLocator {
        self.locator.get()
    }

    pub(crate) fn as_mut_ptr(&self) -> *mut OCILobLocator {
        self.locator.get()
    }

    pub(crate) fn make(locator: Descriptor<T>, conn: &'a dyn Conn) -> Self {
        Self { locator, conn, chunk_size: Cell::new(0) }
    }

    /// Creates a new uninitialized LOB.
    pub fn new(conn: &'a dyn Conn) -> Result<Self> {
        let locator = Descriptor::new(conn.env_ptr())?;
        Ok( Self { locator, conn, chunk_size: Cell::new(0) } )
    }

    /**
        Creates a new LOB locator that points to the same LOB data as the provided locator.

        For internal LOBs, the source locator's LOB data gets copied to the destination locator's
        LOB data only when the destination locator gets stored in the table. Therefore, issuing a
        flush of the object containing the destination locator copies the LOB data. For BFILEs,
        only the locator that refers to the operating system file is copied to the table; the
        operating system file is not copied.

        If the source locator is for an internal LOB that was enabled for buffering, and the source
        locator has been used to modify the LOB data through the LOB buffering subsystem, and the
        buffers have not been flushed since the write, then the source locator may not be assigned
        to the destination locator. This is because only one locator for each LOB can modify the LOB
        data through the LOB buffering subsystem.

        If the source LOB locator refers to a temporary LOB, the destination is made into a temporary
        LOB too. The source and the destination are conceptually different temporary LOBs. The source
        temporary LOB is deep copied, and a destination locator is created to refer to the new deep
        copy of the temporary LOB. Hence `is_equal` returns `false` when the new locator is created
        from the temporary one. However, as an optimization is made to minimize the number of deep
        copies, so the source and destination locators point to the same LOB until any modification
        is made through either LOB locator. Hence `is_equal` returns `true` right after the clone
        locator is created until the first modification. In both these cases, after new locator is
        constructed any changes to either LOB do not reflect in the other LOB.

        ## Failures

        Returns `Err` when a remote locator is passed to it.

        ## Example
        ```
        use sibyl::{ CLOB };

        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        # let stmt = conn.prepare("
        #     declare
        #         name_already_used exception; pragma exception_init(name_already_used, -955);
        #     begin
        #         execute immediate '
        #             create table test_lobs (
        #                 id       number generated always as identity,
        #                 text     clob,
        #                 data     blob,
        #                 ext_file bfile
        #             )
        #         ';
        #     exception
        #         when name_already_used then null;
        #     end;
        # ")?;
        # stmt.execute(&[])?;
        let stmt = conn.prepare("
            insert into test_lobs (text) values (empty_clob()) returning id, text into :id, :text
        ")?;
        let mut id1 : usize = 0;
        let mut lob1 = CLOB::new(&conn)?;
        stmt.execute_into(&[], &mut [ &mut id1, &mut lob1 ])?;

        let text = [
            "To see a World in a Grain of Sand\n",
            "And a Heaven in a Wild Flower\n",
            "Hold Infinity in the palm of your hand\n",
            "And Eternity in an hour\n"
        ];

        let written = lob1.append(text[0])?;
        assert_eq!(written, text[0].len());
        assert_eq!(lob1.len()?, text[0].len());

        let lob2 = lob1.clone()?;

        // They point to the same value and at this time they are completely in sync
        assert!(lob2.is_equal(&lob1)?);

        let written = lob2.append(text[1])?;
        assert_eq!(written, text[1].len());

        // Now they are out of sync
        assert!(!lob2.is_equal(&lob1)?);
        // At this time `lob1` is not yet aware that `lob2` added more text the LOB they "share".
        assert_eq!(lob2.len()?, text[0].len() + text[1].len());
        assert_eq!(lob1.len()?, text[0].len());

        let written = lob1.append(text[2])?;
        assert_eq!(written, text[2].len());

        // Now, after writing, `lob1` has caught up with `lob2` prior writing and added more text
        // on its own. But now it's `lob2` turn to lag behind and not be aware of the added text.
        assert_eq!(lob1.len()?, text[0].len() + text[1].len() + text[2].len());
        assert_eq!(lob2.len()?, text[0].len() + text[1].len());

        // Let's save `lob2` now. It is still only knows about `text[0]` and `text[1]` fragments.
        let stmt = conn.prepare("
            insert into test_lobs (text) values (:text) returning id, text into :id, :saved_text
        ")?;
        let mut saved_lob_id : usize = 0;
        let mut saved_lob = CLOB::new(&conn)?;
        let res = stmt.execute_into(&[ &lob2 ], &mut [ &mut saved_lob_id, &mut saved_lob ])?;

        // And thus `saved_lob` locator points to a distinct LOB value ...
        assert!(!saved_lob.is_equal(&lob2)?);
        // ... which has only the `text[0]` and `text[1]`
        assert_eq!(saved_lob.len()?, text[0].len() + text[1].len());

        let written = lob2.append(text[3])?;
        assert_eq!(written, text[3].len());

        assert_eq!(lob2.len()?, text[0].len() + text[1].len() + text[2].len() + text[3].len());
        assert_eq!(lob1.len()?, text[0].len() + text[1].len() + text[2].len());

        // As `saved_lob` points to the enturely different LOB ...
        assert!(!saved_lob.is_equal(&lob2)?);
        // ... it is not affected by `lob1` and `lob2` additions.
        assert_eq!(saved_lob.len()?, text[0].len() + text[1].len());
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn clone(&self) -> Result<Self> {
        let locator = Descriptor::new(self.conn.env_ptr())?;
        catch!{self.conn.err_ptr() =>
            OCILobLocatorAssign(self.conn.svc_ptr(), self.conn.err_ptr(), self.locator.get(), locator.as_ptr())
        }
        Ok( Self { locator, conn: self.conn, chunk_size: self.chunk_size.clone() } )
    }

    /**
        Determines whether the LOB locator belongs to a local database table or a remote database
        table. The value `true` indicates that the LOB locator is from a remote database table.
        The application must fetch the LOB descriptor from the database before querying this attribute.
    */
    pub fn is_remote(&self) -> Result<bool> {
        let is_remote: u8 = self.locator.get_attr(OCI_ATTR_LOB_REMOTE, self.conn.err_ptr())?;
        Ok( is_remote != 0 )
    }

    /// Returns the LOB's `SQLT` type, i.e. SQLT_CLOB, SQLT_BLOB or SQLT_BFILE.
    pub fn get_type(&self) -> Result<u16> {
        let lob_type: u16 = self.locator.get_attr(OCI_ATTR_LOB_TYPE, self.conn.err_ptr())?;
        Ok( lob_type )
    }

    /**
        Closes a previously opened internal or external LOB.

        Closing a LOB requires a round-trip to the server for both internal and external LOBs.
        For internal LOBs, `close` triggers other code that relies on the close call and for external
        LOBs (BFILEs), close actually closes the server-side operating system file.

        It is not required to close LOB explicitly as it will be automatically closed when Rust drops
        the locator.

        ## Failures
        - An error is returned if the internal LOB is not open.

        No error is returned if the BFILE exists but is not opened.

        When the error is returned, the LOB is no longer marked as open, but the transaction is successfully
        committed. Hence, all the changes made to the LOB and non-LOB data in the transaction are committed,
        but the domain and function-based indexing are not updated. If this happens, rebuild your functional
        and domain indexes on the LOB column.

        ## Example
        ```
        use sibyl::{ CLOB };

        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        # let stmt = conn.prepare("
        #     declare
        #         name_already_used exception; pragma exception_init(name_already_used, -955);
        #     begin
        #         execute immediate '
        #             create table test_lobs (
        #                 id       number generated always as identity,
        #                 text     clob,
        #                 data     blob,
        #                 ext_file bfile
        #             )
        #         ';
        #     exception
        #         when name_already_used then null;
        #     end;
        # ")?;
        # stmt.execute(&[])?;
        let stmt = conn.prepare("
            insert into test_lobs (text) values (empty_clob()) returning id, text into :id, :text
        ")?;
        let mut id : usize = 0;
        let mut lob = CLOB::new(&conn)?;
        stmt.execute_into(&[], &mut [ &mut id, &mut lob ])?;

        let text = [
            "Love seeketh not itself to please,\n",
            "Nor for itself hath any care,\n",
            "But for another gives its ease,\n",
            "And builds a Heaven in Hell's despair.\n",
        ];

        lob.open()?;
        lob.append(text[0])?;
        lob.append(text[1])?;
        lob.append(text[2])?;
        lob.append(text[3])?;
        lob.close()?;

        assert_eq!(lob.len()?, text[0].len() + text[1].len() + text[2].len() + text[3].len());
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn close(&self) -> Result<()> {
        catch!{self.conn.err_ptr() =>
            OCILobClose(self.conn.svc_ptr(), self.conn.err_ptr(), self.as_mut_ptr())
        }
        Ok(())
    }

    /**
        Returns the length of a LOB.

        For character LOBs, it is the number of characters; for binary LOBs and BFILEs, it is the number of
        bytes in the LOB.

        Notes:
        - If the LOB is NULL, the length is undefined.
        - The length of a BFILE includes the EOF, if it exists.
        - The length of an empty internal LOB is zero.
        - Any zero-byte or space fillers in the LOB written by previous calls to `erase` or `write` are also
          included in the length count.
    */
    pub fn len(&self) -> Result<usize> {
        let mut len = mem::MaybeUninit::<u64>::uninit();
        catch!{self.conn.err_ptr() =>
            OCILobGetLength2(self.conn.svc_ptr(), self.conn.err_ptr(), self.as_mut_ptr(), len.as_mut_ptr())
        }
        Ok( unsafe { len.assume_init() } as usize )
    }

    /**
        Compares the given LOB or BFILE locators for equality. Two LOB or BFILE locators are equal
        if and only if they both refer to the same LOB or BFILE value.

        Two NULL locators are considered not equal by this function.
        ## Example
        ```
        use sibyl::{ CLOB };

        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        # let stmt = conn.prepare("
        #     declare
        #         name_already_used exception; pragma exception_init(name_already_used, -955);
        #     begin
        #         execute immediate '
        #             create table test_lobs (
        #                 id       number generated always as identity,
        #                 text     clob,
        #                 data     blob,
        #                 ext_file bfile
        #             )
        #         ';
        #     exception
        #         when name_already_used then null;
        #     end;
        # ")?;
        # stmt.execute(&[])?;
        let stmt = conn.prepare("
            insert into test_lobs (text) values (empty_clob()) returning id, text into :id, :text
        ")?;
        let mut id : usize = 0;
        let mut lob = CLOB::new(&conn)?;
        stmt.execute_into(&[], &mut [ &mut id, &mut lob ])?;

        let text = "
            To Mercy, Pity, Peace, and Love
            All pray in their distress;
            And to these virtues of delight
            Return their thankfulness.
        ";
        lob.open()?;
        lob.append(text)?;
        lob.close()?;

        let stmt = conn.prepare("
            select text from test_lobs where id = :id
        ")?;
        let rows = stmt.query(&[ &id ])?;
        let row = rows.next()?.expect("selected row");
        let selected_lob = row.get::<CLOB>(0)?.expect("CLOB locator");

        assert_eq!(lob.len()?, selected_lob.len()?);
        // Every once in a while `is_equal` in this test would return `false`...
        assert!(selected_lob.is_equal(&lob)?);
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn is_equal<U>(&self, other: &LOB<'a,U>) -> Result<bool>
        where U: DescriptorType<OCIType=OCILobLocator>
    {
        let mut flag = mem::MaybeUninit::<u8>::uninit();
        catch!{self.conn.err_ptr() =>
            OCILobIsEqual(self.conn.env_ptr(), self.as_mut_ptr(), other.as_mut_ptr(), flag.as_mut_ptr())
        }
        let flag = unsafe { flag.assume_init() };
        println!("is_equal={}", flag);
        Ok( flag != 0 )
    }

    /**
        Returns `true` if the internal LOB is open or if the BFILE was opened using the input locator.

        If the input BFILE locator was never passed to `open` or `open_file`  the BFILE is considered
        not to be opened by this BFILE locator. However, a different BFILE locator may have opened
        the BFILE. Multiple opens can be performed on the same BFILE using different locators. In other
        words, openness is associated with a specific locator for BFILEs.

        For internal LOBs openness is associated with the LOB, not with the locator. If locator1 opened
        the LOB, then locator2 also sees the LOB as open.

        For internal LOBs, this call requires a server round-trip because it checks the state on the
        server to see if the LOB is open. For external LOBs (BFILEs), this call also requires a round-trip
        because the operating system file on the server side must be checked to see if it is open.
    */
    pub fn is_open(&self) -> Result<bool> {
        let mut flag = mem::MaybeUninit::<u8>::uninit();
        catch!{self.conn.err_ptr() =>
            OCILobIsOpen(self.conn.svc_ptr(), self.conn.err_ptr(), self.as_mut_ptr(), flag.as_mut_ptr())
        }
        Ok( unsafe { flag.assume_init() } != 0 )
    }

    /**
        Returns `true` if the given LOB or BFILE locator is initialized.

        InternalLob LOB locators can be initialized by one of these methods:
        - Selecting a non-NULL LOB into the locator
        - Pinning an object that contains a non-NULL LOB attribute
        - Setting the locator to empty by calling `empty`

        BFILE locators can be initialized by one of these methods:
        - Selecting a non-NULL BFILE into the locator
        - Pinning an object that contains a non-NULL BFILE attribute
        - Calling `set_file_name`
    */
    pub fn is_initialized(&self) -> Result<bool> {
        let mut flag = mem::MaybeUninit::<u8>::uninit();
        catch!{self.conn.err_ptr() =>
            OCILobLocatorIsInit(self.conn.env_ptr(), self.conn.err_ptr(), self.as_ptr(), flag.as_mut_ptr())
        }
        Ok( unsafe { flag.assume_init() } != 0 )
    }

    /**
        Opens a LOB, internal or external, only for reading.

        Opening a LOB requires a round-trip to the server for both internal and external LOBs. For internal
        LOBs, the open triggers other code that relies on the open call. For external LOBs (BFILEs), open
        requires a round-trip because the actual operating system file on the server side is being opened.

        ## Failures
        - It is an error to open the same LOB twice.
        - If a user tries to write to a LOB that was opened in read-only mode, an error is returned.
    */
    pub fn open_readonly(&self) -> Result<()> {
        catch!{self.conn.err_ptr() =>
            OCILobOpen(self.conn.svc_ptr(), self.conn.err_ptr(), self.as_mut_ptr(), OCI_LOB_READONLY)
        }
        Ok(())
    }

    /**
        Returns the character set form of the input CLOB or NCLOB locator. If the input locator is for a BLOB
        or a BFILE, it returns `CharSetForm::Undefined` because there is no concept of a character set for binary
        LOBs or binary files.
    */
    pub fn charset_form(&self) -> Result<CharSetForm> {
        let mut csform = mem::MaybeUninit::<u8>::uninit();
        catch!{self.conn.err_ptr() =>
            OCILobCharSetForm(self.conn.env_ptr(), self.conn.err_ptr(), self.as_ptr(), csform.as_mut_ptr())
        }
        let res = match unsafe { csform.assume_init() } {
            SQLCS_IMPLICIT => CharSetForm::Implicit,
            SQLCS_NCHAR    => CharSetForm::NChar,
            _              => CharSetForm::Undefined
        };
        Ok( res )
    }

    /**
        Returns the LOB locator's database character set ID. If the input locator is for a BLOB or a BFILE,
        it returns 0 because there is no concept of a character set for binary LOBs or binary files.
    */
    pub fn charset_id(&self) -> Result<u16> {
        let mut csid = mem::MaybeUninit::<u16>::uninit();
        catch!{self.conn.err_ptr() =>
            OCILobCharSetId(self.conn.env_ptr(), self.conn.err_ptr(), self.as_ptr(), csid.as_mut_ptr())
        }
        Ok( unsafe { csid.assume_init() } )
    }

    /**
        For CLOBs and NCLOBs, if you do not pass `char_len`, then `char_len` is calculated internally as
        `byte_len/max char width`, so if max char width is 4, `char_len` is calculated as `byte_len/4`.
        OCILobRead2() does not calculate how many bytes are required for each character. Instead, OCILobRead2()
        fetches in the worst case the number of characters that can fit in `byte_len`. To fill the buffer, check
        the returned value to see how much of the buffer is filled, then call OCILobRead2() again to fetch the
        remaining bytes.
    */
    fn read_piece(&self, piece: u8, piece_size: usize, offset: usize, byte_len: &mut usize, char_len: usize, char_form: u8, buf: &mut Vec<u8>) -> Result<bool> {
        buf.reserve(piece_size);
        let mut num_bytes = *byte_len as u64;
        let mut num_chars = char_len as u64;
        let res = unsafe {
            OCILobRead2(
                self.conn.svc_ptr(), self.conn.err_ptr(), self.as_mut_ptr(),
                &mut num_bytes, &mut num_chars, (offset + 1) as u64,
                buf.as_mut_ptr().add(buf.len()), piece_size as u64, piece,
                ptr::null_mut::<c_void>(), ptr::null::<c_void>(),
                AL32UTF8, char_form
            )
        };
        match res {
            OCI_ERROR | OCI_INVALID_HANDLE => {
                Err( Error::oci(self.conn.err_ptr(), res) )
            }
            _ => {
                unsafe {
                    buf.set_len(buf.len() + num_bytes as usize);
                }
                *byte_len = num_bytes as usize;
                Ok( res == OCI_NEED_DATA )
            }
        }
    }
}

impl<'a,T> LOB<'a,T>
    where T: DescriptorType<OCIType=OCILobLocator> + InternalLob
{
    /**
        Creates a new empty LOB.

        The locator can then be used as a bind variable for an INSERT or UPDATE statement
        to initialize the LOB to empty. Once the LOB is empty, `write` can be called to
        populate the LOB with data.

        ## Example
        ```
        use sibyl::{ CLOB };

        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        # let stmt = conn.prepare("
        #     declare
        #         name_already_used exception; pragma exception_init(name_already_used, -955);
        #     begin
        #         execute immediate '
        #             create table test_lobs (
        #                 id       number generated always as identity,
        #                 text     clob,
        #                 data     blob,
        #                 ext_file bfile
        #             )
        #         ';
        #     exception
        #         when name_already_used then null;
        #     end;
        # ")?;
        # stmt.execute(&[])?;
        let stmt = conn.prepare("
            insert into test_lobs (text) values (:text) returning id into :id
        ")?;
        let mut id : usize = 0;
        let lob = CLOB::empty(&conn)?;
        stmt.execute_into(&[ &lob ], &mut [ &mut id ])?;
        # assert!(id > 0);
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn empty(conn: &'a dyn Conn) -> Result<Self> {
        let locator = Descriptor::new(conn.env_ptr())?;
        locator.set_attr(OCI_ATTR_LOBEMPTY, 0u32, conn.err_ptr())?;
        Ok( Self { locator, conn, chunk_size: Cell::new(0) } )
    }

    /**
        Sets the internal LOB locator to empty.

        The locator can then be used as a bind variable for an INSERT or UPDATE statement
        to initialize the LOB to empty. Once the LOB is empty, `write` can be called to
        populate the LOB with data.
    */
    pub fn clear(&self) -> Result<()> {
        self.locator.set_attr(OCI_ATTR_LOBEMPTY, 0u32, self.conn.err_ptr())
    }

    /**
        Appends another LOB value at the end of this LOB.

        ## Example
        ```
        use sibyl::{CLOB, Cache, CharSetForm};

        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let text1 = "
            The sun does arise,
            And make happy the skies.
            The merry bells ring
            To welcome the Spring.
        ";
        let text2 = "
            The sky-lark and thrush,
            The birds of the bush,
            Sing louder around,
            To the bells’ cheerful sound.
            While our sports shall be seen
            On the Ecchoing Green.
        ";
        let lob1 = CLOB::temp(&conn, CharSetForm::Implicit, Cache::No)?;
        lob1.append(text1)?;
        assert_eq!(lob1.len()?, text1.len());

        let lob2 = CLOB::temp(&conn, CharSetForm::Implicit, Cache::No)?;
        lob2.append(text2)?;
        // Cannot use `len` shortcut with `text2` because of `RIGHT SINGLE QUOTATION MARK`
        assert_eq!(lob2.len()?, text2.chars().count());

        lob1.append_lob(&lob2)?;
        assert_eq!(lob1.len()?, text1.len() + text2.chars().count());
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn append_lob(&self, lob: &Self) -> Result<()> {
        catch!{self.conn.err_ptr() =>
            OCILobAppend(self.conn.svc_ptr(), self.conn.err_ptr(), self.as_mut_ptr(), lob.as_ptr())
        }
        Ok(())
    }

    /**
        Copies all or a portion of another LOB value.

        If the data exists at the destination's start position, it is overwritten with the source data.

        If the destination's start position is beyond the end of the current data, zero-byte fillers (for BLOBs)
        or spaces (for CLOBs) are written into the destination LOB from the end of the current data to the beginning
        of the newly written data from the source.

        The destination LOB is extended to accommodate the newly written data if it extends beyond the current
        length of the destination LOB.

        LOB buffering must not be enabled for either locator.

        Notes:
        - To copy the entire source LOB specify `amount` as `std::usize::MAX`.
        - `offset` and `src_offset` - the number of characters (character LOB) or bytes (binary LOB) from the
          beginning of the LOB - start at 0.
        - You can call `len` to determine the length of the source LOB.

        ## Example
        ```
        use sibyl::{CLOB, Cache, CharSetForm};

        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let text = "
        O Nightingale, that on yon bloomy Spray
        Warbl'st at eeve, when all the Woods are still,
        Thou with fresh hope the Lovers heart dost fill,
        While the jolly hours lead on propitious May,

        Thy liquid notes that close the eye of Day,
        First heard before the shallow Cuccoo's bill
        Portend success in love; O if Jove's will
        Have linkt that amorous power to thy soft lay,

        ...........................................
        ...........................................
        ..........................................

        For my relief; yet hadst no reason why,
        Whether the Muse, or Love call thee his mate,
        Both them I serve, and of their train am I.
        ";
        let lob1 = CLOB::temp(&conn, CharSetForm::Implicit, Cache::No)?;
        lob1.append(text)?;

        let lost_text = "
        Now timely sing, ere the rude Bird of Hate
        Foretell my hopeles doom, in som Grove ny:
        As thou from yeer to yeer hast sung too late
        ";
        let lob2 = CLOB::temp(&conn, CharSetForm::Implicit, Cache::No)?;
        lob2.append(lost_text)?;

        // Set source offset to 1 to skip leading \n:
        lob1.copy(&lob2, 1, 131, 364)?;

        // Read back the overwriten fragment.
        // Start with the leading \n, to make comparison easier:
        let mut fragment = String::with_capacity(135*4);
        lob1.read(363, 132, &mut fragment)?;

        assert_eq!(fragment, lost_text);

        // ASCII only, so we can use `len` as a "shortcut".
        let text_len = lob1.len()?;
        // Recall that the buffer needs to be allocated for the worst case
        let mut sonnet = String::with_capacity(text_len*4);
        lob1.read(0, text_len, &mut sonnet)?;

        let orig = "
        O Nightingale, that on yon bloomy Spray
        Warbl'st at eeve, when all the Woods are still,
        Thou with fresh hope the Lovers heart dost fill,
        While the jolly hours lead on propitious May,

        Thy liquid notes that close the eye of Day,
        First heard before the shallow Cuccoo's bill
        Portend success in love; O if Jove's will
        Have linkt that amorous power to thy soft lay,

        Now timely sing, ere the rude Bird of Hate
        Foretell my hopeles doom, in som Grove ny:
        As thou from yeer to yeer hast sung too late

        For my relief; yet hadst no reason why,
        Whether the Muse, or Love call thee his mate,
        Both them I serve, and of their train am I.
        ";

        assert_eq!(sonnet, orig);
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn copy(&self, src: &Self, src_offset: usize, amount: usize, offset: usize) -> Result<()> {
        catch!{self.conn.err_ptr() =>
            OCILobCopy2(
                self.conn.svc_ptr(), self.conn.err_ptr(),
                self.as_mut_ptr(), src.as_mut_ptr(),
                amount as u64, (offset + 1) as u64, (src_offset + 1) as u64
            )
        }
        Ok(())
    }

    /**
        Loads and copies all or a portion of the file into an internal LOB.

        The data are copied from the source BFILE to the destination internal LOB (BLOB or CLOB). No character set
        conversions are performed when copying the BFILE data to a CLOB or NCLOB. Also, when binary data is loaded
        into a BLOB, no character set conversions are performed. Therefore, the BFILE data must be in the same
        character set as the LOB in the database. No error checking is performed to verify this.

        If the data exists at the destination's start position, it is overwritten with the source data. If the
        destination's start position is beyond the end of the current data, zero-byte fillers (for BLOBs) or spaces
        (for CLOBs) are written into the destination LOB from the end of the data to the beginning of the newly
        written data from the source. The destination LOB is extended to accommodate the newly written data if it
        extends beyond the current length of the destination LOB.

        ## Failures
        - This function throws an error when a remote locator is passed to it.
        - It is an error to try to copy from a NULL BFILE.

        ## Example
        Note that this example assumes that the demo directories were created (@?/demo/schema/mk_dir) and
        the test user has permissions to read them (see `etc/create_sandbox.sql`)

        ```
        use sibyl::{CLOB, Cache, CharSetForm, BFile};

        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let file = BFile::new(&conn)?;
        file.set_file_name("MEDIA_DIR", "hello_world.txt")?;
        assert!(file.file_exists()?, "hello_world.txt");

        // To load this file into a temporary CLOB Oracle expects it to be UTF-16 BE encoded.

        // File must be opened first
        file.open_readonly()?;
        assert!(file.is_open()?);

        let lob = CLOB::temp(&conn, CharSetForm::Implicit, Cache::No)?;
        lob.load_from_file(&file, 0, 28, 0)?;
        file.close()?;

        assert_eq!(13, lob.len()?);

        let mut text = String::with_capacity(64);
        // Even though we are asking for more characters than the LOB has it should only return what it has
        lob.read(0, 20, &mut text)?;

        assert_eq!(text, "Hello, World!");

        // Do it again in Cyrillic. No need to erase it. We'll just load new content over the old one
        file.set_file_name("MEDIA_DIR", "hello_world_cyrillic.txt")?;
        assert!(file.file_exists()?, "hello_world_cyrillic.txt");
        file.open_readonly()?;
        assert!(file.is_open()?);

        lob.load_from_file(&file, 0, 34, 0)?;
        file.close()?;

        assert_eq!(16, lob.len()?);

        text.clear();
        lob.read(0, 20, &mut text)?;

        assert_eq!(text, "Здравствуй, Мир!");

        // And the supplemental plane
        file.set_file_name("MEDIA_DIR", "hello_supplemental.txt")?;
        assert!(file.file_exists()?, "hello_supplemental.txt");
        file.open_readonly()?;
        assert!(file.is_open()?);

        // As the text in the "cyrillic" test was longer...
        lob.trim(0)?;
        lob.load_from_file(&file, 0, 18, 0)?;
        file.close()?;

        // Note that "char count" for all characters used in this test is defined in Unicode as 2.
        assert_eq!(8, lob.len()?);

        text.clear();
        lob.read(0, 20, &mut text)?;

        // Depending on the font you use, you might or might not see the the test characters
        // They are represented in "Cousine" for example
        assert_eq!(text, "🚲🛠📬🎓");

        // Same set of tests, but now with a CLOB from the test table
        # let stmt = conn.prepare("
        #     declare
        #         name_already_used exception; pragma exception_init(name_already_used, -955);
        #     begin
        #         execute immediate '
        #             create table test_lobs (
        #                 id       number generated always as identity,
        #                 text     clob,
        #                 data     blob,
        #                 ext_file bfile
        #             )
        #         ';
        #     exception
        #         when name_already_used then null;
        #     end;
        # ")?;
        # stmt.execute(&[])?;
        let stmt = conn.prepare("
            insert into test_lobs (text) values (empty_clob()) returning text into :text
        ")?;
        let mut lob = CLOB::new(&conn)?;
        stmt.execute_into(&[], &mut [ &mut lob ])?;

        lob.open()?;

        file.set_file_name("MEDIA_DIR", "hello_world.txt")?;
        assert!(file.file_exists()?, "hello_world.txt");
        file.open_readonly()?;
        assert!(file.is_open()?);

        lob.load_from_file(&file, 0, 28, 0)?;
        file.close()?;

        assert_eq!(13, lob.len()?);

        text.clear();
        lob.read(0, 20, &mut text)?;

        assert_eq!(text, "Hello, World!");

        file.set_file_name("MEDIA_DIR", "hello_world_cyrillic.txt")?;
        assert!(file.file_exists()?, "hello_world_cyrillic.txt");
        file.open_readonly()?;
        assert!(file.is_open()?);

        lob.load_from_file(&file, 0, 34, 0)?;
        file.close()?;

        assert_eq!(16, lob.len()?);

        text.clear();
        lob.read(0, 20, &mut text)?;

        assert_eq!(text, "Здравствуй, Мир!");

        file.set_file_name("MEDIA_DIR", "hello_supplemental.txt")?;
        assert!(file.file_exists()?, "hello_supplemental.txt");
        file.open_readonly()?;
        assert!(file.is_open()?);

        lob.trim(0)?;
        lob.load_from_file(&file, 0, 18, 0)?;
        file.close()?;

        assert_eq!(8, lob.len()?);

        text.clear();
        lob.read(0, 20, &mut text)?;

        assert_eq!(text, "🚲🛠📬🎓");
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn load_from_file(&self, src: &BFile<'a>, src_offset: usize, amount: usize, offset: usize) -> Result<()> {
        catch!{self.conn.err_ptr() =>
            OCILobLoadFromFile2(
                self.conn.svc_ptr(), self.conn.err_ptr(),
                self.as_mut_ptr(), src.as_mut_ptr(),
                amount as u64, (offset + 1) as u64, (src_offset + 1) as u64
            )
        }
        Ok(())
    }

    /**
        Erases a specified portion of the internal LOB data starting at a specified offset.
        Returns the actual number of characters or bytes erased.

        For BLOBs, erasing means that zero-byte fillers overwrite the existing LOB value.
        For CLOBs, erasing means that spaces overwrite the existing LOB value.
    */
    pub fn erase(&self, offset: usize, amount: usize) -> Result<usize> {
        let mut count: u64 = amount as u64;
        catch!{self.conn.err_ptr() =>
            OCILobErase2(
                self.conn.svc_ptr(), self.conn.err_ptr(), self.as_mut_ptr(),
                &mut count as *mut u64, (offset + 1) as u64
            )
        }
        Ok( count as usize )
    }

    /**
        Returns the chunk size (in bytes) of a LOB.

        For LOBs with storage parameter BASICFILE, chunk size is amount of a chunk's space that is
        used to store the internal LOB value. This is the amount that users should use when reading
        or writing the LOB value. If possible, users should start their writes at chunk boundaries,
        such as the beginning of a chunk, and write a chunk at a time.

        For LOBs with storage parameter SECUREFILE, chunk size is an advisory size and is provided
        for backward compatibility.

        When creating a table that contains an internal LOB, the user can specify the chunking factor,
        which can be a multiple of Oracle Database blocks. This corresponds to the chunk size used by
        the LOB data layer when accessing and modifying the LOB value. Part of the chunk is used to store
        system-related information, and the rest stores the LOB value. This function returns the amount
        of space used in the LOB chunk to store the LOB value. Performance is improved if the application
        issues read or write requests using a multiple of this chunk size. For writes, there is an added
        benefit because LOB chunks are versioned and, if all writes are done on a chunk basis, no extra
        versioning is done or duplicated. Users could batch up the write until they have enough for a chunk
        instead of issuing several write calls for the same chunk.
    */
    pub fn chunk_size(&self) -> Result<usize> {
        let mut size: u32 = self.chunk_size.get();
        if size == 0 {
            catch!{self.conn.err_ptr() =>
                OCILobGetChunkSize(self.conn.svc_ptr(), self.conn.err_ptr(), self.as_mut_ptr(), &mut size)
            }
            self.chunk_size.replace(size);
        }
        Ok( size as usize )
    }

    /**
        Returns he user-specified content type string for the data in a SecureFile, if set.

        This function only works on SecureFiles.
    */
    pub fn content_type(&self) -> Result<String> {
        let mut txt = String::with_capacity(OCI_LOB_CONTENTTYPE_MAXSIZE);
        let mut len = txt.capacity() as u32;
        catch!{self.conn.err_ptr() =>
            OCILobGetContentType(
                self.conn.env_ptr(), self.conn.svc_ptr(), self.conn.err_ptr(), self.as_mut_ptr(),
                txt.as_mut_vec().as_mut_ptr(), &mut len, 0
            )
        }
        unsafe {
            txt.as_mut_vec().set_len(len as usize);
        }
        Ok( txt )
    }

    /**
        Sets a content type string for the data in the SecureFile to something that can be used by an application.

        This function only works on SecureFiles.
    */
    pub fn set_content_type(&self, content_type: &str) -> Result<()> {
        let len = content_type.len() as u32;
        let ptr = if len > 0 { content_type.as_ptr() } else { ptr::null::<u8>() };
        catch!{self.conn.err_ptr() =>
            OCILobSetContentType(
                self.conn.env_ptr(), self.conn.svc_ptr(), self.conn.err_ptr(), self.as_mut_ptr(),
                ptr, len, 0
            )
        }
        Ok(())
    }

    /**
        Opens an internal LOB for reading and writing.

        Opening a LOB requires a round-trip to the server for both internal and external LOBs. For internal
        LOBs, the open triggers other code that relies on the open call. For external LOBs (BFILEs), open
        requires a round-trip because the actual operating system file on the server side is being opened.

        It is not mandatory that you wrap all LOB operations inside the open and close calls. However, if you
        open a LOB, then you must close it before you commit your transaction. When an internal LOB is closed,
        it updates the functional and domain indexes on the LOB column. It is an error to commit the transaction
        before closing all opened LOBs that were opened by the transaction.

        When the error is returned, the LOB is no longer marked as open, but the transaction is successfully
        committed. Hence, all the changes made to the LOB and non-LOB data in the transaction are committed,
        but the domain and function-based indexing are not updated. If this happens, rebuild your functional
        and domain indexes on the LOB column.

        It is not necessary to open a LOB to perform operations on it. When using function-based indexes,
        extensible indexes or context, and making multiple calls to update or write to the LOB, you should
        first call `open`, then update the LOB as many times as you want, and finally call `close`. This
        sequence of operations ensures that the indexes are only updated once at the end of all the write
        operations instead of once for each write operation.

        If you do not wrap your LOB operations inside the open or close API, then the functional
        and domain indexes are updated each time you write to the LOB. This can adversely affect
        performance. If you have functional or domain indexes, Oracle recommends that you enclose
        write operations to the LOB within the open or close statements.

        ## Failures
        - It is an error to open the same LOB twice.
    */
    pub fn open(&self) -> Result<()> {
        catch!{self.conn.err_ptr() =>
            OCILobOpen(self.conn.svc_ptr(), self.conn.err_ptr(), self.as_mut_ptr(), OCI_LOB_READWRITE)
        }
        Ok(())
    }

    /**
        Truncates the LOB value to a shorter length.

        For character LOBs, `new_len` is the number of characters; for binary LOBs and BFILEs, it is the number
        of bytes in the LOB.
    */
    pub fn trim(&self, new_len: usize) -> Result<()> {
        catch!{self.conn.err_ptr() =>
            OCILobTrim2(self.conn.svc_ptr(), self.conn.err_ptr(), self.as_mut_ptr(), new_len as u64)
        }
        Ok(())
    }

    fn write_piece(&self, piece: u8, offset: usize, data: &[u8]) -> Result<usize> {
        let mut byte_cnt = if piece == OCI_ONE_PIECE { data.len() as u64 } else { 0u64 };
        let mut char_cnt = 0u64;
        let charset_form = self.charset_form()? as u8;
        catch!{self.conn.err_ptr() =>
            OCILobWrite2(
                self.conn.svc_ptr(), self.conn.err_ptr(), self.as_mut_ptr(),
                &mut byte_cnt, &mut char_cnt, (offset + 1) as u64,
                data.as_ptr(), data.len() as u64, piece,
                ptr::null_mut::<c_void>(), ptr::null::<c_void>(),
                AL32UTF8, charset_form
            )
        }
        Ok( byte_cnt as usize )
    }

    fn append_piece(&self, piece: u8, data: &[u8]) -> Result<usize> {
        let mut byte_cnt = if piece == OCI_ONE_PIECE { data.len() as u64 } else { 0u64 };
        let mut char_cnt = 0u64;
        let charset_form = self.charset_form()? as u8;
        catch!{self.conn.err_ptr() =>
            OCILobWriteAppend2(
                self.conn.svc_ptr(), self.conn.err_ptr(), self.as_mut_ptr(),
                &mut byte_cnt, &mut char_cnt,
                data.as_ptr(), data.len() as u64, piece,
                ptr::null_mut::<c_void>(), ptr::null::<c_void>(),
                AL32UTF8, charset_form
            )
        }
        Ok( byte_cnt as usize )
    }
}

impl<'a> LOB<'a,OCICLobLocator> {
    /**
        Creates an empty temporary CLOB or NCLOB.

        The LOB is feed automatically either when a LOB goes out of scope or at the end of the session whichever comes first.
    */
    pub fn temp(conn: &'a dyn Conn, csform: CharSetForm, cache: Cache) -> Result<Self> {
        let locator = Descriptor::new(conn.env_ptr())?;
        catch!{conn.err_ptr() =>
            OCILobCreateTemporary(
                conn.svc_ptr(), conn.err_ptr(), locator.get(),
                OCI_DEFAULT as u16, csform as u8, OCI_TEMP_CLOB, cache as u8, OCI_DURATION_SESSION
            )
        }
        Ok( Self { locator, conn, chunk_size: Cell::new(0) } )
    }

    /**
        Returns `true` if the LOB locator is for an NCLOB.

        ## Example
        ```
        use sibyl::{CLOB, Cache, CharSetForm};

        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let lob = CLOB::temp(&conn, CharSetForm::NChar, Cache::No)?;

        assert!(lob.is_nclob()?);
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn is_nclob(&self) -> Result<bool> {
        let csform = self.charset_form()?;
        Ok( csform as u8 == CharSetForm::NChar as u8 )
    }

    /**
        Writes a buffer into a LOB.

        Returns the number of bytes written to the database.

        ## Example
        ```
        use sibyl::{CLOB, Cache, CharSetForm};

        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let lob = CLOB::temp(&conn, CharSetForm::Implicit, Cache::No)?;
        let written = lob.write(3, "Hello, World!")?;
        assert_eq!(13, written);
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn write(&self, offset: usize, text: &str) -> Result<usize> {
        self.write_piece(OCI_ONE_PIECE, offset, text.as_bytes())
    }

    /**
        Starts piece-wise writing into a LOB.

        The application must call `write_next` to write more pieces into the LOB.
        `write_last` terminates the piecewise write.

        Returns the number of bytes written to the database for the first piece.

        ## Example
        ```
        use sibyl::{ CLOB, CharSetForm, Cache };

        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let lob = CLOB::temp(&conn, CharSetForm::Implicit, Cache::No)?;
        lob.open()?;
        let chunk_size = lob.chunk_size()?;
        let data = vec![42u8;chunk_size];
        let text = std::str::from_utf8(&data)?;

        let written = lob.write_first(0, text)?;
        assert_eq!(written, chunk_size);
        for i in 0..8 {
            let written = lob.write_next(text)?;
            assert_eq!(written, chunk_size);
        }
        let written = lob.write_last(text)?;
        assert_eq!(written, chunk_size);

        assert_eq!(lob.len()?, chunk_size * 10);
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn write_first(&self, offset: usize, text: &str) -> Result<usize> {
        self.write_piece(OCI_FIRST_PIECE, offset, text.as_bytes())
    }

    /**
        Continues piece-wise writing into a LOB.

        The application must call `write_next` to write more pieces into the LOB.
        `write_last` terminates the piecewise write.

        Returns the number of bytes written to the database for this piece.
    */
    pub fn write_next(&self, text: &str) -> Result<usize> {
        self.write_piece(OCI_NEXT_PIECE, 0, text.as_bytes())
    }

    /**
        Terminates piece-wise writing into a LOB.

        Returns the number of bytes written to the database for the last piece.
    */
    pub fn write_last(&self, text: &str) -> Result<usize> {
        self.write_piece(OCI_LAST_PIECE, 0, text.as_bytes())
    }

    /**
        Writes data starting at the end of a LOB.

        Returns the number of bytes written to the database.

        ## Example
        ```
        use sibyl::{CLOB, Cache, CharSetForm};

        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let lob = CLOB::temp(&conn, CharSetForm::Implicit, Cache::No)?;
        let written = lob.append("Hello, World!")?;
        assert_eq!(13, written);
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn append(&self, text: &str) -> Result<usize> {
        self.append_piece(OCI_ONE_PIECE, text.as_bytes())
    }

    /**
        Starts piece-wise writing at the end of a LOB.

        The application must call `append_next` to write more pieces into the LOB.
        `append_last` terminates the piecewise write.

        Returns the number of bytes written to the database for the first piece.

        ## Example
        ```
        use sibyl::{ CLOB, CharSetForm, Cache };

        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let lob = CLOB::temp(&conn, CharSetForm::Implicit, Cache::No)?;
        lob.open()?;
        let chunk_size = lob.chunk_size()?;
        let data = vec![42u8;chunk_size];
        let text = std::str::from_utf8(&data)?;

        let written = lob.append_first(text)?;
        assert_eq!(written, chunk_size);
        for i in 0..8 {
            let written = lob.append_next(text)?;
            assert_eq!(written, chunk_size);
        }
        let written = lob.append_last(text)?;
        assert_eq!(written, chunk_size);

        assert_eq!(lob.len()?, chunk_size * 10);
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn append_first(&self, text: &str) -> Result<usize> {
        self.append_piece(OCI_FIRST_PIECE, text.as_bytes())
    }

    /**
        Continues piece-wise writing at the end of a LOB.

        The application must call `append_next` to write more pieces into the LOB.
        `append_last` terminates the piecewise write.

        Returns the number of bytes written to the database for this piece.
    */
    pub fn append_next(&self, text: &str) -> Result<usize> {
        self.append_piece(OCI_NEXT_PIECE, text.as_bytes())
    }

    /**
        Terminates piece-wise writing at the end of a LOB.

        Returns the number of bytes written to the database for the last piece.
    */
    pub fn append_last(&self, text: &str) -> Result<usize> {
        self.append_piece(OCI_LAST_PIECE, text.as_bytes())
    }

    /**
        Reads specified number of characters from this LOB, appending them to `buf`.
        If successful, this function will return the total number of bytes read.

        Note that the specified number of characters to read is only honored if the buffer is large enough to accept that many.
        The upper bound on the number of returned characters is calculated by Oracle as `buf_remaining_capacity/max_char_width`.
        Because of the AL32UTF8 encoding configured in *sibyl* environments the `max_char_width` in that calculation is 4. The
        underlying OCI call - OCILobRead2() - does not calculate how many bytes are required for **each** character. Instead,
        it fetches the number of characters that even in **the worst case** - for example when all characters in the requested
        fragment are from the supplementary planes - would fit in the provided buffer.

        ## Example
        ```
        use sibyl::{CLOB, Cache, CharSetForm};

        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let lob = CLOB::temp(&conn, CharSetForm::Implicit, Cache::No)?;
        let written = lob.write(3, "Hello, World!")?;
        assert_eq!(13, written);

        let mut buf = String::with_capacity(16*4);
        let read = lob.read(0, 16, &mut buf)?;

        assert_eq!(16, read);
        assert_eq!(buf, "   Hello, World!");
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn read(&self, offset: usize, len: usize, buf: &mut String) -> Result<usize> {
        let form = self.charset_form()? as u8;
        let buf = unsafe { buf.as_mut_vec() };
        let mut num_read : usize = 0;
        self.read_piece(OCI_ONE_PIECE, len * 4, offset, &mut num_read, len, form, buf)?;
        Ok( num_read )
    }

    /**
        Starts piece-wise reading of the specified number of characters from this LOB into the provided buffer,
        returning a flag which indicates whether there are more pieces to read until the requested fragment is
        complete. Application should call `read_next` repeatedly until `read_next` returns `false`.

        ## Example
        ```
        use sibyl::{CLOB, Cache, CharSetForm};

        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let lob = CLOB::temp(&conn, CharSetForm::Implicit, Cache::No)?;
        let chunk_size = lob.chunk_size()?;
        lob.open()?;
        let fragment = vec![52u8;chunk_size];
        let fragment_as_text = unsafe { String::from_utf8_unchecked(fragment) };
        let mut written = lob.append_first(&fragment_as_text)?;
        for _i in 0..10 {
            written += lob.append_next(&fragment_as_text)?;
        }
        written += lob.append_last(&fragment_as_text)?;

        assert_eq!(chunk_size * 12, written);

        let mut text = String::new();
        let mut text_len : usize = 0;
        let mut read_len : usize = 0;
        let mut has_next = lob.read_first(chunk_size * 2, chunk_size * 5, &mut text, &mut text_len)?;
        while has_next {
            has_next = lob.read_next(&mut text, &mut read_len)?;
            text_len += read_len;
        }
        assert_eq!(text_len, chunk_size * 5);
        assert_eq!(text_len, text.len());
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn read_first(&self, offset: usize, len: usize, buf: &mut String, num_read: &mut usize) -> Result<bool> {
        let form = self.charset_form()? as u8;
        let buf = unsafe { buf.as_mut_vec() };
        let chunk_size = self.chunk_size()?;
        *num_read = 0;
        self.read_piece(OCI_FIRST_PIECE, chunk_size, offset, num_read, len, form, buf)
    }

    /**
        Continues piece-wise reading of the fragment started by `read_first`, returning a flag which indicates
        whether there are more pieces to read until the requested fragment is complete. Application should keep
        calling `read_next` until it returns `false`.
    */
    pub fn read_next(&self, buf: &mut String, num_read: &mut usize) -> Result<bool> {
        // let form = self.charset_form()? as u8;
        let buf = unsafe { buf.as_mut_vec() };
        let chunk_size = self.chunk_size()?;
        *num_read = 0;
        self.read_piece(OCI_NEXT_PIECE, chunk_size, 0, num_read, 0, 0, buf)
    }
}

impl<'a> LOB<'a,OCIBLobLocator> {
    /**
        Creates an empty temporary BLOB.

        The LOB is feed automatically either when a LOB goes out of scope or at the end of the session whichever comes first.
    */
    pub fn temp(conn: &'a dyn Conn, cache: Cache) -> Result<Self> {
        let locator = Descriptor::new(conn.env_ptr())?;
        catch!{conn.err_ptr() =>
            OCILobCreateTemporary(
                conn.svc_ptr(), conn.err_ptr(), locator.get(),
                OCI_DEFAULT as u16, 0u8, OCI_TEMP_BLOB, cache as u8, OCI_DURATION_SESSION
            )
        }
        Ok( Self { locator, conn, chunk_size: Cell::new(0) } )
    }

    /**
        Writes a buffer into a LOB.

        Returns the number of bytes written to the database.

        ## Example
        ```
        use sibyl::{BLOB, Cache};

        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let lob = BLOB::temp(&conn, Cache::No)?;
        let written = lob.write(3, "Hello, World!".as_bytes())?;
        assert_eq!(13, written);
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn write(&self, offset: usize, data: &[u8]) -> Result<usize> {
        self.write_piece(OCI_ONE_PIECE, offset, data)
    }

    /**
        Starts piece-wise writing into a LOB.

        The application must call `write_next` to write more pieces into the LOB.
        `write_last` terminates the piecewise write.

        Returns the number of bytes written to the database for the first piece.

        ## Example
        ```
        use sibyl::BLOB;

        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        # let stmt = conn.prepare("
        #     declare
        #         name_already_used exception; pragma exception_init(name_already_used, -955);
        #     begin
        #         execute immediate '
        #             create table test_lobs (
        #                 id       number generated always as identity,
        #                 text     clob,
        #                 data     blob,
        #                 ext_file bfile
        #             )
        #         ';
        #     exception
        #         when name_already_used then null;
        #     end;
        # ")?;
        # stmt.execute(&[])?;
        let stmt = conn.prepare("
            insert into test_lobs (data) values (empty_blob()) returning data into :data
        ")?;
        let mut lob = BLOB::new(&conn)?;
        stmt.execute_into(&[], &mut [ &mut lob ])?;
        lob.open()?;
        let chunk_size = lob.chunk_size()?;
        let data = vec![42u8;chunk_size];

        let written = lob.write_first(0, &data)?;
        assert_eq!(written, chunk_size);
        for i in 0..8 {
            let written = lob.write_next(&data)?;
            assert_eq!(written, chunk_size);
        }
        let written = lob.write_last(&data)?;
        assert_eq!(written, chunk_size);

        assert_eq!(lob.len()?, chunk_size * 10);
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn write_first(&self, offset: usize, data: &[u8]) -> Result<usize> {
        self.write_piece(OCI_FIRST_PIECE, offset, data)
    }

    /**
        Continues piece-wise writing into a LOB.

        The application must call `write_next` to write more pieces into the LOB.
        `write_last` terminates the piecewise write.

        Returns the number of bytes written to the database for this piece.
    */
    pub fn write_next(&self, data: &[u8]) -> Result<usize> {
        self.write_piece(OCI_NEXT_PIECE, 0, data)
    }

    /**
        Terminates piece-wise writing into a LOB.

        Returns the number of bytes written to the database for the last piece.
    */
    pub fn write_last(&self, data: &[u8]) -> Result<usize> {
        self.write_piece(OCI_LAST_PIECE, 0, data)
    }

    /**
        Writes data starting at the end of a LOB.

        Returns the number of bytes written to the database.

        ## Example
        ```
        use sibyl::{BLOB, Cache};

        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let lob = BLOB::temp(&conn, Cache::No)?;
        let written = lob.append("Hello, World!".as_bytes())?;
        assert_eq!(13, written);
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn append(&self, data: &[u8]) -> Result<usize> {
        self.append_piece(OCI_ONE_PIECE, data)
    }

    /**
        Starts piece-wise writing at the end of a LOB.

        The application must call `append_next` to write more pieces into the LOB.
        `append_last` terminates the piecewise write.

        Returns the number of bytes written to the database for the first piece.

        ## Example
        ```
        use sibyl::{BLOB, Cache};

        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let mut lob = BLOB::temp(&conn, Cache::No)?;
        let chunk_size = lob.chunk_size()?;
        let data = vec![165u8;chunk_size];

        let written = lob.append_first(&data)?;
        assert_eq!(written, chunk_size);
        for i in 0..8 {
            let written = lob.append_next(&data)?;
            assert_eq!(written, chunk_size);
        }
        let written = lob.append_last(&data)?;
        assert_eq!(written, chunk_size);

        assert_eq!(lob.len()?, chunk_size * 10);
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn append_first(&self, data: &[u8]) -> Result<usize> {
        self.append_piece(OCI_FIRST_PIECE, data)
    }

    /**
        Continues piece-wise writing at the end of a LOB.

        The application must call `append_next` to write more pieces into the LOB.
        `append_last` terminates the piecewise write.

        Returns the number of bytes written to the database for this piece.
    */
    pub fn append_next(&self, data: &[u8]) -> Result<usize> {
        self.append_piece(OCI_NEXT_PIECE, data)
    }

    /**
        Terminates piece-wise writing at the end of a LOB.

        Returns the number of bytes written to the database for the last piece.
    */
    pub fn append_last(&self, data: &[u8]) -> Result<usize> {
        self.append_piece(OCI_LAST_PIECE, data)
    }

    /**
        Reads specified number of bytes from this LOB, appending them to `buf`.
        If successful, this function returns the number of bytes that were read and appended to `buf`.

        ## Example
        ```
        use sibyl::{BLOB, Cache, BFile};

        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let file = BFile::new(&conn)?;
        file.set_file_name("MEDIA_DIR", "modem.jpg")?;
        assert!(file.file_exists()?);
        let file_len = file.len()?;
        file.open_readonly()?;
        let lob = BLOB::temp(&conn, Cache::No)?;
        lob.load_from_file(&file, 0, file_len, 0)?;
        assert_eq!(lob.len()?, file_len);

        let mut data = Vec::with_capacity(file_len);
        let num_read = lob.read(0, file_len, &mut data)?;

        assert_eq!(num_read, file_len);
        assert_eq!(data.len(), file_len);
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn read(&self, offset: usize, mut len: usize, buf: &mut Vec<u8>) -> Result<usize> {
        self.read_piece(OCI_ONE_PIECE, len, offset, &mut len, 0, 0, buf)?;
        Ok( len )
    }

    /**
        Starts piece-wise reading of the specified number of bytes from this LOB into the provided buffer, returning a tuple
        with 2 elements - the number of bytes read in the current piece and the flag which indicates whether there are more
        pieces to read until the requested fragment is complete. Application should call `read_next` repeatedly until "more
        data" flag becomes `false`.

        ## Example
        ```
        use sibyl::{BLOB, Cache, BFile};

        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let file = BFile::new(&conn)?;
        file.set_file_name("MEDIA_DIR", "monitor.jpg")?;
        assert!(file.file_exists()?);
        let file_len = file.len()?;
        file.open_readonly()?;
        let lob = BLOB::temp(&conn, Cache::No)?;
        lob.load_from_file(&file, 0, file_len, 0)?;
        assert_eq!(lob.len()?, file_len);

        let chunk_size = lob.chunk_size()?;
        let mut data = Vec::with_capacity(file_len);
        let mut data_len : usize = 0;
        let mut has_next = lob.read_first(0, file_len, &mut data, &mut data_len)?;
        assert_eq!(data_len, chunk_size);
        assert_eq!(data.len(), data_len);
        while has_next {
            let mut bytes_read : usize = 0;
            has_next = lob.read_next(&mut data, &mut bytes_read)?;
            data_len += bytes_read;
            assert_eq!(data_len, data.len());
        }

        assert_eq!(data_len, file_len);
        assert_eq!(data.len(), file_len);
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn read_first(&self, offset: usize, len: usize, buf: &mut Vec<u8>, num_read: &mut usize) -> Result<bool> {
        let chunk_size = self.chunk_size()?;
        *num_read = len;
        self.read_piece(OCI_FIRST_PIECE, chunk_size, offset, num_read, 0, 0, buf)
    }

    /**
        Continues piece-wise reading of the fragment started by `read_first`, returning a tuple with 2 elements - the number of
        bytes read in the current piece and the flag which indicates whether there are more pieces to read until the requested
        fragment is complete. Application should keep calling `read_next` until "more data" flag becomes `false`.
    */
    pub fn read_next(&self, buf: &mut Vec<u8>, num_read: &mut usize) -> Result<bool> {
        let chunk_size = self.chunk_size()?;
        *num_read = 0;
        self.read_piece(OCI_NEXT_PIECE, chunk_size, 0, num_read, 0, 0, buf)
    }
}

impl<'a> LOB<'a,OCIBFileLocator> {
    /**
        Closes a previously opened BFILE.

        No error is returned if the BFILE exists but is not opened.

        This function is only meaningful the first time it is called for a particular
        BFILE locator. Subsequent calls to this function using the same BFILE locator
        have no effect.
    */
    pub fn close_file(&self) -> Result<()> {
        catch!{self.conn.err_ptr() =>
            OCILobFileClose(
                self.conn.svc_ptr(), self.conn.err_ptr(), self.as_mut_ptr()
            )
        }
        Ok(())
    }

    /**
        Tests to see if the BFILE exists on the server's operating system.

        ## Example
        ```
        use sibyl::{BFile};

        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let file = BFile::new(&conn)?;
        file.set_file_name("MEDIA_DIR", "formatted_doc.txt")?;

        assert!(file.file_exists()?);
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn file_exists(&self) -> Result<bool> {
        let mut exists = mem::MaybeUninit::<u8>::uninit();
        catch!{self.conn.err_ptr() =>
            OCILobFileExists(
                self.conn.svc_ptr(), self.conn.err_ptr(), self.as_mut_ptr(),
                exists.as_mut_ptr()
            )
        }
        Ok( unsafe { exists.assume_init() } != 0 )
    }

    /**
        Returns the directory object and file name associated with this BFILE locator.

        ## Example
        ```
        use sibyl::{BFile};

        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let file = BFile::new(&conn)?;
        file.set_file_name("MEDIA_DIR", "hello_world.txt")?;
        let (dir_name, file_name) = file.file_name()?;

        assert_eq!(dir_name, "MEDIA_DIR");
        assert_eq!(file_name, "hello_world.txt");
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn file_name(&self) -> Result<(String,String)> {
        let mut dir  = String::with_capacity(30);
        let mut name = String::with_capacity(255);
        let mut dir_len  = dir.capacity() as u16;
        let mut name_len = name.capacity() as u16;
        catch!{self.conn.err_ptr() =>
            let dir  = dir.as_mut_vec();
            let name = name.as_mut_vec();
            OCILobFileGetName(
                self.conn.env_ptr(), self.conn.err_ptr(), self.as_mut_ptr(),
                dir.as_mut_ptr(),  &mut dir_len  as *mut u16,
                name.as_mut_ptr(), &mut name_len as *mut u16
            )
        }
        dir.set_len(dir_len as usize);
        name.set_len(name_len as usize);
        Ok( ( dir, name ) )
    }

    /**
        Returns `true` if the BFILE was opened using this particular locator.
        However, a different locator may have the file open. Openness is associated
        with a particular locator.

        ## Example
        ```
        use sibyl::{BFile};

        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let file = BFile::new(&conn)?;
        file.set_file_name("MEDIA_DIR", "hello_world.txt")?;
        assert!(!file.is_file_open()?);

        file.open_file()?;
        assert!(file.is_file_open()?);

        file.close_file()?;
        assert!(!file.is_file_open()?);
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn is_file_open(&self) -> Result<bool> {
        let mut is_open = mem::MaybeUninit::<u8>::uninit();
        catch!{self.conn.err_ptr() =>
            OCILobFileIsOpen(
                self.conn.svc_ptr(), self.conn.err_ptr(), self.as_mut_ptr(),
                is_open.as_mut_ptr()
            )
        }
        Ok( unsafe { is_open.assume_init() } != 0 )
    }

    /**
        Opens a BFILE on the file system of the server. The BFILE can only be opened
        for read-only access. BFILEs can not be written through Oracle Database.

        This function is only meaningful the first time it is called for a particular
        BFILE locator. Subsequent calls to this function using the same BFILE locator
        have no effect.

        ## Example
        ```
        use sibyl::{BFile};

        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let file = BFile::new(&conn)?;
        file.set_file_name("MEDIA_DIR", "hello_world.txt")?;

        file.open_file()?;

        assert!(file.is_file_open()?);
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn open_file(&self) -> Result<()> {
        catch!{self.conn.err_ptr() =>
            OCILobFileOpen(
                self.conn.svc_ptr(), self.conn.err_ptr(), self.as_mut_ptr(),
                OCI_FILE_READONLY
            )
        }
        Ok(())
    }

    /**
        Sets the directory object and file name in the BFILE locator.

        ## Example
        ```
        use sibyl::{BFile};

        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let file = BFile::new(&conn)?;
        file.set_file_name("MEDIA_DIR", "hello_world.txt")?;

        assert!(file.file_exists()?);
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn set_file_name(&self, dir: &str, name: &str) -> Result<()> {
        let mut filep = self.locator.get();
        catch!{self.conn.err_ptr() =>
            OCILobFileSetName(
                self.conn.env_ptr(), self.conn.err_ptr(),
                &mut filep as *mut *mut OCILobLocator,
                dir.as_ptr(),  dir.len() as u16,
                name.as_ptr(), name.len() as u16
            )
        }
        self.locator.replace(filep as *mut OCIBFileLocator);
        Ok(())
    }

    /**
        Reads specified number of bytes from this LOB, appending them to `buf`.
        If successful, this function returns the number of bytes that were read and appended to `buf`.

        ## Example
        ```
        use sibyl::{BFile};

        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let file = BFile::new(&conn)?;
        file.set_file_name("MEDIA_DIR", "hello_world.txt")?;
        let file_len = file.len()?;
        file.open_file()?;

        let mut data = Vec::with_capacity(file_len);
        let num_read = file.read(0, file_len, &mut data)?;

        assert_eq!(num_read, file_len);
        assert_eq!(data, [0xfeu8, 0xff, 0x00, 0x48, 0x00, 0x65, 0x00, 0x6c, 0x00, 0x6c, 0x00, 0x6f, 0x00, 0x2c, 0x00, 0x20, 0x00, 0x57, 0x00, 0x6f, 0x00, 0x72, 0x00, 0x6c, 0x00, 0x64, 0x00, 0x21]);
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn read(&self, offset: usize, mut len: usize, buf: &mut Vec<u8>) -> Result<usize> {
        self.read_piece(OCI_ONE_PIECE, len, offset, &mut len, 0, 0, buf)?;
        Ok( len )
    }

    /**
        Starts piece-wise reading of the specified number of bytes from this LOB into the provided buffer, returning a tuple
        with 2 elements - the number of bytes read in the current piece and the flag which indicates whether there are more
        pieces to read until the requested fragment is complete. Application should call `read_next` repeatedly until "more
        data" flag becomes `false`.

        ## Example
        ```
        use sibyl::{BFile};

        # let dbname = std::env::var("DBNAME")?;
        # let dbuser = std::env::var("DBUSER")?;
        # let dbpass = std::env::var("DBPASS")?;
        # let oracle = sibyl::env()?;
        # let conn = oracle.connect(&dbname, &dbuser, &dbpass)?;
        let file = BFile::new(&conn)?;
        file.set_file_name("MEDIA_DIR", "keyboard.jpg")?;
        assert!(file.file_exists()?);
        let file_len = file.len()?;
        file.open_readonly()?;

        let mut data = Vec::with_capacity(file_len);
        let mut data_len : usize = 0;
        let mut has_next = file.read_first(0, file_len, &mut data, &mut data_len)?;
        assert_eq!(data.len(), data_len);
        while has_next {
            let mut bytes_read : usize = 0;
            has_next = file.read_next(&mut data, &mut bytes_read)?;
            data_len += bytes_read;
            assert_eq!(data_len, data.len());
        }

        assert_eq!(data_len, file_len);
        assert_eq!(data.len(), file_len);
        # Ok::<(),Box<dyn std::error::Error>>(())
        ```
    */
    pub fn read_first(&self, offset: usize, len: usize, buf: &mut Vec<u8>, num_read: &mut usize) -> Result<bool> {
        *num_read = len;
        self.read_piece(OCI_FIRST_PIECE, cmp::min(len, 8192), offset, num_read, 0, 0, buf)
    }

    /**
        Continues piece-wise reading of the fragment started by `read_first`, returning a tuple with 2 elements - the number of
        bytes read in the current piece and the flag which indicates whether there are more pieces to read until the requested
        fragment is complete. Application should keep calling `read_next` until "more data" flag becomes `false`.
    */
    pub fn read_next(&self, buf: &mut Vec<u8>, num_read: &mut usize) -> Result<bool> {
        *num_read = 0;
        self.read_piece(OCI_NEXT_PIECE, 8192, 0, num_read, 0, 0, buf)
    }
}

macro_rules! impl_lob_to_sql {
    ($ts:ty => $sqlt:ident) => {
        impl ToSql for LOB<'_, $ts> {
            fn to_sql(&self) -> (u16, *const c_void, usize) {
                ( $sqlt, self.locator.as_ptr() as *const c_void, std::mem::size_of::<*mut OCILobLocator>() )
            }
        }
    };
}

impl_lob_to_sql!{ OCICLobLocator  => SQLT_CLOB  }
impl_lob_to_sql!{ OCIBLobLocator  => SQLT_BLOB  }
impl_lob_to_sql!{ OCIBFileLocator => SQLT_BFILE }

macro_rules! impl_lob_to_sql_output {
    ($ts:ty => $sqlt:ident) => {
        impl ToSqlOut for Descriptor<$ts> {
            fn to_sql_output(&mut self, _col_size: usize) -> (u16, *mut c_void, usize) {
                ($sqlt, self.as_ptr() as *mut c_void, std::mem::size_of::<*mut OCILobLocator>())
            }
        }
        impl ToSqlOut for LOB<'_, $ts> {
            fn to_sql_output(&mut self, col_size: usize) -> (u16, *mut c_void, usize) {
                self.locator.to_sql_output(col_size)
            }
        }
    };
}

impl_lob_to_sql_output!{ OCICLobLocator  => SQLT_CLOB  }
impl_lob_to_sql_output!{ OCIBLobLocator  => SQLT_BLOB  }
impl_lob_to_sql_output!{ OCIBFileLocator => SQLT_BFILE }
