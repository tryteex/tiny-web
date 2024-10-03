# tiny-web library
This asynchronous library is designed for creating the server-side (back-end) of a web application (site). Users get the minimum necessary functionality for quickly writing web controllers. The library should not be applied to handling user web requests directly; it is intended to be behind a reliable web server. Developers are advised to use it with the Nginx server through the FastCGI, SCGI, UWSGI (modifier1=0) and HTTP protocols.

To implement functionality using this library, mandatory interaction with the Postgresql database server is required. Support for other databases is unlikely. Developers must be able to write raw SQL queries for data manipulation.

This library is developed in parallel with an internet store (marketplace), and all changes and additions are primarily implemented for this application.

## Contents
* General concept [https://github.com/tryteex/tiny-web/blob/main/doc/General.md](https://github.com/tryteex/tiny-web/blob/main/doc/General.md)
* Basic functionality [https://github.com/tryteex/tiny-web/blob/main/doc/Basic.md](https://github.com/tryteex/tiny-web/blob/main/doc/Basic.md)
* File structure of the project [https://github.com/tryteex/tiny-web/blob/main/doc/Files.md](https://github.com/tryteex/tiny-web/blob/main/doc/Files.md)
* Config [https://github.com/tryteex/tiny-web/blob/main/doc/Config.md](https://github.com/tryteex/tiny-web/blob/main/doc/Config.md)
* Controller [https://github.com/tryteex/tiny-web/blob/main/doc/Controller.md](https://github.com/tryteex/tiny-web/blob/main/doc/Controller.md)
* __Action__ structure [https://github.com/tryteex/tiny-web/blob/main/doc/Action.md](https://github.com/tryteex/tiny-web/blob/main/doc/Action.md)
* __Answer__ structure [https://github.com/tryteex/tiny-web/blob/main/doc/Answer.md](https://github.com/tryteex/tiny-web/blob/main/doc/Answer.md)
* __Data__ structure [https://github.com/tryteex/tiny-web/blob/main/doc/Data.md](https://github.com/tryteex/tiny-web/blob/main/doc/Data.md)
* Access system [https://github.com/tryteex/tiny-web/blob/main/doc/Access.md](https://github.com/tryteex/tiny-web/blob/main/doc/Access.md)
* Template maker [https://github.com/tryteex/tiny-web/blob/main/doc/Template.md](https://github.com/tryteex/tiny-web/blob/main/doc/Template.md)
* I18N [https://github.com/tryteex/tiny-web/blob/main/doc/I18N.md](https://github.com/tryteex/tiny-web/blob/main/doc/I18N.md)
* Database [https://github.com/tryteex/tiny-web/blob/main/doc/Database.md](https://github.com/tryteex/tiny-web/blob/main/doc/Database.md)
* Sessions [https://github.com/tryteex/tiny-web/blob/main/doc/Sessions.md](https://github.com/tryteex/tiny-web/blob/main/doc/Sessions.md)
* Caching [https://github.com/tryteex/tiny-web/blob/main/doc/Caching.md](https://github.com/tryteex/tiny-web/blob/main/doc/Caching.md)
* Request [https://github.com/tryteex/tiny-web/blob/main/doc/Request.md](https://github.com/tryteex/tiny-web/blob/main/doc/Request.md)
* Response [https://github.com/tryteex/tiny-web/blob/main/doc/Response.md](https://github.com/tryteex/tiny-web/blob/main/doc/Response.md)
* Email system [https://github.com/tryteex/tiny-web/blob/main/doc/Email.md](https://github.com/tryteex/tiny-web/blob/main/doc/Email.md)
* First-Time start [https://github.com/tryteex/tiny-web/blob/main/doc/First.md](https://github.com/tryteex/tiny-web/blob/main/doc/First.md)
* Network protocols [https://github.com/tryteex/tiny-web/blob/main/doc/Net.md](https://github.com/tryteex/tiny-web/blob/main/doc/Net.md)
* Configuring nginx [https://github.com/tryteex/tiny-web/blob/main/doc/Nginx.md](https://github.com/tryteex/tiny-web/blob/main/doc/Nginx.md)
* Example [https://github.com/tryteex/tiny-web/blob/main/doc/Example.md](https://github.com/tryteex/tiny-web/blob/main/doc/Example.md)
* Todo [Thttps://github.com/tryteex/tiny-web/blob/main/doc/odo.md](https://github.com/tryteex/tiny-web/blob/main/doc/Todo.md)

> **Note**  
> If you have any questions ask us on [discord channel](https://discord.com/channels/1116858532491448332/1116858533061869742).