/// OCI pub(crate) constants used by `Statement`

// Parsing Syntax Types
pub(crate) const OCI_NTV_SYNTAX   : u32 = 1;

// Statement Types
// pub(crate) const OCI_STMT_UNKNOWN : u16 = 0;
pub(crate) const OCI_STMT_SELECT  : u16 = 1;
// pub(crate) const OCI_STMT_UPDATE  : u16 = 2;
// pub(crate) const OCI_STMT_DELETE  : u16 = 3;
// pub(crate) const OCI_STMT_INSERT  : u16 = 4;
// pub(crate) const OCI_STMT_CREATE  : u16 = 5;
// pub(crate) const OCI_STMT_DROP    : u16 = 6;
// pub(crate) const OCI_STMT_ALTER   : u16 = 7;
// pub(crate) const OCI_STMT_BEGIN   : u16 = 8;
// pub(crate) const OCI_STMT_DECLARE : u16 = 9;
// pub(crate) const OCI_STMT_CALL    : u16 = 10;
// pub(crate) const OCI_STMT_MERGE   : u16 = 16;

// Attributes common to Columns and Stored Procs
pub(crate) const OCI_ATTR_DATA_SIZE         : u32 =  1; // maximum size of the data
pub(crate) const OCI_ATTR_DATA_TYPE         : u32 =  2; // the SQL type of the column/argument
// pub(crate) const OCI_ATTR_DISP_SIZE         : u32 =  3; // the display size
pub(crate) const OCI_ATTR_NAME              : u32 =  4; // the name of the column/argument
pub(crate) const OCI_ATTR_PRECISION         : u32 =  5; // precision if number type
pub(crate) const OCI_ATTR_SCALE             : u32 =  6; // scale if number type
pub(crate) const OCI_ATTR_IS_NULL           : u32 =  7; // is it null ?
pub(crate) const OCI_ATTR_TYPE_NAME         : u32 =  8; // name of the named data type or a package name for package private types
pub(crate) const OCI_ATTR_SCHEMA_NAME       : u32 =  9; // the schema name
// pub(crate) const OCI_ATTR_SUB_NAME          : u32 = 10; // type name if package private type
// pub(crate) const OCI_ATTR_POSITION          : u32 = 11; // relative position of col/arg in the list of cols/args
// pub(crate) const OCI_ATTR_PACKAGE_NAME      : u32 = 12; // package name of package type
pub(crate) const OCI_ATTR_CHARSET_FORM      : u32 = 32;
pub(crate) const OCI_ATTR_COL_PROPERTIES    : u32 = 104;
pub(crate) const OCI_ATTR_CHAR_SIZE         : u32 = 286;

// Flags coresponding to the column properties
pub(crate) const OCI_ATTR_COL_PROPERTY_IS_IDENTITY             : u8 = 0x01;
pub(crate) const OCI_ATTR_COL_PROPERTY_IS_GEN_ALWAYS           : u8 = 0x02;
pub(crate) const OCI_ATTR_COL_PROPERTY_IS_GEN_BY_DEF_ON_NULL   : u8 = 0x04;
