use std::{
    collections::{btree_map::Entry, HashMap},
    fs::{read_dir, read_to_string},
    path::PathBuf,
    sync::Arc,
};

#[cfg(feature = "html-reload")]
use std::sync::atomic::Ordering;

#[cfg(feature = "html-reload")]
use std::time::SystemTime;

#[cfg(feature = "html-reload")]
use tokio::fs;

#[cfg(feature = "html-reload")]
use tokio::sync::OnceCell;

#[cfg(feature = "html-reload")]
use tokio::sync::RwLock;

use crate::{fnv1a_64, log};

#[cfg(feature = "html-reload")]
use crate::sys::wrlock::WrLock;

use super::{action::Answer, data::Data};

/// The filter of the variable
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Filter {
    /// None filter
    None,
    /// Do not escape the output
    Raw,
    /// Use function len()
    Len,
    /// Values is set
    Set,
    /// Values is unset
    Unset,
    /// Index of loop
    Index,
    /// Dump of value
    Dump,
}

/// The value of the variable
///
/// # Values
///
/// * `Number(i64)` - i64 value.
/// * `Value { name: Vec<String>, filter: Filter }` - Variable name and its filter.
#[derive(Debug, Clone)]
pub(crate) enum Value {
    /// i64 value
    Number(i64),
    /// Variable name and its filter
    Value { name: Vec<String>, filter: Filter },
}

/// For branch
///
/// # Values
///
/// * `name: Value` - The value.
/// * `local: String` - Local (inside) name of value.
/// * `nodes: Nodes` - Nodes inside a loop.
/// * `empty: Option<Nodes>` - Nodes that will be executed if the cycle is empty.
#[derive(Debug, Clone)]
pub(crate) struct For {
    /// The value
    name: Value,
    /// Local (inside) name of value
    local: String,
    /// Nodes inside a loop
    nodes: Nodes,
    /// Nodes that will be executed if the cycle is empty
    empty: Option<Nodes>,
}

/// Equality comparisons
///
/// # Values
///
/// * `None` - None comparisons.
/// * `Eq` - `==`.
/// * `Ne` - `!=`.
/// * `Lt` - `<`.
/// * `Le` - `<=`.
/// * `Gt` - `>`.
/// * `Ge` - `>=`.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Eq {
    /// None comparisons
    None,
    /// `==`
    Equal,
    /// `!=`
    NotEqual,
    /// `<`
    LessThan,
    /// `<=`
    LessThanOrEqual,
    /// `>`
    GreaterThan,
    /// `>=`
    GreaterThanOrEqual,
}

/// Condition for `If`
///
/// # Values
///
/// * `val: Value` - Value.
/// * `eq: Eq` - Equality comparisons.
/// * `other: Option<Value>` - Second values if needed.
#[derive(Debug, Clone)]
pub(crate) struct ExpValue {
    /// Value
    val: Value,
    /// Equality comparisons
    eq: Eq,
    /// Second values if needed
    other: Option<Value>,
}

/// Expression for `If`
///
/// # Values
///
/// * `val: ExpValue` - Condition for `If`.
/// * `nodes: Nodes` - Nodes if true.
#[derive(Debug, Clone)]
pub(crate) struct Exp {
    /// Condition for `If`
    val: ExpValue,
    /// Nodes if true
    nodes: Nodes,
}

/// If branch
///
/// # Values
///
/// * `exp: Vec<Exp>` - List of expression for `If`.
/// * `else_exp: Option<Nodes>` - Nodes if else.
#[derive(Debug, Clone)]
pub(crate) struct If {
    /// List of expression for `If`
    exp: Vec<Exp>,
    /// Nodes if else
    else_exp: Option<Nodes>,
}

/// Echo value branch
///
/// # Values
///
/// * `val: Value` - Value.
/// * `begin: bool` - Trim text in front.
/// * `end: bool` - Trim the text at the back.
#[derive(Debug, Clone)]
pub(crate) struct EchoValue {
    /// Value
    val: Value,
    /// Trim text in front
    begin: bool,
    /// Trim the text at the back
    end: bool,
}

/// Describes a Node of template.
///
/// # Values
///
/// * `Text(String)` - Simple text.
/// * `Value(EchoValue)` - Echo value.
/// * `For(For)` - For value.
/// * `IF(If)` - If value.
#[derive(Debug, Clone)]
pub(crate) enum Node {
    /// Simple text
    Text(String),
    /// Echo value
    Value(EchoValue),
    /// For value
    For(For),
    /// If value
    IF(If),
}

/// Conditions in template
#[derive(Debug, Clone, PartialEq)]
enum ItemCondition {
    /// None conditions
    None,
    /// Simple text
    Text,
    /// If condition
    If,
    /// ElseIf condition
    ElseIf,
    /// Else condition
    Else,
    /// EndIf condition
    EndIf,
    /// For condition
    For,
    /// ElseFor condition
    ElseFor,
    /// EndFor condition
    EndFor,
}

/// Item of condition for parsing tmplate
#[derive(Debug, Clone)]
struct Item {
    /// Begin position of condition
    begin: usize,
    /// Begin position of condition
    end: usize,
    /// Content of condition
    text: String,
    /// Level of condition
    level: usize,
    /// Conditions in template
    cond: ItemCondition,
    /// Childen list of item
    child: Vec<Item>,
}

/// Template nodes
pub(crate) type Nodes = Vec<Node>;

#[cfg(feature = "html-reload")]
static WRLOCK: OnceCell<WrLock> = OnceCell::const_new();

/// Html template marker
#[derive(Debug)]
pub(crate) struct Html {
    /// List of templates
    ///
    /// # Index
    ///
    /// * 1 - Module ID
    /// * 2 - Class ID
    /// * 3 - Template ID
    /// * 4 - List of Nodes
    #[allow(clippy::type_complexity)]
    pub list: HashMap<i64, HashMap<i64, Arc<HashMap<i64, Nodes>>>>,
    /// SystemTime last modification
    #[cfg(feature = "html-reload")]
    pub(crate) last: SystemTime,
    /// Sum of all filename hashes
    #[cfg(feature = "html-reload")]
    hash: i128,
    /// Path to templates' files
    root: Arc<PathBuf>,
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
    /// @ - If it is at the beginning, then do not change the expression after it, but simply remove this symbol
    /// {{- trim left
    /// -}} trim right
    /// {{ name }} htmlspecialchar
    /// {{ name.title }} htmlspecialchar
    /// {{ name.title.title_ua }} htmlspecialchar
    /// {{ name|raw }}
    /// {{ name|dump }}
    /// {# comment #}
    ///
    /// {% if bool %}
    /// {% if bool|len %}
    /// {% if bool|set %}
    /// {% if bool|unset %}
    /// {% if int > 0 %} > >= < <= = !=
    /// {% elseif ... %}
    /// {% else ... %}
    /// {% endif ... %}
    ///
    /// {% for arr in array %}
    ///   {{ arr|idx }}
    ///   {{ arr.title|raw }}
    ///   {{ arr.body }}
    /// {% elsefor %} empty or null array
    /// {% endfor %}
    pub async fn new(root: Arc<PathBuf>) -> Result<Html, ()> {
        #[cfg(feature = "html-reload")]
        if WRLOCK.set(WrLock::new()).is_err() {
            return Err(());
        }

        #[cfg(feature = "html-reload")]
        let last_time = SystemTime::UNIX_EPOCH;

        let mut root = root.as_ref().clone();
        root.push("app");

        let mut html = Html {
            list: HashMap::new(),
            #[cfg(feature = "html-reload")]
            last: last_time,
            #[cfg(feature = "html-reload")]
            hash: 0,
            root,
        };
        html.load().await;
        Ok(html)
    }

