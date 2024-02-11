## Basic functionality
This library automatically accepts connections from clients, forms a pool of servers to the database, analyzes the incoming URL, and redirects the request to the appropriate controller. Consequently, the primary task for developers is to write the controller's functionality.

The basic functionality of the library includes the following modules:

* __Route__

  Automatically transforms the URL request into a pre-defined controller. Additionally, provides the controller with the ability to retrieve the URL.

* __Database__

  Represents a connection pool to the database. Each database query is immediately sent for execution, but if there are no available connections, it goes into a queue. Only raw SQL queries are supported.

* __Session__

  Automatically identifies the user using cookies, stores personalized user data in the database. Supports simultaneous requests from a single user without blocking cookies.

* __Access__

  Ensures hierarchical (__Module/Class/Action__) access to any controller. Access is absent by default. Additionally, provides the ability to check access to any controller.

* __Template__

  HTML templating engine that generates HTML pages based on cached templates with minimal support for loops, conditional branches, etc.

* __I18n__

  Automatically, depending on the user's set language, allows obtaining translations for simple texts.

* __Cache__

  Provides asynchronous access to shared data. Supports a hierarchical structure.

* __Mail__

  Sends email messages. It is not an independent SMTP server but allows connecting to an existing server, supporting the latest interaction protocols.

The full list of available functions in the controller is in the section Useful functions / macros [Functions.md](https://github.com/tryteex/tiny-web/blob/main/doc/Functions.md).
___
Next => File structure of the project [Files.md](https://github.com/tryteex/tiny-web/blob/main/doc/Files.md)  
Index => Contents [Index.md](https://github.com/tryteex/tiny-web/blob/main/doc/Index.md)  
