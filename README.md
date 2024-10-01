# tiny-web

`tiny-web` is a tiny async library (backend web server) that allows you to write a Laravel-style or Django-style backend in Rust language.

Short documentation for the library is [https://github.com/tryteex/tiny-web/blob/main/doc/Index.md](https://github.com/tryteex/tiny-web/blob/main/doc/Index.md).

> **Note**  
> For security reasons, this library must be located behind the Primary web server, for example, under Nginx.

This library works only with FastCGI, SCGI, UWSGI (modifier1=0) and HTTP protocols.

This library works with Postgresql or MS Sql Server databases.

## Stability Notice

Please be aware that this library is not yet stable. Some components may change, however, the main interface for interacting with the application will remain finalized. The functionality will only be expanded. We are planning to create a stable version with the release of version 0.6.

## Contributing

If you'd like to contribute to tiny-web, check out our repository [https://github.com/tryteex/tiny-web](https://github.com/tryteex/tiny-web).

## License

This project is licensed under the MIT License - see the LICENSE file for details.