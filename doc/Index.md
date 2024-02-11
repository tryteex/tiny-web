# tiny-web library
This asynchronous library is designed for creating the server-side (back-end) of a web application (site). Users get the minimum necessary functionality for quickly writing web controllers. The library should not be applied to handling user web requests directly; it is intended to be behind a reliable web server. Developers are advised to use it with the Nginx server through the Fast CGI protocol. In the future, support for other network protocols is planned.

To implement functionality using this library, mandatory interaction with the Postgresql database server is required. Support for other databases is unlikely. Developers must be able to write raw SQL queries for data manipulation.

This library is developed in parallel with an internet store (marketplace), and all changes and additions are primarily implemented for this application.

## Contents
* General concept [General.md](https://github.com/tryteex/tiny-web/blob/main/doc/General.md)
* Basic functionality [Basic.md](https://github.com/tryteex/tiny-web/blob/main/doc/Basic.md)
* File structure of the project [Files.md](https://github.com/tryteex/tiny-web/blob/main/doc/Files.md)
* Config [Config.md](https://github.com/tryteex/tiny-web/blob/main/doc/Config.md)
* Controller [Controller.md](https://github.com/tryteex/tiny-web/blob/main/doc/Controller.md)
* __Action__ structure [Action.md](https://github.com/tryteex/tiny-web/blob/main/doc/Action.md)
* __Answer__ structure [Answer.md](https://github.com/tryteex/tiny-web/blob/main/doc/Answer.md)
* __Data__ structure [Data.md](https://github.com/tryteex/tiny-web/blob/main/doc/Data.md)
* Access system [Access.md](https://github.com/tryteex/tiny-web/blob/main/doc/Access.md)
* Template maker [Template.md](https://github.com/tryteex/tiny-web/blob/main/doc/Template.md)
* I18N [I18N.md](https://github.com/tryteex/tiny-web/blob/main/doc/I18N.md)
* Database [Database.md](https://github.com/tryteex/tiny-web/blob/main/doc/Database.md)
* Router [Router.md](https://github.com/tryteex/tiny-web/blob/main/doc/Router.md)
* Sessions [Sessions.md](https://github.com/tryteex/tiny-web/blob/main/doc/Sessions.md)
* Caching [Caching.md](https://github.com/tryteex/tiny-web/blob/main/doc/Caching.md)
* Email system [Email.md](https://github.com/tryteex/tiny-web/blob/main/doc/Email.md)
* Call another controller [Call.md](https://github.com/tryteex/tiny-web/blob/main/doc/Call.md)
* Useful functions / macros [Functions.md](https://github.com/tryteex/tiny-web/blob/main/doc/Functions.md)
* Request [Request.md](https://github.com/tryteex/tiny-web/blob/main/doc/Request.md)
* Response [Response.md](https://github.com/tryteex/tiny-web/blob/main/doc/Response.md)
* Configuring nginx [Nginx.md](https://github.com/tryteex/tiny-web/blob/main/doc/Nginx.md)
* Performance [Performance.md](https://github.com/tryteex/tiny-web/blob/main/doc/Controller.md)
* Simple example [Example.md](https://github.com/tryteex/tiny-web/blob/main/doc/Example.md)

> **Note**  
> Some things might be written unclearly or with incorrect meanings, so I apologize in advance because my native language is Ukrainian.