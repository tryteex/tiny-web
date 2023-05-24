use std::{
    borrow::Cow,
    collections::BTreeMap,
    fs::{read_dir, read_to_string},
    sync::Arc,
};

use crate::fnv1a_64;

use super::{
    action::{Answer, Data},
    log::Log,
};

/// Describes a Node of template.
///
/// # Values
///
/// * `Text(String)` - Simple text.
/// * `Value(String)` - Some value.
/// * `ValueEscape(String)` - Escaped value: `&` => `&amp;`, `"` => `&quot;`, `'` => `&apos;`, `<` => `&lt;`, `>` => `&gt;`.
/// * `If(i64, Arc<Vec<Node>>, Arc<Vec<Node>>)` - If node. if `i64` key is `true` then use first `vec` else second `vec`.
/// * `Loop(i64, Arc<Vec<Node>>)` - Loop node. loop in `i64` key, use `vec`.
#[derive(Debug, Clone)]
pub enum Node {
    /// Simple text
    Text(String),
    /// Some value
    Value(String),
    /// Escaped value: `&` => `&amp;`, `"` => `&quot;`, `'` => `&apos;`, `<` => `&lt;`, `>` => `&gt;`.
    ValueEscape(String),
    /// If node. if `i64` key is `true` then use first `vec` else second `vec`.
    If(i64, Arc<Vec<Node>>, Arc<Vec<Node>>),
    /// Loop node. loop in `i64` key, use `vec`.
    Loop(i64, Arc<Vec<Node>>),
}

