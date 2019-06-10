create user sandbox identified by password;
grant connect, resource, unlimited tablespace, select_catalog_role to sandbox;

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
            execute immediate 'grant insert, select, update, delete on ' || r.owner || '.' || r.table_name || ' to sandbox';
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
            execute immediate 'grant select on ' || r.owner || '.' || r.view_name || ' to sandbox';
            if r.read_only = 'N' then
                execute immediate 'grant insert, update, delete on ' || r.owner || '.' || r.view_name || ' to sandbox';
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
                when 'SEQUENCE' then
                    execute immediate 'grant select  on ' || r.owner || '.' || r.object_name || ' to sandbox';
                when 'FUNCTION' then
                    execute immediate 'grant execute on ' || r.owner || '.' || r.object_name || ' to sandbox';
                when 'PROCEDURE' then
                    execute immediate 'grant execute on ' || r.owner || '.' || r.object_name || ' to sandbox';
                when 'PACKAGE' then
                    execute immediate 'grant execute on ' || r.owner || '.' || r.object_name || ' to sandbox';
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
        execute immediate 'GRANT read, write ON DIRECTORY '||r.directory_name||' TO sandbox';
    end loop;
end;
/
