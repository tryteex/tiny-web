## General concept

Each internet request is associated with a specific web controller. These controllers are grouped into a three-tier hierarchy: / __Module__ / __Class__ / __Action__ /.

The first level, __Module__, defines the global mechanisms of the web application, for example:

* __/api/__ – access to the API interface;
* __/admin/__ – site administration;
* __/index/__ – main site;
* and so on.

The second level, __Class__, establishes additional differentiation for each __Module__, for example:

* __/api/auth/__ – authentication system for the API;
* __/api/product/__ – requests for information related to products;
* __/api/delivery/__ – everything about logistics.

The third level, __Action__, corresponds to a specific controller, for example:

* __/api/product/list/__ – get a list of all products;
* __/api/product/get/__ – get complete information about a specific product;
* __/api/product/promo/__ – get a list of all products with a promo code.

Along with the third level, an additional parameter, __Param__, can be applied, adding detailed request parameters, for example:

* __/api/product/get/123__ – get complete information about the product with id=123.

In a sense, any web request is analyzed as a three-tier hierarchical structure, and the optional fourth level is not independent. For instance, to retrieve information about a product with id=4321 through the API, you need to make the following request:

    https://example.com/api/product/get/4321

If no part of the controller is specified in the request, it is replaced with '__index__':

* https://example.com/ => https://example.com/index/index/index/
* https://example.com/api/ => https://example.com/api/index/index/
* https://example.com/api/product/ => https://example.com/api/product/index/

The system should have two default controllers: __/index/index/index/__ and __/index/index/not_found/__. The __/index/index/not_found/__ controller is triggered when the specified controller is not found or when there is no access.

For SEO optimizations, route mechanisms are applied, which can transform any URL request into a pre-defined controller, that is, __/Module/Class/Action/Param__.
___
Next => Basic functionality [Basic.md](https://github.com/tryteex/tiny-web/blob/main/doc/Basic.md)  
Index => Contents [Index.md](https://github.com/tryteex/tiny-web/blob/main/doc/Index.md)  
