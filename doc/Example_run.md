## Configuration and Execution
* Our server runs on Ubuntu 22.04.
* User: `tiny`.
* Home directory: `/home/tiny` (indicated as `~`).
* Using Rust 1.77.0.
* All necessary programs and libraries are installed and configured.
* Demo site [https://demo1.tiny.com.ua/](https://demo1.tiny.com.ua/).
___
### Compilation of the Project
First, let's compile our application.  
We'll place it in: `/home/tiny/app/demo1/`:
```bash
~/
└── app/
    └── demo1/
        ├── src/
        │   └── *.rs
        ├── Cargo.toml
        ├── LICENSE
        ├── install.sql
        └── lib-install.sql
```
Type in the command line:
```bash
tiny@server:~/app/demo1$ cargo build --release
   Updating crates.io index
  Downloaded async-executor v1.8.0
...
  Downloaded 168 crates (13.4 MB) in 3.26s 
   Compiling proc-macro2 v1.0.79
...
   Compiling tiny-web v0.5.0
   Compiling tiny-demo v0.0.1 (/usr/usr/tiny/app/demo1)
    Finished release [optimized] target(s) in 21.13s
tiny@server:~/app/demo1$
```
The compiler has created the file `/home/tiny/app/demo1/target/release/tiny-demo`. 
___
### Placement of the Web Server
The compiled application will be placed in `/home/tiny/web/demo1/bin`, along with templates and translations.  
Additionally, we add the configuration file `tiny.toml` to this folder.   
Don't forget to add your own settings.
```bash
~/
└── web/
    └── demo1/
        └── bin/
            ├── app/
            │   ├── index/
            │   │   └── index/
            │   │       ├── foot.html
            │   │       ├── head.html
            │   │       ├── index.html
            │   │       ├── lang.en
            │   │       ├── lang.uk
            │   │       └── not_found.html
            │   └── review/
            │       └── index/
            │          ├── list.html
            │          ├── index.html
            │          ├── lang.en
            │          └── lang.uk
            ├── tiny-demo
            ├── LICENSE
            ├── tiny.sample.toml
            └── tiny.toml
```
The root folder for the website files will be located at `/home/tiny/web/demo1/www`.
```bash
~/
└── web/
    └── demo1/
        └── www/
            ├── css/
            │   └── styles.css
            ├── img/
            │   └── about-bg.jpg
            ├── js/
            │   ├── all.js
            │   ├── bootstrap.bundle.min.js
            │   └── scripts.js
            └── favicon.ico
```
___
### Installing the Database
The next step is to install tables.
All data is located in two files:
* `/home/tiny/app/demo1/lib-install.sql`
* `/home/tiny/app/demo1/install.sql`
___
### Configuration 
Nginx configuration file for `https.demo1.tiny.com.ua`.
```nginx
upstream demo1_tiny_fcgi_backend {
	server 127.0.0.1:12501 max_conns=5;
	keepalive 5;
}
server {
	listen 443 ssl http2;
	listen [::]:443 ssl http2;
	server_name demo1.tiny.com.ua;

    access_log /home/tiny/web/demo1/log/access.log;
    error_log /home/tiny/web/demo1/log/error.log;

	ssl_certificate /etc/letsencrypt/live/demo1.tiny.com.ua/fullchain.pem;
	ssl_certificate_key /etc/letsencrypt/live/demo1.tiny.com.ua/privkey.pem;
	ssl_trusted_certificate /etc/letsencrypt/live/demo1.tiny.com.ua/fullchain.pem;

	root /home/tiny/web/demo1/www;
	location / {
		autoindex off;

		location ~* ^.+\.(?:css|cur|js|jpg|gif|ico|png|xml|otf|ttf|eot|woff|woff2|svg)$ {
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
			fastcgi_pass demo1_tiny_fcgi_backend;
			fastcgi_read_timeout 5d;
		    fastcgi_param REDIRECT_URL $request_uri;
			include fastcgi_params;
			fastcgi_keep_conn on;
            fastcgi_buffering off;
            fastcgi_socket_keepalive on;
			break;
		}
        
		if (!-f $request_filename) {
			rewrite ^(.*)$ //$1 last;
		}
	}
}

```
Don't forget to restart the service:
```bash
tiny@server:~$ sudo service nginx reload
```
___
### Test Run
To perform a test run, execute the command:
```bash
tiny@server:~$ ./web/demo1/bin/tiny-demo start
tiny@server:~$
```
Check server startup:
```bash
tiny@server:~$ ps -a | grep tiny-demo
2759790 pts/0    00:00:00 tiny-demo
tiny@server:~$
```
Verify the website [https://demo1.tiny.com.ua/](https://demo1.tiny.com.ua/).  
All OK.

Stop the server:
```bash
tiny@server:~$ ./web/demo1/bin/tiny-demo stop
tiny@server:~$
```
___
### Automatic Start
Now we need to automate the start of our application so that it starts automatically and stops when the computer is shut down.

Creating a service file:
```bash
tiny@server:~$ sudo nano /etc/systemd/system/tiny-demo1.service
```
With the following content:
```bash
[Unit]
Description=Tiny-demo for tiny-web library

[Service]
Type=forking
Restart=always
ExecStart=/home/tiny/web/demo1/bin/tiny-demo start
ExecStop=/home/tiny/web/demo1/bin/tiny-demo stop
User=tiny
Group=tiny
WorkingDirectory=/home/tiny/web/demo1/bin/

[Install]
WantedBy=multi-user.target
```
Restarting systemd:
```bash
tiny@server:~$ sudo systemctl daemon-reload
```
Enable the service:
```bash
tiny@server:~$ sudo systemctl enable tiny-demo1.service
```
Start the service:
```bash
tiny@server:~$ sudo systemctl start tiny-demo1.service
```
Check the status:
```bash
tiny@server:~$ sudo systemctl status tiny-demo1.service
```
To verify, restart the server, and after startup, navigate to [https://demo1.tiny.com.ua/](https://demo1.tiny.com.ua/).
___
Top => Simple example [Example.md](https://github.com/tryteex/tiny-web/blob/main/doc/Example.md)   
Index => Contents [Index.md](https://github.com/tryteex/tiny-web/blob/main/doc/Index.md)  