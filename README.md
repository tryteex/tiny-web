# tiny-web

> **Note**  
> Preliminary development only! This library is under development..

`tiny-web` is a tiny async library (backend web server) that allows you to write a Laravel-style or Django-style backend in Rust language.

> **Note**  
> For security reasons, this library must be located behind the Primary web server, for example, under Nginx.

This library works only with FastCGI, SCGI, and UWSGI (modifier1=0) protocols.
> **Note**  
> Support for GRPC, HTTP (hiding behind a Reverse Proxy), and WebSocket is also under development. Check out our repository on GitHub.
>
> In addition, testing is performed exclusively under Nginx.

This library works with Postgresql 15+ database. But you can try the lower version.

The `install.sql` file is in the root of the project as a temporary solution so that the server can start. In the future, the `install` and `update` command will be added to the library to install and update the database.

## Documentation and examples

* `tiny-web` library and documentation https://rust.tiny.com.ua/ .

> **Note**  
> The sites are under construction, follow the projects, and check out our repository on GitHub.

## Installation

Add `tiny-web` to your `Cargo.toml` dependencies:

```toml
[dependencies]
tiny-web = "0.4"
tiny-web-macro = "0.1"
```

You also need to prepare a `tiny.conf` file in your web server. To do this, take the sample configuration file `tiny.sample.conf` and place it in the root of your project with the new name `tiny.conf`. And adjust the corresponding values. Be sure to change the `salt` parameter. In the future, the `tiny.conf` file will be created when the `install` command is executed.

## Usage

Just enter the following code to start the server

```rust
/// Actions (web controllers)
pub mod app;

fn main() {
    tiny_web::run(
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        env!("CARGO_PKG_DESCRIPTION"),
        || { tiny_web_macro::addfn!(); },
    );
}
```

## Web site

A commercial website will be created soon. The project can still be viewed on [https://github.com/tryteex/tiny-shop](https://github.com/tryteex/tiny-shop).

## Settings for Nginx server

The working configuration for Linux and Windows should look like this:

```nginx
worker_processes 7;

events {
  worker_connections  1024;
}

http {
    error_log /home/test/log/error.log;

    include mime.types;
    default_type application/octet-stream;

    sendfile on;
    client_max_body_size 2M;
    
    gzip on;
    proxy_buffering off;
    
    upstream fcgi_backend {
        server 127.0.0.1:12500;
        keepalive 32;
    }

    server {
        listen 443 ssl;
        http2 on;
        ssl_certificate /home/test/cert/certificate.crt;
        ssl_certificate_key /home/test/cert/privateKey.key;

        server_name fcgi.test.ua;
        root /home/test/www;
          
        location ~* ^.+\.(?:css|cur|js|jpg|gif|ico|png|xml|otf|ttf|eot|woff|woff2|svg|map)$ {
            break;
        }

        location ~\.(ini|html)$ {
            rewrite ^(.*)$ //$1/ last;
        }

        location ~ ^/$ {
            rewrite ^(.*)$ // last;
        }
        
        location ~ ^// {
            fastcgi_connect_timeout 1;
            fastcgi_next_upstream timeout;
            fastcgi_pass fcgi_backend;
            fastcgi_read_timeout 5d;
            fastcgi_param REDIRECT_URL $request_uri;
            include fastcgi_params;
            fastcgi_keep_conn on;
            fastcgi_buffering off;
            fastcgi_socket_keepalive on;
            fastcgi_ignore_client_abort on;
            break;
        }
        
        if (!-f $request_filename) {
            rewrite ^(.*)$ //$1 last;
        }
    }
}
```


## Dependencies

The `tiny-web` library depends on a number of other packages. A full list can be found in the Cargo.toml file.

## Contributing

If you'd like to contribute to tiny-web, check out our repository on GitHub.

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Community

Our project is in its infancy, if you want to join us, welcome!  
https://discord.gg/E8vZhjUDg8