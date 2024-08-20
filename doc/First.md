## First-Time start

When the program starts, if the configuration file is not found, the library runs in "single" mode.

This means that there are no database connections. When the stop signal is sent, the program automatically restarts and checks for the configuration file. The user will be able to perform the initial setup, which will save the configuration file, and then the program will start in standard operation mode.

On first launch (without config file) only the controller from `/index/install/*` will be available.
___
Next => Configuring nginx [Nginx.md](https://github.com/tryteex/tiny-web/blob/main/doc/Nginx.md)  
Index => Contents [Index.md](https://github.com/tryteex/tiny-web/blob/main/doc/Index.md)  