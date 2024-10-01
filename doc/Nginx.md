## NGINX 
The library is designed to work with a web server in the form of a backend application.  
It can be used with any web server, but we have tested it with the Nginx server and utilize the FastCGI protocol.

Below is a working configuration for the Nginx server.

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
For the SCGI protocol, the settings will be the same
```nginx
    ...
    upstream scgi_backend { 
        server 127.0.0.1:12500;
        keepalive 32;
    }
    ...
        server_name scgi.test.ua;
    ...
        location ~ ^// {
            scgi_connect_timeout 1;
            scgi_next_upstream timeout;
            scgi_pass scgi_backend;
            scgi_read_timeout 5d;
            scgi_param REDIRECT_URL $request_uri;
            include scgi_params;
            scgi_buffering off;
            scgi_socket_keepalive on;
            break;
        }
    ...
```
For the UWSGI protocol, the settings will be the same
```nginx
    ...
    upstream uwsgi_backend { 
        server 127.0.0.1:12500;
        keepalive 32;
    } 
    ...
        server_name uwsgi.test.ua;
    ...
        location ~ ^// {
            uwsgi_connect_timeout 1;
            uwsgi_next_upstream timeout;
            uwsgi_pass uwsgi_backend;
            uwsgi_read_timeout 5d;
            uwsgi_param REDIRECT_URL $request_uri;
            include uwsgi_params;
            uwsgi_buffering off;
            uwsgi_socket_keepalive on;
            break;
        }      
    ...
```
For the HTTP protocol, the settings will be the same
```nginx
    ...
    upstream http_backend { 
        server 127.0.0.1:12500;
        keepalive 32;
    } 
    ...
        server_name http.test.ua;
    ...
        location ~ ^// {
            proxy_connect_timeout 1;
            proxy_next_upstream error timeout;
            proxy_pass http://http_backend; 
            proxy_read_timeout 5d;
            proxy_set_header X-Real-IP $remote_addr;
            proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
            proxy_set_header Host $http_host;
            proxy_set_header X-NginX-Proxy true;
            proxy_set_header X-Forwarded-Proto $scheme;
            proxy_set_header X-Request-URI $request_uri;
            proxy_set_header X-DOCUMENT_ROOT $document_root;
            proxy_http_version 1.1;
            proxy_set_header Connection "";
            proxy_buffering off;
            proxy_socket_keepalive on;
            break;
        }     
    ...
```
Additionally, it is recommended to note that the recommended file structure of the compiled application is provided [here](https://github.com/tryteex/tiny-web/blob/main/doc/Files.md).
___
Next => Simple example [https://github.com/tryteex/tiny-web/blob/main/doc/Example.md](https://github.com/tryteex/tiny-web/blob/main/doc/Example.md)  
Index => Contents [https://github.com/tryteex/tiny-web/blob/main/doc/Index.md](https://github.com/tryteex/tiny-web/blob/main/doc/Index.md)  