    /// Gets temptale from String
    fn parse(orig: &str) -> Result<Nodes, String> {
        if orig.is_empty() {
            return Ok(Vec::new());
        }
        let mut shift = 1;
        let mut last = 1;

        let mut html = String::with_capacity(orig.len() + 4);
        let mut len = orig.len();
        html.push('_');
        html.push_str(orig);
        html.push_str("___");

        let mut current;
        let mut prev;

        let mut result = String::with_capacity(len);

        // remove comment
        let mut ignore = None;
        while shift < len + 1 {
            // Take 2 symbols
            current = unsafe { html.get_unchecked(shift..shift + 2) };
            // Take prev symbol
            prev = unsafe { html.get_unchecked(shift - 1..shift) };
            match current {
                "{#" => {
                    if prev == "@" {
                        ignore = Some(shift - 1);
                    } else {
                        result.push_str(&html[last..shift]);
                    }
                    shift += 2;
                }
                "#}" => {
                    if let Some(idx) = ignore {
                        result.push_str(&html[last..idx]);
                        result.push_str(&html[idx + 1..shift + 2]);
                        ignore = None;
                    }
                    shift += 2;
                    last = shift;
                }
                _ => shift += 1,
            }
        }
        result.push_str(&html[last..len + 1]);

        let mut html = String::with_capacity(result.len() + 4);
        len = result.len();
        html.push('_');
        html.push_str(&result);
        html.push_str("___");

        // read conditions
        shift = 1;
        last = 1;

        let mut order = false;
        let mut idx = 0;
        let mut vec = Vec::new();
        while shift < len + 1 {
            // Take 2 symbols
            current = unsafe { html.get_unchecked(shift..shift + 2) };
            // Take prev symbol
            prev = unsafe { html.get_unchecked(shift - 1..shift) };
            match (current, order) {
                // Begin condition
                ("{%", false) => {
                    if prev == "@" {
                        ignore = Some(0);
                    } else {
                        idx = shift + 2;
                    }
                    shift += 2;
                    order = true;
                }
                // End condition
                ("%}", true) => {
                    if ignore.is_some() {
                        ignore = None;
                    } else {
                        vec.push(Item {
                            begin: last - 1,
                            end: idx - 3,
                            text: html[last..idx - 2].to_owned(),
                            level: 0,
                            cond: ItemCondition::Text,
                            child: Vec::new(),
                        });
                        vec.push(Item {
                            begin: idx - 1,
                            end: shift - 1,
                            text: html[idx..shift].trim().to_owned(),
                            level: 0,
                            cond: ItemCondition::None,
                            child: Vec::new(),
                        });
                        last = shift + 2;
                    }
                    shift += 2;
                    order = false;
                }
                ("{%", true) | ("%}", false) => {
                    return Err(format!(r#"Mismatched parentheses in "{}""#, Html::get_err_msg(shift, shift, &html)));
                }
                _ => shift += 1,
            }
        }
        // Add closing text
        vec.push(Item {
            begin: last - 1,
            end: len,
            text: html[last..len + 1].to_owned(),
            level: 0,
            cond: ItemCondition::Text,
            child: Vec::new(),
        });

        // check tree conditions
        let mut level = 0;
        for i in vec.as_mut_slice() {
            if i.cond != ItemCondition::Text {
                idx = match i.text.find(' ') {
                    Some(idx) => idx,
                    None => i.text.len(),
                };
                match &i.text[..idx] {
                    "if" => {
                        level += 1;
                        i.level = level;
                        i.cond = ItemCondition::If;
                        if idx == i.text.len() {
                            return Err(format!(
                                r#"The expression has an incorrect format in "{}""#,
                                Html::get_err_msg(i.begin, i.end, &html)
                            ));
                        }
                        i.text = i.text[idx + 1..].to_string();
                    }
                    "elseif" => {
                        i.level = level;
                        i.cond = ItemCondition::ElseIf;
                        if idx == i.text.len() {
                            return Err(format!(
                                r#"The expression has an incorrect format in "{}""#,
                                Html::get_err_msg(i.begin, i.end, &html)
                            ));
                        }
                        i.text = i.text[idx + 1..].to_string();
                    }
                    "else" => {
                        i.level = level;
                        i.cond = ItemCondition::Else;
                        if idx != i.text.len() {
                            return Err(format!(
                                r#"The expression has an incorrect format in "{}""#,
                                Html::get_err_msg(i.begin, i.end, &html)
                            ));
                        }
                        i.text = String::new();
                    }
                    "endif" => {
                        i.level = level;
                        level -= 1;
                        i.cond = ItemCondition::EndIf;
                        if idx != i.text.len() {
                            return Err(format!(
                                r#"The expression has an incorrect format in "{}""#,
                                Html::get_err_msg(i.begin, i.end, &html)
                            ));
                        }
                        i.text = String::new();
                    }
                    "for" => {
                        level += 1;
                        i.level = level;
                        i.cond = ItemCondition::For;
                        if idx == i.text.len() {
                            return Err(format!(
                                r#"The expression has an incorrect format in "{}""#,
                                Html::get_err_msg(i.begin, i.end, &html)
                            ));
                        }
                        i.text = i.text[idx + 1..].to_string();
                    }
                    "elsefor" => {
                        i.level = level;
                        i.cond = ItemCondition::ElseFor;
                        if idx != i.text.len() {
                            return Err(format!(
                                r#"The expression has an incorrect format in "{}""#,
                                Html::get_err_msg(i.begin, i.end, &html)
                            ));
                        }
                        i.text = String::new();
                    }
                    "endfor" => {
                        i.level = level;
                        level -= 1;
                        i.cond = ItemCondition::EndFor;
                        if idx != i.text.len() {
                            return Err(format!(
                                r#"The expression has an incorrect format in "{}""#,
                                Html::get_err_msg(i.begin, i.end, &html)
                            ));
                        }
                        i.text = String::new();
                    }
                    _ => {
                        return Err(format!(r#"Unrecognized operator in "{}""#, Html::get_err_msg(i.begin, i.end, &html)));
                    }
                }
            } else {
                i.level = level + 1;
            }
        }
        if level != 0 {
            return Err("No closing tag found".to_owned());
        }
        // build tree
        let vec = match Html::build_tree(&vec, &mut 0, 1) {
            Some(v) => v,
            None => return Err("The nesting structure does not match".to_owned()),
        };
        // create template
        let mut vec = match Html::build_vec(&vec, &html) {
            Ok(vec) => vec,
            Err(e) => return Err(format!("An error occurred while creating the template nodes: {}", e)),
        };
        // Clear templates
        let mut remove = false;
        if let Node::Text(node) = unsafe { vec.get_unchecked_mut(0) } {
            if node.is_empty() {
                remove = true;
            }
        }
        if remove {
            vec.remove(0);
            remove = false;
        }
        let len = vec.len() - 1;
        if let Node::Text(node) = unsafe { vec.get_unchecked_mut(len) } {
            if node.is_empty() {
                remove = true;
            }
        }
        if remove {
            vec.pop();
        }
        Ok(vec)
    }

    /// Check name of value A-Za-z0-9_|
    fn is_valid_input(val: &str) -> bool {
        if val.is_empty() {
            return false;
        }
        let val = val.as_bytes();
        for c in val {
            match *c {
                46 | 48..=57 | 65..=90 | 95 | 97..=122 => {}
                _ => return false,
            }
        }
        !(48..=57).contains(unsafe { val.get_unchecked(0) })
    }

    /// Check name of value A-Za-z0-9_
    fn is_simple_name(val: &str) -> bool {
        if val.is_empty() {
            return false;
        }
        let val = val.as_bytes();
        for c in val {
            match *c {
                48..=57 | 65..=90 | 95 | 97..=122 => {}
                _ => return false,
            }
        }
        !(48..=57).contains(unsafe { val.get_unchecked(0) })
    }

    /// Check name of value 0-9
    fn is_number_input(val: &str) -> bool {
        if val.is_empty() {
            return false;
        }
        let val = val.as_bytes();
        for c in val {
            match *c {
                48..=57 => {}
                _ => return false,
            }
        }
        true
    }

    /// Get Value fron text
    fn get_val(val: &str, exp: Option<bool>) -> Option<Value> {
        if !val.is_empty() {
            match val.find('|') {
                Some(idx) => {
                    let v = &val[0..idx];
                    if Html::is_valid_input(v) {
                        let vl: Vec<&str> = v.split('.').collect();
                        match &val[idx + 1..] {
                            "len" => Some(Value::Value {
                                name: vl.iter().map(|s| s.to_string()).collect(),
                                filter: Filter::Len,
                            }),
                            "set" => Some(Value::Value {
                                name: vl.iter().map(|s| s.to_string()).collect(),
                                filter: Filter::Set,
                            }),
                            "unset" => Some(Value::Value {
                                name: vl.iter().map(|s| s.to_string()).collect(),
                                filter: Filter::Unset,
                            }),
                            "raw" => {
                                if exp.is_none() {
                                    Some(Value::Value {
                                        name: vl.iter().map(|s| s.to_string()).collect(),
                                        filter: Filter::Raw,
                                    })
                                } else {
                                    None
                                }
                            }
                            "dump" => {
                                if exp.is_none() {
                                    Some(Value::Value {
                                        name: vl.iter().map(|s| s.to_string()).collect(),
                                        filter: Filter::Dump,
                                    })
                                } else {
                                    None
                                }
                            }
                            "key" => {
                                if exp.is_none() {
                                    Some(Value::Value {
                                        name: vl.iter().map(|s| s.to_string()).collect(),
                                        filter: Filter::Index,
                                    })
                                } else {
                                    None
                                }
                            }
                            _ => None,
                        }
                    } else {
                        None
                    }
                }
                None => {
                    if exp.unwrap_or(true) {
                        if Html::is_valid_input(val) {
                            let vl: Vec<&str> = val.split('.').collect();
                            Some(Value::Value {
                                name: vl.iter().map(|s| s.to_string()).collect(),
                                filter: Filter::None,
                            })
                        } else {
                            None
                        }
                    } else if Html::is_number_input(val) {
                        Some(Value::Number(val.parse().ok()?))
                    } else if Html::is_valid_input(val) {
                        let vl: Vec<&str> = val.split('.').collect();
                        Some(Value::Value {
                            name: vl.iter().map(|s| s.to_string()).collect(),
                            filter: Filter::None,
                        })
                    } else {
                        None
                    }
                }
            }
        } else {
            None
        }
    }

    /// Get condition for `If`
    fn get_exp(text: &str) -> Option<ExpValue> {
        // Split name
        let res = text.split_whitespace().collect::<Vec<&str>>();
        if res.len() == 1 {
            // One value
            let val = unsafe { res.get_unchecked(0).to_string() };
            Some(ExpValue {
                val: Html::get_val(&val, Some(true))?,
                eq: Eq::None,
                other: None,
            })
        } else if res.len() == 3 {
            // Two value
            let val = unsafe { res.get_unchecked(0).to_string() };
            let val = Html::get_val(&val, Some(true))?;
            let eq = match unsafe { *res.get_unchecked(1) } {
                ">" => Eq::GreaterThan,
                ">=" => Eq::GreaterThanOrEqual,
                "<" => Eq::LessThan,
                "<=" => Eq::LessThanOrEqual,
                "=" => Eq::Equal,
                "==" => Eq::Equal,
                "!=" => Eq::NotEqual,
                _ => return None,
            };
            let other = unsafe { res.get_unchecked(2).to_string() };
            let other = Some(Html::get_val(&other, Some(false))?);
            Some(ExpValue { val, eq, other })
        } else {
            None
        }
    }

    /// Parse text for searching `echo` conditions
    fn get_echo(orig: &str) -> Result<Nodes, usize> {
        let len = orig.len();
        let mut val = String::with_capacity(len + 4);
        val.push('_');
        val.push_str(orig);
        val.push_str("___");

        let mut shift = 1;
        let mut last = 1;
        let mut current;
        let mut prev;
        let mut idx = 0;

        let mut ignore = false;
        let mut ignore_idx = 0;
        let mut result = Vec::new();
        let mut order = false;
        let mut vl;
        let mut trim_begin = false;
        let mut trim_end = false;
        let mut val = val.to_owned();
        while shift < len + 1 {
            // Get the initial condition and ignore symbol
            current = unsafe { val.get_unchecked(shift..shift + 2) };
            prev = unsafe { val.get_unchecked(shift - 1..shift) };
            match (current, order) {
                ("{{", false) => {
                    if prev == "@" {
                        ignore = true;
                        ignore_idx = shift - 1;
                    } else {
                        idx = shift + 2;
                    }
                    shift += 2;
                    order = true;
                }
                ("}}", true) => {
                    if ignore {
                        val.remove(ignore_idx);
                        ignore = false;
                    } else {
                        result.push(Node::Text(val[last..idx - 2].to_owned()));
                        vl = val[idx..shift].trim().to_owned();
                        if vl.is_empty() {
                            return Err(shift);
                        }
                        // Check begin trim
                        if unsafe { vl.get_unchecked(..1) } == "-" {
                            trim_begin = true;
                            vl = vl[1..].trim().to_owned();
                            if vl.is_empty() {
                                return Err(shift);
                            }
                        }
                        // Check end trim
                        if unsafe { vl.get_unchecked(vl.len() - 1..) } == "-" {
                            trim_end = true;
                            vl = vl[..vl.len() - 1].trim().to_owned();
                            if vl.is_empty() {
                                return Err(shift);
                            }
                        }
                        // Save begin/end trim
                        match (trim_begin, trim_end) {
                            (true, true) => {
                                result.push(Node::Value(EchoValue {
                                    val: Html::get_val(&vl, None).ok_or(shift)?,
                                    begin: true,
                                    end: true,
                                }));
                                trim_begin = false;
                                trim_end = false;
                            }
                            (true, false) => {
                                result.push(Node::Value(EchoValue {
                                    val: Html::get_val(&vl, None).ok_or(shift)?,
                                    begin: true,
                                    end: false,
                                }));
                                trim_begin = false;
                            }
                            (false, true) => {
                                result.push(Node::Value(EchoValue {
                                    val: Html::get_val(&vl, None).ok_or(shift)?,
                                    begin: false,
                                    end: true,
                                }));
                                trim_end = false;
                            }
                            (false, false) => result.push(Node::Value(EchoValue {
                                val: Html::get_val(&vl, None).ok_or(shift)?,
                                begin: false,
                                end: false,
                            })),
                        }
                        last = shift + 2;
                    }
                    shift += 2;
                    order = false;
                }
                ("{{", true) => return Err(shift),
                _ => shift += 1,
            }
        }
        result.push(Node::Text(val[last..len + 1].to_owned()));
        Ok(result)
    }

    /// Check if expressions
    fn get_if_exp(val: &ExpValue, data: &HashMap<i64, Data>, tmp: &HashMap<i64, Data>) -> bool {
        match &val.other {
            Some(d) => {
                if val.eq != Eq::None {
                    let first = match Html::get_if_data(&val.val, data, tmp) {
                        Some(d) => d,
                        None => return false,
                    };
                    let second = match Html::get_if_data(d, data, tmp) {
                        Some(d) => d,
                        None => return false,
                    };
                    Html::compare_value(&first, &second, &val.eq)
                } else {
                    false
                }
            }
            None => match Html::get_if_data(&val.val, data, tmp) {
                Some(Data::Bool(b)) => b,
                _ => false,
            },
        }
    }

    /// Buld branch tree from list
    fn build_vec(vec: &Vec<Item>, html: &str) -> Result<Nodes, String> {
        let mut is_if = false;
        let mut is_if_else = false;
        let mut is_for = false;
        let mut is_for_else = false;
        let mut nodes = Vec::new();
        let mut if_list = Vec::new();
        let mut if_else = None;
        let mut for_list = Vec::new();
        let mut for_else = None;
        let mut for_local = String::new();
        let mut for_name = None;
        let mut sub_nodes;
        for item in vec {
            match item.cond {
                ItemCondition::None => {
                    return Err(format!(r#"Incorrect tag None in "{}""#, Html::get_err_msg(item.begin, item.end, html)));
                }
                ItemCondition::Text => match Html::get_echo(&item.text) {
                    Ok(ns) => {
                        for n in ns {
                            nodes.push(n);
                        }
                    }
                    Err(shift) => {
                        let start = if shift < 25 { 0 } else { shift - 25 };
                        let finish = if shift + 25 > item.text.len() - 3 { item.text.len() - 3 } else { shift + 25 };
                        return Err(format!(
                            r#"Incorrect echo "{}" in "{}""#,
                            &item.text[start..finish],
                            Html::get_err_msg(item.begin, item.end, html)
                        ));
                    }
                },
                ItemCondition::If => {
                    if !(is_if || is_if_else || is_for || is_for_else) {
                        is_if = true;
                        let exp = match Html::get_exp(&item.text) {
                            Some(e) => e,
                            None => {
                                return Err(format!(r#"Incorrect expression in "{}""#, Html::get_err_msg(item.begin, item.end, html)));
                            }
                        };
                        sub_nodes = Html::build_vec(&item.child, html)?;
                        if_list.push(Exp { val: exp, nodes: sub_nodes });
                    } else {
                        return Err(format!(r#"Incorrect identical 'IF' tags in "{}""#, Html::get_err_msg(item.begin, item.end, html)));
                    }
                }
                ItemCondition::ElseIf => {
                    if is_if && !is_if_else && !is_for && !is_for_else {
                        let exp = match Html::get_exp(&item.text) {
                            Some(e) => e,
                            None => {
                                return Err(format!(r#"Incorrect expression in "{}""#, Html::get_err_msg(item.begin, item.end, html)));
                            }
                        };
                        sub_nodes = Html::build_vec(&item.child, html)?;
                        if_list.push(Exp { val: exp, nodes: sub_nodes });
                    } else {
                        return Err(format!(r#"Incorrect identical 'ElseIf' tag in "{}""#, Html::get_err_msg(item.begin, item.end, html)));
                    }
                }
                ItemCondition::Else => {
                    if is_if && !is_for && !is_for_else {
                        is_if_else = true;
                        if_else = Some(Html::build_vec(&item.child, html)?);
                    } else {
                        return Err(format!(r#"Incorrect identical 'Else' tag in "{}""#, Html::get_err_msg(item.begin, item.end, html)));
                    }
                }
                ItemCondition::EndIf => {
                    if is_if && !is_for && !is_for_else {
                        is_if = false;
                        is_if_else = false;
                        if if_list.is_empty() {
                            return Err(format!(r#"Incorrect 'IF' tag format in "{}""#, Html::get_err_msg(item.begin, item.end, html)));
                        }
                        nodes.push(Node::IF(If {
                            exp: if_list.clone(),
                            else_exp: if_else,
                        }));
                        if_else = None;
                        if_list.clear();
                    } else {
                        return Err(format!(r#"Incorrect identical 'EndIf' tag in "{}""#, Html::get_err_msg(item.begin, item.end, html)));
                    }
                }
                ItemCondition::For => {
                    if !(is_if || is_if_else || is_for || is_for_else) {
                        is_for = true;
                        let vl: Vec<&str> = item.text.split_whitespace().collect();
                        if vl.len() != 3 {
                            return Err(format!(r#"Incorrect identical 'For' tag in "{}""#, Html::get_err_msg(item.begin, item.end, html)));
                        }
                        if unsafe { *vl.get_unchecked(1) } != "in" {
                            return Err(format!(r#"Incorrect identical 'For' tag in "{}""#, Html::get_err_msg(item.begin, item.end, html)));
                        }
                        for_name = match Html::get_val(unsafe { vl.get_unchecked(2) }, Some(true)) {
                            Some(n) => Some(n),
                            None => {
                                return Err(format!(
                                    r#"Incorrect identical 'For' tag in "{}""#,
                                    Html::get_err_msg(item.begin, item.end, html)
                                ));
                            }
                        };
                        for_local = unsafe { *vl.get_unchecked(0) }.to_string();
                        if !Html::is_simple_name(&for_local) {
                            return Err(format!(r#"Incorrect identical 'For' tag in "{}""#, Html::get_err_msg(item.begin, item.end, html)));
                        }
                        for_list = Html::build_vec(&item.child, html)?;
                    } else {
                        return Err(format!(r#"Incorrect identical 'For' tag in "{}""#, Html::get_err_msg(item.begin, item.end, html)));
                    }
                }
                ItemCondition::ElseFor => {
                    if is_for && !is_if && !is_if_else && !is_for_else {
                        is_for_else = true;
                        for_else = Some(Html::build_vec(&item.child, html)?);
                    } else {
                        return Err(format!(r#"Incorrect identical 'ElseFor' tag in "{}""#, Html::get_err_msg(item.begin, item.end, html)));
                    }
                }
                ItemCondition::EndFor => {
                    if is_for && !is_if && !is_if_else {
                        is_for = false;
                        is_for_else = false;
                        if for_list.is_empty() {
                            return Err(format!(r#"Incorrect 'For' tag format in "{}""#, Html::get_err_msg(item.begin, item.end, html)));
                        }
                        if let Some(name) = for_name {
                            nodes.push(Node::For(For {
                                name,
                                local: for_local.clone(),
                                nodes: for_list.clone(),
                                empty: for_else,
                            }));
                        } else {
                            return Err(format!(r#"Incorrect 'For' tag format in "{}""#, Html::get_err_msg(item.begin, item.end, html)));
                        }
                        for_local = String::new();
                        for_name = None;
                        for_else = None;
                        for_list.clear();
                    } else {
                        return Err(format!(r#"Incorrect identical 'EndFor' tag in "{}""#, Html::get_err_msg(item.begin, item.end, html)));
                    }
                }
            }
        }
        nodes.shrink_to_fit();
        Ok(nodes)
    }

    /// Prepares an error message
    fn get_err_msg(begin: usize, end: usize, html: &str) -> String {
        let html = &html[1..html.len() - 3];
        let end = if end > html.len() { html.len() } else { end };
        let chars_start: Vec<char> = html[0..begin].chars().collect();
        let chars_middle: Vec<char> = html[begin..end].chars().collect();
        let chars_finish: Vec<char> = html[end..html.len()].chars().collect();
        let start = if chars_start.len() < 25 { 0 } else { chars_start.len() - 25 };
        let finish = if chars_finish.len() < 25 { chars_finish.len() } else { 25 };
        let mut msg = chars_start[start..].iter().collect::<String>();
        msg.push_str(&chars_middle[..].iter().collect::<String>());
        msg.push_str(&chars_finish[..finish].iter().collect::<String>());
        msg
    }

    /// Set level for list of conditions
    fn build_tree(vec: &Vec<Item>, shift: &mut usize, level: usize) -> Option<Vec<Item>> {
        let mut item;
        let mut res = Vec::new();
        let mut last;
        while *shift < vec.len() {
            item = unsafe { vec.get_unchecked(*shift) };
            if item.level < level {
                return Some(res);
            } else if item.level == level {
                res.push(item.clone());
                *shift += 1;
            } else if item.level == level + 1 {
                last = res.pop()?;
                last.child = Html::build_tree(vec, shift, level + 1)?;
                res.push(last);
            }
        }
        Some(res)
    }

    /// Render of html template
    pub fn render<'a>(data: &'a HashMap<i64, Data>, list: &'a Nodes) -> Answer {
        let mut tmp = HashMap::new();
        Answer::String(Html::render_level(list, data, &mut tmp))
    }

    /// Render one level of template
    fn render_level(list: &Nodes, data: &HashMap<i64, Data>, tmp: &mut HashMap<i64, Data>) -> String {
        let mut html = String::new();
        let mut trim_end = false;
        for item in list {
            match item {
                Node::Text(s) => {
                    if trim_end {
                        let t = html.trim_end().len();
                        if t < html.len() {
                            unsafe {
                                html.as_mut_vec().truncate(t);
                            }
                        }
                        trim_end = false;
                        html.push_str(s.trim_start());
                    } else {
                        html.push_str(s)
                    }
                }
                Node::Value(v) => {
                    if trim_end {
                        let t = html.trim_end().len();
                        if t < html.len() {
                            unsafe {
                                html.as_mut_vec().truncate(t);
                            }
                        }
                        trim_end = false;
                    }
                    if v.begin {
                        let t = html.trim_end().len();
                        if t < html.len() {
                            unsafe {
                                html.as_mut_vec().truncate(t);
                            }
                        }
                        html.push_str(Html::print_echo(&v.val, data, tmp).trim_start())
                    } else {
                        html.push_str(&Html::print_echo(&v.val, data, tmp))
                    }
                    if v.end {
                        trim_end = true;
                    }
                }
                Node::For(f) => {
                    if trim_end {
                        let t = html.trim_end().len();
                        if t < html.len() {
                            unsafe {
                                html.as_mut_vec().truncate(t);
                            }
                        }
                        trim_end = false;
                    }
                    match Html::get_for_data(&f.name, data, tmp) {
                        Some(d) => match d {
                            Data::Vec(vec) => {
                                if !vec.is_empty() {
                                    let key_idx = fnv1a_64(format!("{}|key", f.local).as_bytes());
                                    let key = fnv1a_64(f.local.as_bytes());
                                    for (idx, v) in vec.into_iter().enumerate() {
                                        tmp.insert(key_idx, Data::Usize(idx + 1));
                                        tmp.insert(key, v.clone());
                                        html.push_str(&Html::render_level(&f.nodes, data, tmp));
                                    }
                                    tmp.remove(&key_idx);
                                    tmp.remove(&key);
                                }
                            }
                            Data::Map(map) => {
                                if !map.is_empty() {
                                    let key_idx = fnv1a_64(format!("{}|key", f.local).as_bytes());
                                    let key = fnv1a_64(f.local.as_bytes());
                                    for (key, v) in map {
                                        tmp.insert(key_idx, Data::I64(key));
                                        tmp.insert(key, v.clone());
                                        html.push_str(&Html::render_level(&f.nodes, data, tmp));
                                    }
                                    tmp.remove(&key_idx);
                                    tmp.remove(&key);
                                }
                            }
                            _ => {}
                        },
                        None => {
                            if let Some(v) = &f.empty {
                                html.push_str(&Html::render_level(v, data, tmp));
                            }
                        }
                    }
                }
                Node::IF(i) => {
                    if trim_end {
                        let t = html.trim_end().len();
                        if t < html.len() {
                            unsafe {
                                html.as_mut_vec().truncate(t);
                            }
                        }
                        trim_end = false;
                    }
                    let mut run = false;
                    for item in &i.exp {
                        if Html::get_if_exp(&item.val, data, tmp) {
                            html.push_str(&Html::render_level(&item.nodes, data, tmp));
                            run = true;
                            break;
                        }
                    }
                    if !run {
                        if let Some(n) = &i.else_exp {
                            html.push_str(&Html::render_level(n, data, tmp));
                        }
                    }
                }
            }
        }
        if trim_end {
            let t = html.trim_end().len();
            if t < html.len() {
                unsafe {
                    html.as_mut_vec().truncate(t);
                }
            }
        }
        html
    }

    #[inline]
    fn compare<T: PartialOrd>(a: T, b: T, eq: &Eq) -> bool {
        match eq {
            Eq::Equal => a == b,
            Eq::NotEqual => a != b,
            Eq::LessThan => a < b,
            Eq::LessThanOrEqual => a <= b,
            Eq::GreaterThan => a > b,
            Eq::GreaterThanOrEqual => a >= b,
            Eq::None => false,
        }
    }

    /// Compare values for if condition
    fn compare_value(first: &Data, second: &Data, eq: &Eq) -> bool {
        match (first, second) {
            (Data::Usize(a), Data::U8(b)) => Html::compare(*a, *b as usize, eq),
            (Data::Usize(a), Data::U16(b)) => Html::compare(*a, *b as usize, eq),
            (Data::Usize(a), Data::U32(b)) => Html::compare(*a, *b as usize, eq),
            (Data::Usize(a), Data::U64(b)) => Html::compare(*a, *b as usize, eq),
            (Data::Usize(a), Data::Usize(b)) => Html::compare(*a, *b, eq),
            (Data::Usize(a), Data::I8(b)) => usize::try_from(*b).map_or(false, |b| Html::compare(*a, b, eq)),
            (Data::Usize(a), Data::I16(b)) => usize::try_from(*b).map_or(false, |b| Html::compare(*a, b, eq)),
            (Data::Usize(a), Data::I32(b)) => usize::try_from(*b).map_or(false, |b| Html::compare(*a, b, eq)),
            (Data::Usize(a), Data::I64(b)) => usize::try_from(*b).map_or(false, |b| Html::compare(*a, b, eq)),
            (Data::Usize(a), Data::F32(b)) => {
                if *b > 0.0 && b.fract() == 0.0 {
                    Html::compare(*a, *b as usize, eq)
                } else {
                    false
                }
            }
            (Data::Usize(a), Data::F64(b)) => {
                if *b > 0.0 && b.fract() == 0.0 {
                    Html::compare(*a, *b as usize, eq)
                } else {
                    false
                }
            }

            (Data::U8(a), Data::U8(b)) => Html::compare(*a, *b, eq),
            (Data::U8(a), Data::U16(b)) => Html::compare(*a as u16, *b, eq),
            (Data::U8(a), Data::U32(b)) => Html::compare(*a as u32, *b, eq),
            (Data::U8(a), Data::U64(b)) => Html::compare(*a as u64, *b, eq),
            (Data::U8(a), Data::Usize(b)) => Html::compare(*a as usize, *b, eq),
            (Data::U8(a), Data::I8(b)) => u8::try_from(*b).map_or(false, |b| Html::compare(*a, b, eq)),
            (Data::U8(a), Data::I16(b)) => Html::compare(*a as i16, *b, eq),
            (Data::U8(a), Data::I32(b)) => Html::compare(*a as i32, *b, eq),
            (Data::U8(a), Data::I64(b)) => Html::compare(*a as i64, *b, eq),
            (Data::U8(a), Data::F32(b)) => Html::compare(*a as f32, *b, eq),
            (Data::U8(a), Data::F64(b)) => Html::compare(*a as f64, *b, eq),

            (Data::U16(a), Data::U8(b)) => Html::compare(*a, *b as u16, eq),
            (Data::U16(a), Data::U16(b)) => Html::compare(*a, *b, eq),
            (Data::U16(a), Data::U32(b)) => Html::compare(*a as u32, *b, eq),
            (Data::U16(a), Data::U64(b)) => Html::compare(*a as u64, *b, eq),
            (Data::U16(a), Data::Usize(b)) => Html::compare(*a as usize, *b, eq),
            (Data::U16(a), Data::I8(b)) => u16::try_from(*b).map_or(false, |b| Html::compare(*a, b, eq)),
            (Data::U16(a), Data::I16(b)) => u16::try_from(*b).map_or(false, |b| Html::compare(*a, b, eq)),
            (Data::U16(a), Data::I32(b)) => Html::compare(*a as i32, *b, eq),
            (Data::U16(a), Data::I64(b)) => Html::compare(*a as i64, *b, eq),
            (Data::U16(a), Data::F32(b)) => Html::compare(*a as f32, *b, eq),
            (Data::U16(a), Data::F64(b)) => Html::compare(*a as f64, *b, eq),

            (Data::U32(a), Data::U8(b)) => Html::compare(*a, *b as u32, eq),
            (Data::U32(a), Data::U16(b)) => Html::compare(*a, *b as u32, eq),
            (Data::U32(a), Data::U32(b)) => Html::compare(*a, *b, eq),
            (Data::U32(a), Data::U64(b)) => Html::compare(*a as u64, *b, eq),
            (Data::U32(a), Data::Usize(b)) => Html::compare(*a as usize, *b, eq),
            (Data::U32(a), Data::I8(b)) => u32::try_from(*b).map_or(false, |b| Html::compare(*a, b, eq)),
            (Data::U32(a), Data::I16(b)) => u32::try_from(*b).map_or(false, |b| Html::compare(*a, b, eq)),
            (Data::U32(a), Data::I32(b)) => u32::try_from(*b).map_or(false, |b| Html::compare(*a, b, eq)),
            (Data::U32(a), Data::I64(b)) => Html::compare(*a as i64, *b, eq),
            (Data::U32(a), Data::F32(b)) => {
                if *b > 0.0 && b.fract() == 0.0 {
                    Html::compare(*a, *b as u32, eq)
                } else {
                    false
                }
            }
            (Data::U32(a), Data::F64(b)) => Html::compare(*a as f64, *b, eq),

            (Data::U64(a), Data::U8(b)) => Html::compare(*a, *b as u64, eq),
            (Data::U64(a), Data::U16(b)) => Html::compare(*a, *b as u64, eq),
            (Data::U64(a), Data::U32(b)) => Html::compare(*a, *b as u64, eq),
            (Data::U64(a), Data::U64(b)) => Html::compare(*a, *b, eq),
            (Data::U64(a), Data::Usize(b)) => Html::compare(*a as usize, *b, eq),
            (Data::U64(a), Data::I8(b)) => u64::try_from(*b).map_or(false, |b| Html::compare(*a, b, eq)),
            (Data::U64(a), Data::I16(b)) => u64::try_from(*b).map_or(false, |b| Html::compare(*a, b, eq)),
            (Data::U64(a), Data::I32(b)) => u64::try_from(*b).map_or(false, |b| Html::compare(*a, b, eq)),
            (Data::U64(a), Data::I64(b)) => u64::try_from(*b).map_or(false, |b| Html::compare(*a, b, eq)),
            (Data::U64(a), Data::F32(b)) => {
                if *b > 0.0 && b.fract() == 0.0 {
                    Html::compare(*a, *b as u64, eq)
                } else {
                    false
                }
            }
            (Data::U64(a), Data::F64(b)) => {
                if *b > 0.0 && b.fract() == 0.0 {
                    Html::compare(*a, *b as u64, eq)
                } else {
                    false
                }
            }

            (Data::I8(a), Data::U8(b)) => i8::try_from(*b).map_or(false, |b| Html::compare(*a, b, eq)),
            (Data::I8(a), Data::U16(b)) => i8::try_from(*b).map_or(false, |b| Html::compare(*a, b, eq)),
            (Data::I8(a), Data::U32(b)) => i8::try_from(*b).map_or(false, |b| Html::compare(*a, b, eq)),
            (Data::I8(a), Data::U64(b)) => i8::try_from(*b).map_or(false, |b| Html::compare(*a, b, eq)),
            (Data::I8(a), Data::Usize(b)) => i8::try_from(*b).map_or(false, |b| Html::compare(*a, b, eq)),
            (Data::I8(a), Data::I8(b)) => Html::compare(*a, *b, eq),
            (Data::I8(a), Data::I16(b)) => Html::compare(*a as i16, *b, eq),
            (Data::I8(a), Data::I32(b)) => Html::compare(*a as i32, *b, eq),
            (Data::I8(a), Data::I64(b)) => Html::compare(*a as i64, *b, eq),
            (Data::I8(a), Data::F32(b)) => Html::compare(*a as f32, *b, eq),
            (Data::I8(a), Data::F64(b)) => Html::compare(*a as f64, *b, eq),

            (Data::I16(a), Data::U8(b)) => Html::compare(*a, *b as i16, eq),
            (Data::I16(a), Data::U16(b)) => i16::try_from(*b).map_or(false, |b| Html::compare(*a, b, eq)),
            (Data::I16(a), Data::U32(b)) => i16::try_from(*b).map_or(false, |b| Html::compare(*a, b, eq)),
            (Data::I16(a), Data::U64(b)) => i16::try_from(*b).map_or(false, |b| Html::compare(*a, b, eq)),
            (Data::I16(a), Data::Usize(b)) => i16::try_from(*b).map_or(false, |b| Html::compare(*a, b, eq)),
            (Data::I16(a), Data::I8(b)) => Html::compare(*a, *b as i16, eq),
            (Data::I16(a), Data::I16(b)) => Html::compare(*a, *b, eq),
            (Data::I16(a), Data::I32(b)) => Html::compare(*a as i32, *b, eq),
            (Data::I16(a), Data::I64(b)) => Html::compare(*a as i64, *b, eq),
            (Data::I16(a), Data::F32(b)) => Html::compare(*a as f32, *b, eq),
            (Data::I16(a), Data::F64(b)) => Html::compare(*a as f64, *b, eq),

            (Data::I32(a), Data::U8(b)) => Html::compare(*a, *b as i32, eq),
            (Data::I32(a), Data::U16(b)) => Html::compare(*a, *b as i32, eq),
            (Data::I32(a), Data::U32(b)) => i32::try_from(*b).map_or(false, |b| Html::compare(*a, b, eq)),
            (Data::I32(a), Data::U64(b)) => i32::try_from(*b).map_or(false, |b| Html::compare(*a, b, eq)),
            (Data::I32(a), Data::Usize(b)) => i32::try_from(*b).map_or(false, |b| Html::compare(*a, b, eq)),
            (Data::I32(a), Data::I8(b)) => Html::compare(*a, *b as i32, eq),
            (Data::I32(a), Data::I16(b)) => Html::compare(*a, *b as i32, eq),
            (Data::I32(a), Data::I32(b)) => Html::compare(*a, *b, eq),
            (Data::I32(a), Data::I64(b)) => Html::compare(*a as i64, *b, eq),
            (Data::I32(a), Data::F32(b)) => Html::compare(*a as f32, *b, eq),
            (Data::I32(a), Data::F64(b)) => Html::compare(*a as f64, *b, eq),

            (Data::I64(a), Data::U8(b)) => Html::compare(*a, *b as i64, eq),
            (Data::I64(a), Data::U16(b)) => Html::compare(*a, *b as i64, eq),
            (Data::I64(a), Data::U32(b)) => Html::compare(*a, *b as i64, eq),
            (Data::I64(a), Data::U64(b)) => i64::try_from(*b).map_or(false, |b| Html::compare(*a, b, eq)),
            (Data::I64(a), Data::Usize(b)) => i64::try_from(*b).map_or(false, |b| Html::compare(*a, b, eq)),
            (Data::I64(a), Data::I8(b)) => Html::compare(*a, *b as i64, eq),
            (Data::I64(a), Data::I16(b)) => Html::compare(*a, *b as i64, eq),
            (Data::I64(a), Data::I32(b)) => Html::compare(*a, *b as i64, eq),
            (Data::I64(a), Data::I64(b)) => Html::compare(*a, *b, eq),
            (Data::I64(a), Data::F32(b)) => {
                if b.fract() == 0.0 {
                    Html::compare(*a, *b as i64, eq)
                } else {
                    false
                }
            }
            (Data::I64(a), Data::F64(b)) => Html::compare(*a as f64, *b, eq),

            (Data::F32(a), Data::U8(b)) => Html::compare(*a, *b as f32, eq),
            (Data::F32(a), Data::U16(b)) => Html::compare(*a, *b as f32, eq),
            (Data::F32(a), Data::U32(b)) => {
                if a.fract() == 0.0 {
                    Html::compare(*a as u32, *b, eq)
                } else {
                    false
                }
            }
            (Data::F32(a), Data::U64(b)) => {
                if a.fract() == 0.0 {
                    Html::compare(*a as u64, *b, eq)
                } else {
                    false
                }
            }
            (Data::F32(a), Data::Usize(b)) => {
                if a.fract() == 0.0 {
                    Html::compare(*a as usize, *b, eq)
                } else {
                    false
                }
            }
            (Data::F32(a), Data::I8(b)) => Html::compare(*a, *b as f32, eq),
            (Data::F32(a), Data::I16(b)) => Html::compare(*a, *b as f32, eq),
            (Data::F32(a), Data::I32(b)) => Html::compare(*a, *b as f32, eq),
            (Data::F32(a), Data::I64(b)) => Html::compare(*a as i64, *b, eq),
            (Data::F32(a), Data::F32(b)) => Html::compare(*a, *b, eq),
            (Data::F32(a), Data::F64(b)) => Html::compare(*a as f64, *b, eq),

            (Data::F64(a), Data::U8(b)) => Html::compare(*a, *b as f64, eq),
            (Data::F64(a), Data::U16(b)) => Html::compare(*a, *b as f64, eq),
            (Data::F64(a), Data::U32(b)) => Html::compare(*a, *b as f64, eq),
            (Data::F64(a), Data::U64(b)) => {
                if a.fract() == 0.0 {
                    Html::compare(*a as u64, *b, eq)
                } else {
                    false
                }
            }
            (Data::F64(a), Data::Usize(b)) => {
                if a.fract() == 0.0 {
                    Html::compare(*a as usize, *b, eq)
                } else {
                    false
                }
            }
            (Data::F64(a), Data::I8(b)) => Html::compare(*a, *b as f64, eq),
            (Data::F64(a), Data::I16(b)) => Html::compare(*a, *b as f64, eq),
            (Data::F64(a), Data::I32(b)) => Html::compare(*a, *b as f64, eq),
            (Data::F64(a), Data::I64(b)) => Html::compare(*a, *b as f64, eq),
            (Data::F64(a), Data::F32(b)) => Html::compare(*a, *b as f64, eq),
            (Data::F64(a), Data::F64(b)) => Html::compare(*a, *b, eq),

            (Data::Bool(a), Data::Bool(b)) => Html::compare(*a, *b, eq),
            (Data::String(a), Data::String(b)) => Html::compare(a, b, eq),
            (Data::Date(a), Data::Date(b)) => Html::compare(*a, *b, eq),
            _ => false,
        }
    }

    /// Extract Data from a value for If condition
    fn get_if_data(val: &Value, data: &HashMap<i64, Data>, tmp: &HashMap<i64, Data>) -> Option<Data> {
        match val {
            Value::Number(n) => Some(Data::I64(*n)),
            Value::Value { name, filter } => {
                if !name.is_empty() {
                    match filter {
                        Filter::None => {
                            let mut key = fnv1a_64(unsafe { name.get_unchecked(0) }.as_bytes());
                            let mut val = match data.get(&key) {
                                Some(v) => v,
                                None => match tmp.get(&key) {
                                    Some(v) => v,
                                    None => return None,
                                },
                            };
                            let mut shift = 1;
                            while shift < name.len() {
                                if let Data::Map(map) = val {
                                    key = fnv1a_64(unsafe { name.get_unchecked(shift) }.as_bytes());
                                    val = match map.get(&key) {
                                        Some(v) => v,
                                        None => return None,
                                    };
                                } else {
                                    return None;
                                }
                                shift += 1;
                            }
                            Some(val.clone())
                        }
                        Filter::Len => {
                            let mut key = fnv1a_64(unsafe { name.get_unchecked(0) }.as_bytes());
                            let mut val = match data.get(&key) {
                                Some(v) => v,
                                None => match tmp.get(&key) {
                                    Some(v) => v,
                                    None => return None,
                                },
                            };
                            let mut shift = 1;
                            while shift < name.len() {
                                if let Data::Map(map) = val {
                                    key = fnv1a_64(unsafe { name.get_unchecked(shift) }.as_bytes());
                                    val = match map.get(&key) {
                                        Some(v) => v,
                                        None => return None,
                                    };
                                } else {
                                    return None;
                                }
                                shift += 1;
                            }
                            match val {
                                Data::String(s) => Some(Data::Usize(s.len())),
                                Data::Vec(v) => Some(Data::Usize(v.len())),
                                Data::Map(m) => Some(Data::Usize(m.len())),
                                _ => None,
                            }
                        }
                        Filter::Set => {
                            let mut key = fnv1a_64(unsafe { name.get_unchecked(0) }.as_bytes());
                            let mut val = match data.get(&key) {
                                Some(v) => v,
                                None => match tmp.get(&key) {
                                    Some(v) => v,
                                    None => return Some(Data::Bool(false)),
                                },
                            };
                            let mut shift = 1;
                            while shift < name.len() {
                                if let Data::Map(map) = val {
                                    key = fnv1a_64(unsafe { name.get_unchecked(shift) }.as_bytes());
                                    val = match map.get(&key) {
                                        Some(v) => v,
                                        None => return Some(Data::Bool(false)),
                                    };
                                } else {
                                    return Some(Data::Bool(false));
                                }
                                shift += 1;
                            }
                            Some(Data::Bool(true))
                        }
                        Filter::Unset => {
                            let mut key = fnv1a_64(unsafe { name.get_unchecked(0) }.as_bytes());
                            let mut val = match data.get(&key) {
                                Some(v) => v,
                                None => match tmp.get(&key) {
                                    Some(v) => v,
                                    None => return Some(Data::Bool(true)),
                                },
                            };
                            let mut shift = 1;
                            while shift < name.len() {
                                if let Data::Map(map) = val {
                                    key = fnv1a_64(unsafe { name.get_unchecked(shift) }.as_bytes());
                                    val = match map.get(&key) {
                                        Some(v) => v,
                                        None => return Some(Data::Bool(true)),
                                    };
                                } else {
                                    return Some(Data::Bool(true));
                                }
                                shift += 1;
                            }
                            Some(Data::Bool(false))
                        }
                        _ => None,
                    }
                } else {
                    None
                }
            }
        }
    }

    /// Extract Data from a value for For condition
    fn get_for_data(val: &Value, data: &HashMap<i64, Data>, tmp: &HashMap<i64, Data>) -> Option<Data> {
        if let Value::Value { name, filter } = val {
            if *filter == Filter::None || !name.is_empty() {
                let mut key = fnv1a_64(unsafe { name.get_unchecked(0) }.as_bytes());
                let mut val = match data.get(&key) {
                    Some(v) => v,
                    None => match tmp.get(&key) {
                        Some(v) => v,
                        None => return None,
                    },
                };
                let mut shift = 1;
                while shift < name.len() {
                    if let Data::Map(map) = val {
                        key = fnv1a_64(unsafe { name.get_unchecked(shift) }.as_bytes());
                        val = match map.get(&key) {
                            Some(v) => v,
                            None => return None,
                        };
                    } else {
                        return None;
                    }
                    shift += 1;
                }
                Some(val.clone())
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Extract Data from Value and print its
    fn print_echo(val: &Value, data: &HashMap<i64, Data>, tmp: &HashMap<i64, Data>) -> String {
        match val {
            Value::Number(n) => format!("{{{{err::Number({})}}}}", n),
            Value::Value { name, filter } => match filter {
                Filter::None => Html::escape(Html::data_to_text(name, data, tmp)),
                Filter::Raw => Html::data_to_text(name, data, tmp),
                Filter::Index => Html::data_to_index(name, tmp),
                Filter::Len => "{{err::Len}}".to_owned(),
                Filter::Set => "{{err::Set}}".to_owned(),
                Filter::Unset => "{{err::Unset}}".to_owned(),
                Filter::Dump => Html::data_to_dump(name, data, tmp),
            },
        }
    }

    fn data_to_dump(name: &[String], data: &HashMap<i64, Data>, tmp: &HashMap<i64, Data>) -> String {
        if name.is_empty() {
            return "{{{{EMPTY}}}}".to_owned();
        }
        let mut key = fnv1a_64(unsafe { name.get_unchecked(0) }.as_bytes());
        let mut val = match data.get(&key) {
            Some(v) => v,
            None => match tmp.get(&key) {
                Some(v) => v,
                None => return format!("{{{{KEY={}}}}}", name.join(".")),
            },
        };
        let mut shift = 1;
        while shift < name.len() {
            if let Data::Map(map) = val {
                key = fnv1a_64(unsafe { name.get_unchecked(shift) }.as_bytes());
                val = match map.get(&key) {
                    Some(v) => v,
                    None => return format!("{{{{KEY={}}}}}", name.join(".")),
                };
            } else {
                return format!("{{{{KEY={}}}}}", name.join("."));
            }
            shift += 1;
        }
        format!("{{{{KEY={} VALUE={:?}}}}}", name.join("."), val)
    }

    /// Extract string from value
    /// name.subname.othername|Filter
    fn data_to_text(name: &[String], data: &HashMap<i64, Data>, tmp: &HashMap<i64, Data>) -> String {
        if name.is_empty() {
            return "{{unknown}}".to_owned();
        }
        let mut key = fnv1a_64(unsafe { name.get_unchecked(0) }.as_bytes());
        let mut val = match data.get(&key) {
            Some(v) => v,
            None => match tmp.get(&key) {
                Some(v) => v,
                None => return format!("{{{{{}}}}}", name.join(".")),
            },
        };
        let mut shift = 1;
        while shift < name.len() {
            if let Data::Map(map) = val {
                key = fnv1a_64(unsafe { name.get_unchecked(shift) }.as_bytes());
                val = match map.get(&key) {
                    Some(v) => v,
                    None => return format!("{{{{{}}}}}", name.join(".")),
                };
            } else {
                return format!("{{{{{}}}}}", name.join("."));
            }
            shift += 1;
        }
        Html::print_data(val)
    }

    /// Data to String
    fn print_data(val: &Data) -> String {
        match val {
            Data::U8(u) => u.to_string(),
            Data::U16(u) => u.to_string(),
            Data::U32(u) => u.to_string(),
            Data::U64(u) => u.to_string(),
            Data::I8(i) => i.to_string(),
            Data::None => String::new(),
            Data::Usize(i) => i.to_string(),
            Data::I16(i) => i.to_string(),
            Data::I32(i) => i.to_string(),
            Data::I64(i) => i.to_string(),
            Data::F32(f) => f.to_string(),
            Data::F64(f) => f.to_string(),
            Data::Bool(b) => b.to_string(),
            Data::String(s) => s.to_owned(),
            Data::Date(d) => d.to_string(),
            Data::Json(j) => j.to_string(),
            Data::Vec(v) => format!("{:?}", v),
            Data::Raw(r) => format!("{:?}", r),
            Data::Map(m) => format!("{:?}", m),
        }
    }

    /// Print index of loop
    fn data_to_index(name: &[String], tmp: &HashMap<i64, Data>) -> String {
        if name.len() == 1 {
            let key = fnv1a_64(format!("{}|idx", unsafe { name.get_unchecked(0) }).as_bytes());
            match tmp.get(&key) {
                Some(Data::Usize(i)) => i.to_string(),
                _ => {
                    let mut res = name.join(".");
                    res.push_str("|idx");
                    res
                }
            }
        } else {
            let mut res = name.join(".");
            res.push_str("|idx");
            res
        }
    }

    /// Escape text
    fn escape(text: String) -> String {
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
            return text;
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
        new_text
    }

    /// Load templates's files
    async fn get_files(root: Arc<PathBuf>) -> Vec<(PathBuf, String, String, String)> {
        let mut vec = Vec::new();
        let read_path = match read_dir(&*root) {
            Ok(r) => r,
            Err(_e) => {
                log!(warning, 0, "Path: {:?}. Err: {}", root, _e);
                return vec;
            }
        };

        // Read first level dir
        for entry in read_path {
            let path = match entry {
                Ok(e) => e.path(),
                Err(_e) => {
                    log!(warning, 0, "{} ({:?})", _e, root);
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
                Err(_e) => {
                    log!(warning, 0, "{} ({})", _e, path.display());
                    continue;
                }
            };
            // Read second level dir
            for entry in read_path {
                let path = match entry {
                    Ok(e) => e.path(),
                    Err(_e) => {
                        log!(warning, 0, "{} ({})", _e, path.display());
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
                    Err(_e) => {
                        log!(warning, 0, "{} ({})", _e, path.display());
                        continue;
                    }
                };
                // Read third level dir
                for entry in read_path {
                    let path = match entry {
                        Ok(e) => e.path(),
                        Err(_e) => {
                            log!(warning, 0, "{} ({})", _e, path.display());
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
                        let view = view[..view.len() - 5].to_owned();
                        vec.push((path, module.to_owned(), class.to_owned(), view));
                    }
                }
            }
        }
        vec
    }

    /// Check system time
    #[cfg(feature = "html-reload")]
    pub(crate) async fn check_time(&self) -> bool {
        let files = Html::get_files(Arc::clone(&self.root)).await;
        let mut last_time = SystemTime::UNIX_EPOCH;
        let mut hash: i128 = 0;

        for (path, _, _, _) in files {
            if let Ok(metadata) = fs::metadata(&path).await {
                if let Ok(modified_time) = metadata.modified() {
                    if modified_time > last_time {
                        last_time = modified_time;
                    }
                    if let Some(s) = path.as_os_str().to_str() {
                        hash += fnv1a_64(s.as_bytes()) as i128;
                    }
                }
            }
        }
        last_time != self.last || hash != self.hash
    }

    /// Load translates
    pub(crate) async fn load(&mut self) {
        #[cfg(feature = "html-reload")]
        let mut last_time = SystemTime::UNIX_EPOCH;
        #[cfg(feature = "html-reload")]
        let mut hash: i128 = 0;

        let mut list = HashMap::new();
        let files = Html::get_files(Arc::clone(&self.root)).await;

        for (path, module, class, view) in files {
            if let Ok(html) = read_to_string(&path) {
                #[cfg(feature = "html-reload")]
                if let Ok(metadata) = fs::metadata(&path).await {
                    if let Ok(modified_time) = metadata.modified() {
                        if modified_time > last_time {
                            last_time = modified_time;
                        }
                        if let Some(s) = path.as_os_str().to_str() {
                            hash += fnv1a_64(s.as_bytes()) as i128;
                        }
                    }
                }

                // Parse templates
                match Html::parse(html.as_str()) {
                    Ok(v) => {
                        let module = match list.entry(fnv1a_64(module.as_bytes())) {
                            Entry::Vacant(entry) => entry.insert(HashMap::new()),
                            Entry::Occupied(entry) => entry.into_mut(),
                        };
                        let class = match module.entry(fnv1a_64(class.as_bytes())) {
                            Entry::Vacant(entry) => entry.insert(Arc::new(HashMap::new())),
                            Entry::Occupied(entry) => entry.into_mut(),
                        };
                        if let Some(views) = Arc::get_mut(class) {
                            views.insert(fnv1a_64(view.as_bytes()), v);
                        }
                    }
                    Err(_e) => {
                        log!(warning, 0, "{} ({})", _e, path.display());
                        continue;
                    }
                }
            }
        }
        self.list = list;
        #[cfg(feature = "html-reload")]
        {
            self.last = last_time;
        }
        #[cfg(feature = "html-reload")]
        {
            self.hash = hash;
        }
    }

    #[cfg(feature = "html-reload")]
    pub(crate) async fn reload(html: Arc<RwLock<Html>>) {
        let wr = match WRLOCK.get() {
            Some(wr) => wr,
            None => {
                log!(warning, 0);
                return;
            }
        };
        let wait = wr.lock.swap(true, Ordering::SeqCst);
        if wait {
            wr.notify.notified().await
        } else {
            let reload = html.read().await.check_time().await;
            if reload {
                html.write().await.load().await
            }
            wr.lock.store(false, Ordering::SeqCst);
            wr.notify.notify_waiters();
        }
    }
}
