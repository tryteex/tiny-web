use std::collections::BTreeMap;

use tiny_web::{
    fnv1a_64,
    sys::action::{Action, Answer, Data},
};

pub async fn index(this: &mut Action) -> Answer {
    this.load("head", "index", "index", "head", None).await;
    this.load("foot", "index", "index", "foot", None).await;
    this.load("index", "review", "index", "index", None).await;
    this.render("index")
}

pub async fn head(this: &mut Action) -> Answer {
    this.set("title", Data::String(this.lang("title")));
    this.set("home", Data::String(this.lang("home")));
    this.set("about", Data::String(this.lang("about")));
    this.set("i_do", Data::String(this.lang("i_do")));
    this.set("menu", Data::String(this.lang("menu")));
    this.set("lang", Data::String(this.lang_current().lang.clone()));
    this.set("lang_name", Data::String(this.lang_current().name.clone()));
    let list = this.lang_list();
    let mut vec = Vec::with_capacity(list.len());
    for item in this.lang_list() {
        let mut map = BTreeMap::new();
        map.insert(fnv1a_64(b"id"), Data::I64(item.id));
        map.insert(fnv1a_64(b"name"), Data::String(item.name.to_owned()));
        vec.push(Data::Map(map));
    }
    this.set("lang_list", Data::Vec(vec));
    this.render("head")
}

pub async fn foot(this: &mut Action) -> Answer {
    this.render("foot")
}

pub async fn not_found(this: &mut Action) -> Answer {
    this.load("head", "index", "index", "head", None).await;
    this.load("foot", "index", "index", "foot", None).await;
    this.render("not_found")
}

pub async fn lang(this: &mut Action) -> Answer {
    if let Some(param) = &this.param {
        if let Ok(id) = param.parse::<i64>() {
            let list = this.lang_list();
            for item in list {
                if item.id == id {
                    this.session.set_lang_id(id);
                    break;
                }
            }
        }
    }
    Answer::None
}
