## Database
The library uses PostgreSQL DBMS version 15 and above via adapter. However, you can also try lower versions.
or
The library uses MsSql Server DBMS version 16 and above via adapter. However, you can also try lower versions.


Access to the database is mandatory when starting the server. In case of connection loss during operation, the library will attempt to restore it with each request. The corresponding event will be logged.

___
### Installation
At the root of the project lies the file [lib-install-pgsql.sql](https://raw.githubusercontent.com/tryteex/tiny-web/main/lib-install-pgsql.sql) for Postgres or [lib-install-mssql.sql](https://raw.githubusercontent.com/tryteex/tiny-web/main/lib-install-mssql.sql) for MsSql which needs to be executed before the first run in the DB.  
In the future, the installation and update process will be automated. 

Or start project without `config` file as [First-Time start.md](https://github.com/tryteex/tiny-web/blob/main/doc/First.md).
___
### Access and pool connections
In the configuration file, connection parameters to the database are specified.
* `db_host` - Postgresql database host.  
On Unix systems, a "db_host" starting with a "/" is interpreted as a path to a directory containing Unix domain sockets.
* `db_port` - Postgresql database port.  
Can be empty.
* `db_name` - Postgresql database name.
* `db_user` - Postgresql database username.  
Can be empty.
* `db_pwd` - Postgresql database password.  
Can be empty.
* `sslmode` - Postgresql database sslmode mode.  
`true` is require. 
* `db_max` - Number of connections to the database for all work threads in async.  
Usually set from 2 to 4 on one work thread.  
Set "auto" to detect automatically.
* `[prepare]` - Prepare sql queries.  
For Postgresql types can be `BOOL`, `INT8`, `INT2`, `INT4`, `TEXT`, `VARCHAR`, `FLOAT4`, `FLOAT8`, `JSON`, `TIMESTAMPTZ`, `UUID`, `BYTEA`.
For MsSql types can be `BIT`, `BIGINT`, `INT`, `SMALLINT`, `TINYINT`, `NVARCHAR(MAX)`, `NVARCHAR(N_int <= 4000)`, `VARCHAR(MAX)`, `VARCHAR(N_int <= 4000)`, `FLOAT`, `REAL`, `DATETIMEOFFSET`, `UNIQUEIDENTIFIER`, `VARBINARY(MAX)`, `VARBINARY(N_int <= 8000)`.
```toml
[prepare]
key_name1.query = "SELECT name FROM user WHERE id=$1"
key_name1.types = ["INT4"]
key_name2.query = "INSERT INTO user(name) VALUES ($1)"
key_name2.types = ["TEXT"]
```
During the library startup, a connection pool is created with the database. The number of connections can be specified by the parameter `db_max`. When executing a query, the library automatically selects an available connection.
> **Note**  
> Do not use session temporary tables for database queries.
___
### Request from controller
To execute a request from the controller, you need to use the functions this.`db.query` or `this.db.query`. 

#### Example 
```rust
pub async fn index(this: &mut Action) -> Answer {
    // Execute prepare query from config file
    // 
    // In config file `tiny.toml`:
    //[prepare]
    //prepare_key.query = "SELECT name FROM users"
    //#prepare_key.types = []
    let prepare = this.db.query(tiny_web_macro::fnv1a_64!("prepare_key"), &[], false).await.unwrap();
    this.set("value", Data::Vec(prepare));

    // Execute simple query from config file
    let users = this.db.query("SELECT name FROM users", &[], false).await.unwrap();
    this.set("users", Data::Vec(users));
    
    this.render("index")
}
```
___
For executing queries to the database, you can use the following functions:
* `execute` - Execute a regular query without results.
* `query` - Execute a regular query with results.
* `query_group` - Execute a query returning hierarchical results.

### The `execute` or `query` functions
Execute query to database asynchronously.

Parmeters:
* `query: &str` - SQL query;
* `query: i64` - Key of Statement;
* `params: &[&dyn ToSql]` - Array of params.
* `assoc: bool` - Return columns as associate array if True or Vecor id False.

Return:
* `Option::None` - When error query or diconnected;
* `Option::Some(Vec<Data::Map>)` - Results, if assoc = true.
* `Option::Some(Vec<Data::Vec>)` - Results, if assoc = false.
```rust
async fn query(query: impl KeyOrQuery, params: &[&(dyn ToSql + Sync)], assoc: bool) -> Option<Vec<Data>> { ... }
```
### The `query_group` function
Execute query to database and return a result, and grouping tabular data according to specified conditions.

Parmeters:
* `query: &str` - SQL query;
* `query: i64` - Key of Statement;
* `params: &[&(dyn ToSql + Sync)]` - Array of params.
* `assoc: bool` - Return columns as associate array if True or Vecor id False.
* `conds: Vec<Vec<&str>>` - Grouping condition.  

Grouping condition:
* The number of elements in the first-level array corresponds to the hierarchy levels in the group.
* The number of elements in the second-level array corresponds to the number of items in one hierarchy. The first element of the group (index=0) is considered unique.
* &str - field names for `Data::Vec<Data::Map<...>>`.

The first value in the second-level array must be of type `Data::I64`.

For each group, a new field with the name `sub` (encoded using `fnv1a_64`) will be created, where child groups will be located.

If the data does not match the format `Data::Vec<Data::Map<...>>`, grouping will not occur, `Option::None` will be returned.  
If the data does not match the tabular format, grouping will not occur, `Option::None` will be returned.

Fields that are not included in the group will be excluded.

Return:
* Option::None - If the fields failed to group.  

if `assoc` = `true` 
* `Some(Data::Map<cond[0][0], Data::Map<...>>)` in hierarchical structure.  
```struct
 value=Data::Map
 ├── [value1 from column_name=cond[0][0]] => [value=Data::Map]  : The unique value of the grouping field
 │   ├── [key=cond[0][0]] => [value1 from column_name=cond[0][0]] : The unique value of the grouping field
 │   ├── [key=cond[0][1]] => [value from column_name=cond[0][1]]
 │   │   ...  
 │   ├── [key=cond[0][last]] => [value from column_name=cond[0][last]]
 │   └── [key="sub"] => [value=Data::Map] : (encoded using fnv1a_64)
 │       ├── [value1 from column_name=cond[1][0]] => [value=Data::Map]  : The unique value of the grouping field
 │       │   ├── [cond[1][0]] => [value1 from column_name=cond[1][0]] : The unique value of the grouping field
 │       │   ├── [cond[1][1]] => [value from column_name=cond[1][1]]  
 │       │   │   ...
 │       │   ├── [cond[0][last]] => [value from column_name=cond[1][last]]  
 │       │   └── [key="sub"] => [value Data::Map] : (encoded using fnv1a_64)
 │       └── [value2 from column_name=cond[1][0]] => [value=Data::Map]  : The unique value of the grouping field
 │           │    ...
 ├── [value2 from column_name=cond[0][0]] => [value=Data::Map]  : The unique value of the grouping field
 │   ├── [key=cond[0][0]] => [value2 from column_name=cond[0][0]] : The unique value of the grouping field
 │   ├── [key=cond[0][1]] => [value from column_name=cond[0][1]]
 │   │   ...  
 │   ├── [key=cond[0][last]] => [value from column_name=cond[0][last]]
 │   ├── [key="sub"] => [value Data::Map] : (encoded using fnv1a_64)
 ...
 ```
if `assoc` = `false` 
* `Some(Data::Map<cond[0][0], Data::Map<...>>)` in hierarchical structure.  
```struct
 value=Data::Map
 ├── [value1 from column_name=cond[0][0]] => [value=Data::Vec]  : The unique value of the grouping field
 │   ├── [0] => [value1 from column_name=cond[0][0]] : The unique value of the grouping field
 │   ├── [1] => [value from column_name=cond[0][1]]
 │   │   ...  
 │   ├── [last] => [value from column_name=cond[0][last]]
 │   └── [last + 1] => [value=Data::Map] : (encoded using fnv1a_64)
 │       ├── [value1 from column_name=cond[1][0]] => [value=Data::Vec]  : The unique value of the grouping field
 │       │   ├── [0] => [value1 from column_name=cond[1][0]] : The unique value of the grouping field
 │       │   ├── [1] => [value from column_name=cond[1][1]]  
 │       │   │   ...
 │       │   ├── [last] => [value from column_name=cond[1][last]]  
 │       │   └── [last+1] => [value Data::Map] : (encoded using fnv1a_64)
 │       └── [value2 from column_name=cond[1][0]] => [value=Data::Vec]  : The unique value of the grouping field
 │           │    ...
 ├── [value2 from column_name=cond[0][0]] => [value=Data::Vec]  : The unique value of the grouping field
 │   ├── [0] => [value2 from column_name=cond[0][0]] : The unique value of the grouping field
 │   ├── [1] => [value from column_name=cond[0][1]]
 │   │   ...  
 │   ├── [last] => [value from column_name=cond[0][last]]
 │   ├── [last + 1] => [value Data::Map] : (encoded using fnv1a_64)
 ...
```
```rust
async fn query_group(query: impl KeyOrQuery, params: &[&(dyn ToSql + Sync)], assoc: bool, conds: &[&[impl StrOrI64OrUSize]] ) -> Option<Data> { ... }
```
### The `query_raw` function
Execute query to database and return a raw result synchronously.

Parmeters:
* `query: &str` - SQL query;
* `query: i64` - Key of Statement;
* `params: &[&(dyn ToSql + Sync)]` - Array of params.

Return:
* `Option::None` - When error query or diconnected;
* `Option::Some(Vec<Row>)` - Results.
```rust
async fn query_raw(query: impl KeyOrQuery, params: &[&(dyn ToSql + Sync)]) -> Option<Vec<Row>> { ... }
```
___
### Description of tables
| Table | Description |
|-|-|
|access| Provides access to controllers based on user role. |
|controller| Hierarchically describes controllers that need to be interacted with. Not all controllers may be described. |
|lang| List of languages that can be supported at the database level. Starts from 0. |
|mail| Registry of all mail messages. |
|provider| Provider that facilitates user login. |
|redirect| Automatic redirection of URL requests. |
|role| User roles. |
|route| URL request routing. |
|session| User session data. |
|setting| System settings. |
|user| Users. User with user_id=0 is a guest. |
|user_provider| Selection of login provider for the user. |
___
### ER diagram
Diagram_1.jpg
![Diagram of database](https://raw.githubusercontent.com/tryteex/tiny-web/main/doc/img/Diagram_1.jpg)
___
Next => Sessions [Sessions.md](https://github.com/tryteex/tiny-web/blob/main/doc/Sessions.md)  
Index => Contents [Index.md](https://github.com/tryteex/tiny-web/blob/main/doc/Index.md)  