create user sibyl identified by Or4cl3;
grant connect, resource, unlimited tablespace, select_catalog_role to sibyl;

begin
    for r in (
        select owner, table_name
          from all_tables
         where owner in ('HR', 'OE', 'PM', 'IX', 'SH', 'BI')
           and nested = 'NO'
           and external = 'NO'
           and nvl(iot_type,'_') != 'IOT_OVERFLOW')
    loop
        begin
            execute immediate 'grant insert, select, update, delete on ' || r.owner || '.' || r.table_name || ' to sibyl';
        exception
            when others then
                dbms_output.put_line('ERROR: cannot grant access to table ' || r.owner || '.' || r.table_name || ' -- ' || substr(sqlerrm,1,200));
        end;
    end loop;

    for r in (
        select owner, view_name, read_only
          from all_views
         where owner in ('HR', 'OE', 'PM', 'IX', 'SH', 'BI'))
    loop
        begin
            execute immediate 'grant select on ' || r.owner || '.' || r.view_name || ' to sibyl';
            if r.read_only = 'N' then
                execute immediate 'grant insert, update, delete on ' || r.owner || '.' || r.view_name || ' to sibyl';
            end if;
        exception
            when others then
                dbms_output.put_line('ERROR: cannot grant access to view ' || r.owner || '.' || r.view_name || ' -- ' || substr(sqlerrm,1,200));
        end;
    end loop;

    for r in (
        select owner, object_name, object_type
          from all_objects
         where owner in ('HR', 'OE', 'PM', 'IX', 'SH', 'BI')
           and object_type in ('SEQUENCE', 'FUNCTION', 'PROCEDURE', 'PACKAGE')
           and object_name not like 'BIN$%')
    loop
        begin
            case r.object_type
            when 'SEQUENCE'  then execute immediate 'grant select  on ' || r.owner || '.' || r.object_name || ' to sibyl';
            when 'FUNCTION'  then execute immediate 'grant execute on ' || r.owner || '.' || r.object_name || ' to sibyl';
            when 'PROCEDURE' then execute immediate 'grant execute on ' || r.owner || '.' || r.object_name || ' to sibyl';
            when 'PACKAGE'   then execute immediate 'grant execute on ' || r.owner || '.' || r.object_name || ' to sibyl';
            end case;
        exception
            when others then
                dbms_output.put_line('ERROR: cannot grant access to ' || r.object_type || ' ' || r.owner || '.' || r.object_name || ' -- ' || substr(sqlerrm,1,200));
        end;
    end loop;

    for r in (
        select directory_name 
          from all_directories
         where directory_path like '%/demo/schema/%')
    loop
        execute immediate 'GRANT read, write ON DIRECTORY '||r.directory_name||' TO sibyl';
    end loop;
end;
/

DECLARE
    name_already_used EXCEPTION; PRAGMA EXCEPTION_INIT(name_already_used, -955);
BEGIN
    EXECUTE IMMEDIATE '
        CREATE TABLE test_lobs (
            id       INTEGER GENERATED ALWAYS AS IDENTITY,
            text     CLOB,
            data     BLOB,
            ext_file BFILE
        )
    ';
EXCEPTION
    WHEN name_already_used THEN NULL;
END;
/

DECLARE
    name_already_used EXCEPTION; PRAGMA EXCEPTION_INIT(name_already_used, -955);
BEGIN
    EXECUTE IMMEDIATE '
        CREATE TABLE long_and_raw_test_data (
            id      INTEGER GENERATED ALWAYS AS IDENTITY,
            bin     RAW(100),
            text    LONG
        )
    ';
EXCEPTION
  WHEN name_already_used THEN NULL;
END;
/

DECLARE
    name_already_used EXCEPTION; PRAGMA EXCEPTION_INIT(name_already_used, -955);
BEGIN
    EXECUTE IMMEDIATE '
        CREATE TABLE test_character_data (
            id      NUMBER GENERATED ALWAYS AS IDENTITY,
            text    VARCHAR2(97),
            ntext   NVARCHAR2(99)
        )
    ';
EXCEPTION
    WHEN name_already_used THEN NULL;
END;
/

DECLARE
    name_already_used EXCEPTION; PRAGMA EXCEPTION_INIT(name_already_used, -955);
BEGIN
    EXECUTE IMMEDIATE '
        CREATE TABLE test_datetime_data (
            id      NUMBER GENERATED ALWAYS AS IDENTITY,
            dt      DATE,
            ts      TIMESTAMP(9),
            tsz     TIMESTAMP(9) WITH TIME ZONE,
            tsl     TIMESTAMP(9) WITH LOCAL TIME ZONE,
            iym     INTERVAL YEAR(9) TO MONTH,
            ids     INTERVAL DAY(8) TO SECOND(9)
        )
    ';
EXCEPTION
    WHEN name_already_used THEN NULL;
END;
/

DECLARE
    name_already_used EXCEPTION; PRAGMA EXCEPTION_INIT(name_already_used, -955);
BEGIN
    EXECUTE IMMEDIATE '
        CREATE TABLE test_large_object_data (
            id      NUMBER GENERATED ALWAYS AS IDENTITY,
            bin     BLOB,
            text    CLOB,
            ntxt    NCLOB,
            fbin    BFILE
        )
    ';
EXCEPTION
    WHEN name_already_used THEN NULL;
END;
/

DECLARE
    name_already_used EXCEPTION; PRAGMA EXCEPTION_INIT(name_already_used, -955);
BEGIN
    EXECUTE IMMEDIATE '
        CREATE TABLE test_long_raw_data (
            id      NUMBER GENERATED ALWAYS AS IDENTITY,
            bin     LONG RAW
        )
    ';
EXCEPTION
    WHEN name_already_used THEN NULL;
END;
/

DECLARE
    name_already_used EXCEPTION; PRAGMA EXCEPTION_INIT(name_already_used, -955);
BEGIN
    EXECUTE IMMEDIATE '
        CREATE TABLE test_numeric_data (
            id      NUMBER GENERATED ALWAYS AS IDENTITY,
            num     NUMBER,
            flt     BINARY_FLOAT,
            dbl     BINARY_DOUBLE
        )
    ';
EXCEPTION
    WHEN name_already_used THEN NULL;
END;
/
