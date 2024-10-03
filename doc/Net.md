## Net protocols

To run your application, we recommend using an external web server. We suggest Nginx, as our library testing was conducted specifically with this web server.

The external web server accepts requests from clients and forwards them to your application using the following protocols:
* FastCGI — recommended.
* UWSGI (modifier1=0).
* SCGI.
* HTTP / HTTPS.

The HTTP and HTTPS protocols support the following versions:
* HTTP/1.0
* HTTP/1.1

To configure SSL, you need to create an `ssl` directory in the root of the project or alongside the binary file of your application in the release version. You should place the certificate file `certificate.crt` and the private key `privateKey.key` in this directory.

Private key formats can be:
* RSA (Pkcs1)
* Sec1
* PKCS#8 (Pkcs8)

The library does not perform verification of certificates and keys during the check.
___
Next => Configuring nginx [https://github.com/tryteex/tiny-web/blob/main/doc/Nginx.md](https://github.com/tryteex/tiny-web/blob/main/doc/Nginx.md)
Index => Contents [https://github.com/tryteex/tiny-web/blob/main/doc/Index.md](https://github.com/tryteex/tiny-web/blob/main/doc/Index.md)  