/// Describes a node type of template.
///
/// # Values
///
/// * `Value(&'a str)` - Value output found.
/// * `ValueEscape(&'a str)` - Escaped value output found.
/// * `If(&'a str)` - If found.
/// * `Else` - Else found.
/// * `EndIf` - EndIf found.
/// * `Loop(&'a str)` - Loop found.
/// * `EndLoop` - EndLoop found.
/// * `Comment` - Comment found.
/// * `Err` - Error recognizing the template.
enum TypeNode<'a> {
    /// Value output found
    Value(&'a str),
    /// Escaped value output found.
    ValueEscape(&'a str),
    /// If found.
    If(&'a str),
    /// Else found.
    Else,
    /// EndIf found.
    EndIf,
    /// Loop found.
    Loop(&'a str),
    /// EndLoop found.
    EndLoop,
    /// Comment found.
    Comment,
    /// Error recognizing the template.
    Err,
}

/// Describes a condition of Node
///
/// # Values
///
/// * `End` - End of template.
/// * `EndIf(usize)` - EndIf of If Node.
/// * `ElseIf(usize)` - ElseIf of If Node.
/// * `EndLoop(usize)` - EndLoop of Loop Node.
/// * `Err` - Error recognizing the template.
enum ConditionNode {
    /// End of template.
    End,
    /// EndIf of If Node.
    EndIf(usize),
    /// ElseIf of If Node.
    ElseIf(usize),
    /// EndLoop of Loop Node.
    EndLoop(usize),
    /// Error recognizing the template.
    Err,
}

/// Html template marker
///
/// # Values
///
/// * `list: BTreeMap<i64, BTreeMap<i64, Arc<BTreeMap<i64, Vec<Node>>>>>` - List of templates.
#[derive(Debug)]
pub struct Html {
    /// List of templates
    ///
    /// # Index
    ///
    /// * 1 - Module ID
    /// * 2 - Class ID
    /// * 3 - Template ID
    /// * 4 - List of Nodes
    #[allow(clippy::type_complexity)]
    pub list: BTreeMap<i64, BTreeMap<i64, Arc<BTreeMap<i64, Vec<Node>>>>>,
}

impl Html {
    /// Reads ./app/ and recognizes templates
    ///
    /// # Description
    ///
    /// In the root directory of the project (`Init::root_path`) the `app` directory is searched.
    ///
    /// Template files are logically located in this directory.  
    /// Each file must be named and have the extension `.html`
    ///
    /// ## Example:
    ///
    /// * index template:   ./app/module_name/class_name/index.html
    /// * not_found template: ./app/module_name/class_name/not_found.html
    ///
    /// module_name - Name of the module  <br />
    /// class_name - Class name  
    ///
    /// ## Use in the template:
    ///
    /// To get a template, it is enough to set the `this.render("template")` function <br />
    /// If no template is found, the asnwer will be None.
    ///
    /// # Used symbols
    ///
    /// * `{% str %}` - Unescaped output.
    /// * `{%+ str %}` - Escaped output.
    /// * `{%# comment %}` - Comment.
    /// * `{%? bool %}` - If.
    /// * `{%?~%}` - Else.
    /// * `{%?%}` - End if.
    /// * `{%@ arr %}` - Loop vec.
    /// * `{%@%}` - End loop.
    pub async fn new(root: &str) -> Html {
        let path = format!("{}/app/", root);
        let read_path = match read_dir(&path) {
            Ok(r) => r,
            Err(e) => {
                Log::warning(
                    1100,
                    Some(format!("Path: {}. Err: {}", path, e)),
                );
                return Html {
                    list: BTreeMap::new(),
                };
            }
        };

        let mut list = BTreeMap::new();
        // Read first level dir
        for entry in read_path {
            let path = match entry {
                Ok(e) => e.path(),
                Err(e) => {
                    Log::warning(1101, Some(format!("{} ({})", e, path)));
                    continue;
                }
            };
            if !path.is_dir() {
                continue;
            }
            let module = match path.file_name() {
                Some(m) => match m.to_str() {
                    Some(module) => module,
                    None => continue,
                },
                None => continue,
            };
            let read_path = match read_dir(&path) {
                Ok(r) => r,
                Err(e) => {
                    Log::warning(1102, Some(format!("{} ({})", e, path.display())));
                    continue;
                }
            };
            let mut ls = BTreeMap::new();
            // Read second level dir
            for entry in read_path {
                let path = match entry {
                    Ok(e) => e.path(),
                    Err(e) => {
                        Log::warning(1101, Some(format!("{} ({})", e, path.display())));
                        continue;
                    }
                };
                if !path.is_dir() {
                    continue;
                }
                let class = match path.file_name() {
                    Some(c) => match c.to_str() {
                        Some(class) => class,
                        None => continue,
                    },
                    None => continue,
                };
                let read_path = match read_dir(&path) {
                    Ok(r) => r,
                    Err(e) => {
                        Log::warning(1102, Some(format!("{} ({})", e, path.display())));
                        continue;
                    }
                };
                let mut l = BTreeMap::new();
                // Read third level dir
                for entry in read_path {
                    let path = match entry {
                        Ok(e) => e.path(),
                        Err(e) => {
                            Log::warning(1101, Some(format!("{} ({})", e, path.display())));
                            continue;
                        }
                    };
                    if !path.is_file() {
                        continue;
                    }
                    let view = match path.file_name() {
                        Some(v) => match v.to_str() {
                            Some(view) => view,
                            None => continue,
                        },
                        None => continue,
                    };
                    if view.ends_with(".html") && view.len() > 5 {
                        if let Ok(str) = read_to_string(&path) {
                            let view = &view[..view.len() - 5];
                            let mut vec = Vec::new();
                            if let ConditionNode::End = Html::get_view(&str, &mut vec) {
                                vec.shrink_to_fit();
                                l.insert(fnv1a_64(view), vec);
                            }
                        }
                    }
                }
                ls.insert(fnv1a_64(class), Arc::new(l));
            }
            list.insert(fnv1a_64(module), ls);
        }
        Html { list }
    }

    /// Gets temptale from String
    fn get_view(html: &str, vec: &mut Vec<Node>) -> ConditionNode {
        let len = html.len();
        if len == 0 {
            return ConditionNode::End;
        }
        let mut ind = 0;
        while let Some(b) = html[ind..].find("{%") {
            if b != 0 {
                vec.push(Node::Text(html[ind..ind + b].to_owned()))
            }
            match html[ind + b..].find("%}") {
                Some(e) => {
                    let mut shift = 0;
                    match Html::get_type_node(&html[ind + b..ind + b + e + 2]) {
                        TypeNode::Value(name) => vec.push(Node::Value(name.to_owned())),
                        TypeNode::ValueEscape(name) => vec.push(Node::ValueEscape(name.to_owned())),
                        TypeNode::If(name) => {
                            let mut vt = Vec::new();
                            let mut vf = Vec::new();
                            match Html::get_view(&html[ind + b + e + 2..], &mut vt) {
                                ConditionNode::EndIf(i) => {
                                    vec.push(Node::If(fnv1a_64(name), Arc::new(vt), Arc::new(vf)));
                                    shift = i;
                                }
                                ConditionNode::ElseIf(i) => {
                                    match Html::get_view(&html[ind + b + e + 2 + i..], &mut vf) {
                                        ConditionNode::EndIf(j) => {
                                            vec.push(Node::If(
                                                fnv1a_64(name),
                                                Arc::new(vt),
                                                Arc::new(vf),
                                            ));
                                            shift = i + j;
                                        }
                                        ConditionNode::Err => return ConditionNode::Err,
                                        _ => {
                                            Log::warning(
                                                1201,
                                                Some(html[ind..ind + b + e + 2].to_owned()),
                                            );
                                            return ConditionNode::Err;
                                        }
                                    };
                                }
                                ConditionNode::Err => return ConditionNode::Err,
                                _ => {
                                    Log::warning(1201, Some(html[ind..ind + b + e + 2].to_owned()));
                                    return ConditionNode::Err;
                                }
                            };
                        }
                        TypeNode::Else => return ConditionNode::ElseIf(ind + b + e + 2 + shift),
                        TypeNode::EndIf => return ConditionNode::EndIf(ind + b + e + 2 + shift),
                        TypeNode::Loop(name) => {
                            let mut v = Vec::new();
                            match Html::get_view(&html[ind + b + e + 2..], &mut v) {
                                ConditionNode::EndLoop(i) => {
                                    vec.push(Node::Loop(fnv1a_64(name), Arc::new(v)));
                                    shift = i;
                                }
                                ConditionNode::Err => return ConditionNode::Err,
                                _ => {
                                    Log::warning(1202, Some(html[ind..ind + b + e + 2].to_owned()));
                                    return ConditionNode::Err;
                                }
                            };
                        }
                        TypeNode::EndLoop => {
                            return ConditionNode::EndLoop(ind + b + e + 2 + shift)
                        }
                        TypeNode::Comment => {}
                        TypeNode::Err => return ConditionNode::Err,
                    };
                    ind += b + e + 2 + shift;
                }
                None => break,
            };
        }
        if ind < len {
            vec.push(Node::Text(html[ind..].to_owned()));
        }
        ConditionNode::End
    }

    /// Detect type of Node
    fn get_type_node(text: &str) -> TypeNode {
        let len = text.len();
        match len {
            4 => {
                Log::warning(1200, Some(text.to_owned()));
                return TypeNode::Err;
            }
            5 => {
                match &text[2..3] {
                    "?" => return TypeNode::EndIf,
                    "@" => return TypeNode::EndLoop,
                    _ => {
                        Log::warning(1200, Some(text.to_owned()));
                        return TypeNode::Err;
                    }
                };
            }
            6 => {
                match &text[2..4] {
                    "?~" => return TypeNode::Else,
                    _ => {
                        Log::warning(1200, Some(text.to_owned()));
                        return TypeNode::Err;
                    }
                };
            }
            _ => {}
        }

        if &text[2..3] == " " && &text[len - 3..len - 2] == " " {
            return TypeNode::Value(&text[3..len - 3]);
        };
        if &text[2..4] == "+ " && &text[len - 3..len - 2] == " " {
            return TypeNode::ValueEscape(&text[4..len - 3]);
        };
        if &text[2..4] == "# " && &text[len - 3..len - 2] == " " {
            return TypeNode::Comment;
        };
        if &text[2..4] == "? " && &text[len - 3..len - 2] == " " {
            return TypeNode::If(&text[4..len - 3]);
        };
        if &text[2..4] == "@ " && &text[len - 3..len - 2] == " " {
            return TypeNode::Loop(&text[4..len - 3]);
        };
        Log::warning(1200, Some(text.to_owned()));
        TypeNode::Err
    }

    /// Renders of template
    ///
    /// # Values
    ///
    /// * `data: &BTreeMap<i64, Data>` - Keys with data.
    /// * `list: &Vec<Node>` - List of Nodes.
    /// * `add: Option<&BTreeMap<i64, Data>>` - Additional list of Nodes for If of Loop.
    pub fn render(
        data: &BTreeMap<i64, Data>,
        list: &Vec<Node>,
        add: Option<&BTreeMap<i64, Data>>,
    ) -> Answer {
        let mut render = Vec::with_capacity(list.len());
        for node in list.iter() {
            match node {
                Node::Text(t) => render.push(t.to_owned()),
                Node::Value(v) => {
                    let key = &fnv1a_64(v);
                    if let Some(d) = data.get(key).or_else(|| add.and_then(|a| a.get(key))) {
                        match d {
                            Data::U8(v) => render.push(v.to_string()),
                            Data::I64(v) => render.push(v.to_string()),
                            Data::U64(v) => render.push(v.to_string()),
                            Data::F64(v) => render.push(v.to_string()),
                            Data::String(v) => render.push(v.to_owned()),
                            _ => {}
                        }
                    }
                }
                Node::ValueEscape(v) => {
                    let key = &fnv1a_64(v);
                    if let Some(d) = data.get(key).or_else(|| add.and_then(|a| a.get(key))) {
                        match d {
                            Data::U8(v) => render.push(v.to_string()),
                            Data::I64(v) => render.push(v.to_string()),
                            Data::U64(v) => render.push(v.to_string()),
                            Data::F64(v) => render.push(v.to_string()),
                            Data::String(v) => render.push(Html::escape(v).to_string()),
                            _ => {}
                        }
                    }
                }
                Node::If(key, vec_true, vec_false) => {
                    if let Some(Data::Bool(b)) = data.get(key) {
                        if *b && vec_true.len() > 0 {
                            if let Answer::String(a) = Html::render(data, vec_true, add) {
                                render.push(a);
                            }
                        } else if !*b && vec_false.len() > 0 {
                            if let Answer::String(a) = Html::render(data, vec_false, add) {
                                render.push(a);
                            }
                        }
                    }
                }
                Node::Loop(key, vec) => {
                    if let Some(Data::Vec(v)) = data.get(key) {
                        for i in v {
                            if let Data::Map(m) = i {
                                if let Answer::String(a) = Html::render(data, vec, Some(m)) {
                                    render.push(a);
                                }
                            }
                        }
                    }
                }
            }
        }
        if render.is_empty() {
            Answer::None
        } else {
            Answer::String(render.join(""))
        }
    }

    /// Escape text
    fn escape(text: &str) -> Cow<'_, str> {
        let t = text.as_bytes();
        let mut len = 0;

        for b in t {
            len += match b {
                b'&' => 5,
                b'"' | b'\'' => 6,
                b'<' | b'>' => 4,
                _ => 0,
            };
        }
        if len == 0 {
            return Cow::Borrowed(text);
        }
        let mut new_text = String::with_capacity(text.len() + len);
        for c in text.chars() {
            match c {
                '&' => new_text.push_str("&amp;"),
                '"' => new_text.push_str("&quot;"),
                '\'' => new_text.push_str("&apos;"),
                '<' => new_text.push_str("&lt;"),
                '>' => new_text.push_str("&gt;"),
                _ => new_text.push(c),
            };
        }
        return Cow::Owned(new_text);
    }
}
