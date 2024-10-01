## Configuration File
The configuration file __tiny.toml__ contains essential settings for running the web application. The simplest way to set it up is by renaming the ___tiny.sample.toml___ file to ___tiny.toml___. It is recommended to place the file either in the root of your project or next to the executable file. This file must be in [TOML](https://toml.io/) format.

Here is a list of all options:

### lang
Default language of the server, set according to ISO 639-1. For example, for Ukrainian, it would be 'uk', and for English, it would be 'en'.

Example:
* `lang = "uk"`

### log
Path for the log file. If not set, the file will be created automatically. All errors and warnings, especially those related to the templating engine, are recorded in this file.

Example:
* `log = "/var/log/tiny.log"`
* `log = "/home/web/log/tiny.log"`

### max
Maximum number of threads for asynchronous operations. Typically set slightly higher than the number of CPUs. If set to "auto," the number of threads will be automatically determined by the [tokio](https://tokio.rs/) library.

Example:
* `max = "auto"`
* `max = 12`

### bind_from
IP address from which network connections are expected. If set to "any" connections from any IP address are anticipated. If left empty, connections will be made via Unix domain sockets.

Example:
* `bind_from = "127.0.0.1"`
* `bind_from = "any"`
* `bind_from = ""`

### bind
IP address and port used for expecting connections from the web server. For Linux, Unix domain sockets can be used by starting with "/". 

Example:
* `bind = "127.0.0.1:12500"`
* `bind = "/home/user/bin/tiny.socket"`

### rpc_from
IP address used by this library to expect network connections for managing this server. If set to "any," connections from any IP address are expected. If left empty, connections will be made via Unix domain sockets.

Example:
* `rpc_from = "127.0.0.1"`
* `rpc_from = "any"`
* `rpc_from = ""`

### rpc
IP address and port to manage this server. On Unix systems, a "rpc" starting with a "/" is interpreted as a path to a directory containing Unix domain sockets.

Example:
* `rpc = "127.0.0.1:12501"`
* `rpc = "/home/user/bin/tiny.rpc.socket"`

### salt
"Salt" for a crypto functions

Example:
* `salt = "SameSaltWords12345"`
* `salt = "dhgHKghf^*7fjkdbv6%24%d"`

### session
"session" is name of cookie for session

Example:
* `salt = "tinysession"`
* `salt = "tiny`

### db_host
Postgresql database host. On Unix systems, a "db_host" starting with a "/" is interpreted as a path to a directory containing Unix domain sockets.

Example:
* `db_host = "remove.host.com"`
* `db_host = "127.0.0.1"`
* `db_host = "/var/run/postgresql/db_main.socket"`

### db_port
Postgresql database port. Can be empty.

Example:
* `db_port = 5432`
* `db_port = ""`

### db_name
Postgresql database name.

Example:
* `db_name = "name"`

### db_user
Postgresql database username. Can be empty.

Example:
* `db_user = "user"`
* `db_user = ""`

### db_pwd
Postgresql database password. Can be empty.

Example:
* `db_pwd = "pwd"`
* `db_pwd = ""`

### sslmode
Postgresql database sslmode mode: require or no (default)

Example:
* `sslmode = "require"`
* `sslmode = "no"`
* `sslmode = ""`

### db_max
Number of connections to the database for all work threads in async. Usually set from 2 to 4 on one work thread. Set "auto" to detect automatically.

Example:
* `db_max = "auto"`
* `db_max = 2`
* `db_max = 24`

### protocol
Used net protocol. Maybe: FastCGI, SCGI, uWSGI (modifier1=0), or HTTP.

Example:
* `protocol = "FastCGI"`
* `protocol = "SCGI"`
* `protocol = "uWSGI"`
* `protocol = "HTTP"`

### action_index
Default controller for request "/" or default class or default action.

Example:
* `action_index = "/index/index/index"`
* `action_index = "/main/main/main"`

### action_not_found
Default controller for 404 Not Found.

Example:
* `action_not_found = "/index/index/not_found"`
* `action_not_found = "/main/main/not_found"`

### action_error_route
Default controller for error_route.

Example:
* `action_error_route = "/index/index/err"`
* `action_error_route = "/main/route/err"`

___
Next => Controller [https://github.com/tryteex/tiny-web/blob/main/doc/Controller.md](https://github.com/tryteex/tiny-web/blob/main/doc/Controller.md)  
Index => Contents [https://github.com/tryteex/tiny-web/blob/main/doc/Index.md](https://github.com/tryteex/tiny-web/blob/main/doc/Index.md)  
