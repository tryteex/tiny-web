# tiny-web

`tiny-web` is a tiny async library (backend web server) that allows you to write a Laravel-style or Django-style backend in Rust language.

Short documentation for the library is [here](https://github.com/tryteex/tiny-web/blob/main/doc/Index.md).

> **Note**  
> For security reasons, this library must be located behind the Primary web server, for example, under Nginx.

This library works only with FastCGI, SCGI, and UWSGI (modifier1=0) protocols.
> **Note**  
> Support for GRPC, HTTP (hiding behind a Reverse Proxy), and WebSocket is also under development. Check out our repository on GitHub.
>
> In addition, testing is performed exclusively under Nginx.

This library works with Postgresql, MS Sql Server and MySql databases.

The `lib-install-***.sql` file is in the `sql` directory of the project as a simple solution so that the server can start. Or start project without `config` file as [First-Time start.md](https://github.com/tryteex/tiny-web/blob/main/doc/First.md).

## Contributing

If you'd like to contribute to tiny-web, check out our repository on GitHub.

## License

This project is licensed under the MIT License - see the LICENSE file for details.