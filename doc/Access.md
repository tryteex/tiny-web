## Access system
Access is regulated at the controller level.

At the data level, the programmer must independently control access.

For controllers, access is regulated hierarchically using the __Module__ / __Class__ / __Action__ request.

If access is granted to any part of the hierarchy, access to the controller is considered granted. In other words, if access is granted to the `/api/` module, any controller within that module is allowed to be executed. By default, access is denied to all controllers.

Access is managed at the user role level. This means that all users with the same role have identical access.

Access permissions are stored in the [database ](https://github.com/tryteex/tiny-web/blob/main/doc/Database.md)and are cached upon the first request.

___
Next => Template maker [Template.md](https://github.com/tryteex/tiny-web/blob/main/doc/Template.md)
Index => Contents [Index.md](https://github.com/tryteex/tiny-web/blob/main/doc/Index.md)  
