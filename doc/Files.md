## File structure of the project
It is recommended to use two independent file structures for __Development__ and __Production__.  
These structures are very similar and have minimal differences.

For __Development__, each project should be structured as follows:

```bash
├── tiny-shop/
│    ├── Cargo.toml
│    ├── tiny.toml
│    ├── app/
│    │    ├── module___/
│    │    │    ├── class___/
│    │    │    │    ├── html___.html
│    │    │    │    ├── lang.___
│    ├── www/
│    ├── src/
│    │    ├── main.rs
│    │    ├── app/
│    │    │    ├── mod.rs
│    │    │    ├── module___/
│    │    │    │    ├── class___.rs
│    ├── target/
```
For __Production__, each project should be structured as follows:

```bash
├── tiny-shop/
│    ├── bin/
│    │    ├── tiny-shop.exe
│    │    ├── tiny.toml
│    │    ├── app/
│    │    │    ├── module___/
│    │    │    │    ├── class___/
│    │    │    │    │    ├── html___.html
│    │    │    │    │    ├── lang.___
│    ├── www/
```

"___" means that there may be several files/directories.

Translation files (lang.___) ".___" means that the files have an ending depending on the language. The ending itself is set according to ISO 639-1: `lang.uk` - Ukrainian, `lang.en` - English. This files must be in [TOML](https://toml.io/) format.

The directort ```www``` contains multimedia files for your site, such as ```css```, ```images```, ```fonts```, etc

The root directort ```app``` contains html templates.

Accordingly, the nginx server settings will be different for Development and Production.
___
Next => Config [Config.md](https://github.com/tryteex/tiny-web/blob/main/doc/Config.md)  
Index => Contents [Index.md](https://github.com/tryteex/tiny-web/blob/main/doc/Index.md)  
