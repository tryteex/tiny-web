## Template maker

Upon launching the application, the library recursively scans the directory `/app/module/class`, searching for files with the `.html` extension. These files serve as templates for corresponding __Module__-__Class__ pairs. The template's name is the file name without the extension (without `.html`). Each template can be executed (rendered) from the respective controller.
```rust
pub async fn action(this: &mut Action) -> Answer {
    // Render template from file head.html
    this.render("head")
}
```
As a result, a response is generated in the form of plain HTML text in the `Answer::String(html)` format.

Each template consists of HTML text and special expressions that will be replaced during rendering.

| Expression | Description |
| - | - |
| __Output__ | | 
| @ | If it is at the beginning, do not modify the expression after it, just remove this symbol |
| {{&nbsp;name&nbsp;}} | Output text and escape it (replace characters & " ' < > with corresponding HTML entities) |
| {{-&nbsp;name&nbsp;}} | Trim White_Space from the left |
| {{&nbsp;name&nbsp;-}} | Trim White_Space from the right |
| {{&nbsp;name\|raw&nbsp;}} | Output text without escaping |
| {{&nbsp;name\|dump&nbsp;}} | Output full dump of value |
| {{&nbsp;name.title&nbsp;}} | Output text from nested variable|
| {{&nbsp;name.title.title_ua&nbsp;}} | Output text from nested variables one by one |
| {{#&nbsp;comment&nbsp;#}} | Comment |
| __Condition__ | | 
| {%&nbsp;if&nbsp;bool&nbsp;%} | Start of a condition |
| {%&nbsp;elseif&nbsp;...&nbsp;%} | Additional condition |
| {%&nbsp;else&nbsp;...&nbsp;%} | Else condition |
| {%&nbsp;endif&nbsp;%} | End of a condition |
| {%&nbsp;if&nbsp;bool\|len&nbsp;%} | Start of a condition, check for length |
| {%&nbsp;if&nbsp;bool\|set&nbsp;%} | Start of a condition, check for existence |
| {%&nbsp;if&nbsp;bool\|unset&nbsp;%} | Start of a condition, check for non-existence |
| {%&nbsp;if&nbsp;int&nbsp;>&nbsp;0&nbsp;%} | Start of a condition, check for a logical variable, allowed operators are > >= < <= = != |
| __Loop__ | | 
| {%&nbsp;for&nbsp;arr&nbsp;in&nbsp;array&nbsp;%} | Start of a loop |
| &nbsp;&nbsp;&nbsp;&nbsp;{{&nbsp;arr&nbsp;}} | Output loop variable with escaping |
| &nbsp;&nbsp;&nbsp;&nbsp;{{&nbsp;arr\|key&nbsp;}} | Output index/key of loop variable if array is `Data::Vec`/`Data::Map` |
| &nbsp;&nbsp;&nbsp;&nbsp;{{&nbsp;arr.title\|raw&nbsp;}} | Output loop sub variable without escaping |
| {%&nbsp;elsefor&nbsp;%} | Empty or null array |
| {%&nbsp;endfor&nbsp;%} | End of the loop |

___
In debug, when the `#[cfg(debug_assertions)]` attribute is set, the library checks for changes in templates files at each new connection, and automatically loads the changes.  
In the release, when the `#[cfg(debug_assertions)]` attribute does not work, the library loads the templates files only once, when the application starts.
___
### Examples
___
#### Example 1
Template
```html
<br>Block 1<br>
Value 1: {{ name1 }} end;<br>
Value 2: {{ name2.app }} end;<br>
Value 3: {{ name3.app.name }} end;<br>
```
Data
```rust
// set name1
this.set("name1", Data::String("Hello =>".to_string()));

// set name2.app
let mut map = BTreeMap::new();
map.insert(fnv1a_64("app"), Data::String("Hello =>>".to_string()));
this.set("name2", Data::Map(map));

// set name3.app.name
let mut sub_map = BTreeMap::new();
sub_map.insert(fnv1a_64("name"), Data::String("Hello =>>>".to_string()));
let mut map = BTreeMap::new();
map.insert(fnv1a_64("app"), Data::Map(sub_map));
this.set("name3", Data::Map(map));
```
Result
```html
<br>Block 1<br>
Value 1: Hello =&gt; end;<br>
Value 2: Hello =>> end;<br>
Value 3: Hello =&gt;&gt;&gt; end;<br>
```
___
#### Example 2
Template
```html
<br>Block 2<br>
Value 1:   {{- Tname }} end;<br>
Value 2: {{ nameT -}}  end;<br>
Value 3:    {{- TnameT -}}  end;<br>
```
Data
```rust
// set Tname
this.set("Tname", Data::String("   Hello   ".to_string()));

// set nameT
this.set("nameT", Data::String("   Hello   ".to_string()));

// set TnameT
this.set("TnameT", Data::String("   Hello   ".to_string()));

```
Result
```html
<br>Block 2<br>
Value 1:Hello    end;<br>
Value 2:    Helloend;<br>
Value 3:Helloend;<br>
```
___
#### Example 3
Template
```html
<br>Block 3<br>
Value 1: @@@{{ name }} end;
Value 2: @{# name #} end;
Value 3: {# name #} end;
```
Data
```rust
this.set("name", Data::String("Hello".to_string()));
```
Result
```html
<br>Block 3<br>
Value 1: @@{{ name }} end;
Value 2: {# name #} end;
Value 3:  end;
```
___
#### Example 4
Template
```html
<br>Block 4<br>
{% if bool %} if-bool
{% elseif bool.test|set %} bool.test|set
{% else %} else
{% endif %}
```
Data
```rust
let mut map = BTreeMap::new();
map.insert(fnv1a_64("test".as_bytes()), Data::String("test".to_owned()));
this.set("bool", Data::Map(map));
```
Result
```html
<br>Block 4<br>
bool.test|set 
```
___
#### Example 5
Template
```html
<br>Block 5<br>
Value arr.name: {{ arr.name }} end;<br>
{% for a in arr.array %}
    Value a.body: {{ a.body }} end;<br>
    {% for b in a.array %}
        Value a|idx: {{ a|idx }} end;<br>
        Value b|idx: {{ b|idx }} end;<br>
        Value a.sub: {{ a.sub }} end;<br>
        Value b: {{ b }} end;<br>
    {% endfor %}
{% endfor %}
```
Data
```rust
let mut map = BTreeMap::new();
map.insert(fnv1a_64("name".as_bytes()), Data::String("A.Name".to_owned()));

let mut item1 = BTreeMap::new();
item1.insert(fnv1a_64("body".as_bytes()), Data::String("Body of Item1".to_owned()));
item1.insert(fnv1a_64("sub".as_bytes()), Data::String("Sub body of Item1".to_owned()));
item1.insert(fnv1a_64("array".as_bytes()), 
    Data::Vec(vec![
        Data::String("First value of Item1".to_owned()),
        Data::String("Second value of Item1".to_owned()),
        Data::String("Third value of Item1".to_owned()),
    ])
);

let mut item2 = BTreeMap::new();
item2.insert(fnv1a_64("body".as_bytes()), Data::String("Body of Item2".to_owned()));
item2.insert(fnv1a_64("sub".as_bytes()), Data::String("Sub body of Item2".to_owned()));
item2.insert(fnv1a_64("array".as_bytes()), 
    Data::Vec(vec![
        Data::String("First value of Item2".to_owned()),
        Data::String("Second value of Item2".to_owned()),
        Data::String("Third value of Item2".to_owned()),
    ])
);

let mut item3 = BTreeMap::new();
item3.insert(fnv1a_64("body".as_bytes()), Data::String("Body of Item3".to_owned()));
item3.insert(fnv1a_64("sub".as_bytes()), Data::String("Sub body of Item3".to_owned()));
item3.insert(fnv1a_64("array".as_bytes()), 
    Data::Vec(vec![
        Data::String("First value of Item3".to_owned()),
        Data::String("Second value of Item3".to_owned()),
        Data::String("Third value of Item3".to_owned()),
    ])
);

map.insert(fnv1a_64("array".as_bytes()), Data::Vec(vec![Data::Map(item1), Data::Map(item2), Data::Map(item3)]));
this.set("arr", Data::Map(map));
```
Result
```html
<br>Block 5<br>
Value arr.name: A.Name end;<br>

    Value a.body: Body of Item1 end;<br>
    
        Value a|idx: 1 end;<br>
        Value b|idx: 1 end;<br>
        Value a.sub: Sub body of Item1 end;<br>
        Value b: First value of Item1 end;<br>
    
        Value a|idx: 1 end;<br>
        Value b|idx: 2 end;<br>
        Value a.sub: Sub body of Item1 end;<br>
        Value b: Second value of Item1 end;<br>
    
        Value a|idx: 1 end;<br>
        Value b|idx: 3 end;<br>
        Value a.sub: Sub body of Item1 end;<br>
        Value b: Third value of Item1 end;<br>
    

    Value a.body: Body of Item2 end;<br>
    
        Value a|idx: 2 end;<br>
        Value b|idx: 1 end;<br>
        Value a.sub: Sub body of Item2 end;<br>
        Value b: First value of Item2 end;<br>
    
        Value a|idx: 2 end;<br>
        Value b|idx: 2 end;<br>
        Value a.sub: Sub body of Item2 end;<br>
        Value b: Second value of Item2 end;<br>
    
        Value a|idx: 2 end;<br>
        Value b|idx: 3 end;<br>
        Value a.sub: Sub body of Item2 end;<br>
        Value b: Third value of Item2 end;<br>
    

    Value a.body: Body of Item3 end;<br>
    
        Value a|idx: 3 end;<br>
        Value b|idx: 1 end;<br>
        Value a.sub: Sub body of Item3 end;<br>
        Value b: First value of Item3 end;<br>
    
        Value a|idx: 3 end;<br>
        Value b|idx: 2 end;<br>
        Value a.sub: Sub body of Item3 end;<br>
        Value b: Second value of Item3 end;<br>
    
        Value a|idx: 3 end;<br>
        Value b|idx: 3 end;<br>
        Value a.sub: Sub body of Item3 end;<br>
        Value b: Third value of Item3 end;<br>
```
___
Next => I18N [https://github.com/tryteex/tiny-web/blob/main/doc/I18N.md](https://github.com/tryteex/tiny-web/blob/main/doc/I18N.md)  
Index => Contents [https://github.com/tryteex/tiny-web/blob/main/doc/Index.md](https://github.com/tryteex/tiny-web/blob/main/doc/Index.md)  