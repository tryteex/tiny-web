use tiny_web::sys::action::{Action, Answer, Data};
use tiny_web_macro::fnv1a_64;

pub async fn index(this: &mut Action) -> Answer {
    this.load("list", "review", "index", "list", None).await;
    this.set("review_add", Data::String(this.lang("review_add")));
    this.set("review_refresh", Data::String(this.lang("review_refresh")));
    this.set("http_error", Data::String(this.lang("http_error")));
    this.set("add_new", Data::String(this.lang("add_new")));
    this.set("edit_last", Data::String(this.lang("edit_last")));
    this.set("name", Data::String(this.lang("name")));
    this.set("text", Data::String(this.lang("text")));
    this.set("close", Data::String(this.lang("close")));
    this.set("save", Data::String(this.lang("save")));
    this.render("index")
}

pub async fn list(this: &mut Action) -> Answer {
    if let Some(list) = this
        .db
        .query(fnv1a_64!("list_review"), &[&this.session.key], true)
        .await
    {
        if list.is_empty() {
            this.set(
                "reviews_is_empty",
                Data::String(this.lang("reviews_is_empty")),
            );
        } else {
            let total = if let Data::Map(map) = unsafe { list.get_unchecked(0) } {
                if let Some(Data::I64(t)) = map.get(&tiny_web::fnv1a_64(b"total")) {
                    *t
                } else {
                    0
                }
            } else {
                0
            };
            this.set("total", Data::I64(total));
            this.set("total_text", Data::String(this.lang("total")));
        }
        this.set("reviews", Data::Vec(list));
    } else {
        this.set("error_db", Data::String(this.lang("error_db")));
    }
    this.render("list")
}

pub async fn save(this: &mut Action) -> Answer {
    let name = match this.request.input.post.remove("name") {
        Some(n) => {
            if n.is_empty() {
                return Answer::String(this.lang("name_is_empty"));
            } else if n.chars().count() > 100 {
                return Answer::String(this.lang("name_is_big"));
            } else {
                n
            }
        }
        None => return Answer::String(this.lang("name_is_not_set")),
    };
    let review = match this.request.input.post.remove("review") {
        Some(r) => {
            if r.is_empty() {
                return Answer::String(this.lang("review_is_empty"));
            } else if r.chars().count() > 300 {
                return Answer::String(this.lang("review_is_big"));
            } else {
                r
            }
        }
        None => return Answer::String(this.lang("review_is_not_set")),
    };
    if this
        .db
        .query(
            fnv1a_64!("add_review"),
            &[
                &name,
                &this.request.ip,
                &this.request.agent,
                &this.session.key,
                &review,
            ],
            false,
        )
        .await
        .is_none()
    {
        return Answer::String(this.lang("error_db"));
    }

    Answer::String("ok".to_owned())
}

pub async fn edit(this: &mut Action) -> Answer {
    let name = match this.request.input.post.remove("name") {
        Some(n) => {
            if n.is_empty() {
                return Answer::String(this.lang("name_is_empty"));
            } else if n.chars().count() > 100 {
                return Answer::String(this.lang("name_is_big"));
            } else {
                n
            }
        }
        None => return Answer::String(this.lang("name_is_not_set")),
    };
    let review = match this.request.input.post.remove("review") {
        Some(r) => {
            if r.is_empty() {
                return Answer::String(this.lang("review_is_empty"));
            } else if r.chars().count() > 300 {
                return Answer::String(this.lang("review_is_big"));
            } else {
                r
            }
        }
        None => return Answer::String(this.lang("review_is_not_set")),
    };
    let id = match this.request.input.post.remove("id") {
        Some(r) => match r.parse::<i64>() {
            Ok(i) => i,
            Err(_) => return Answer::String(this.lang("error_edit")),
        },
        None => return Answer::String(this.lang("error_edit")),
    };
    let res = this
        .db
        .query(
            fnv1a_64!("edit_review"),
            &[&name, &review, &this.session.key, &id],
            false,
        )
        .await;
    match res {
        Some(v) => {
            if v.is_empty() {
                return Answer::String(this.lang("error_edit"));
            }
        }
        None => return Answer::String(this.lang("error_db")),
    }
    Answer::String("ok".to_owned())
}
