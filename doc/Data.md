## Struct __Data__
All data that can be passed into an HTTP template or read from the database is always represented in the following format:
```rust
pub enum Data {
    None,
    Usize(usize),
    I16(i16),
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
    Bool(bool),
    String(String),
    Date(DateTime<Utc>),
    Json(Value),
    Vec(Vec<Data>),
    Raw(Vec<u8>),
    Map(BTreeMap<i64, Data>),
}
```
___
### Function ```group```
Дані, які ми отримуємо з бази даних сформовані в форматі Data::Vec<Data::Map<i64, Data>> або Data::Vec<Vec<Data>> часто потрібно перетворити на іерархічні данні (древодвидної структури).

fn group(&mut self, Vec<Vec<Data>>) {

}





___
Additionally, there are auxiliary enumerations used exclusively within the library.

In the future, more types will be added.
___
Next => Access system [Access.md](https://github.com/tryteex/tiny-web/blob/main/doc/Access.md)
Index => Contents [Index.md](https://github.com/tryteex/tiny-web/blob/main/doc/Index.md)  
