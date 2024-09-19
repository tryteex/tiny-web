use crate::fnv1a_64;
use tiny_web_macro::fnv1a_64;

/// Route of request
///
///  # Values
///
/// * `module: String` - Start module.
/// * `class: String` - Start class.
/// * `action: String` - Start action (controller).
/// * `module_id: i64` - Module id.
/// * `class_id: i64` - Class id.
/// * `action_id: i64` - Action id.
/// * `param: Option<String>` - Controller param.
/// * `lang_id: Option<i64>` - Set lang id.
#[derive(Debug, Clone)]
pub struct Route {
    /// Start module
    pub module: String,
    /// Start class
    pub class: String,
    /// Start action (controller)
    pub action: String,
    /// Module id
    pub module_id: i64,
    /// Class id
    pub class_id: i64,
    /// Action id
    pub action_id: i64,
    /// Controller param
    pub param: Option<String>,
    /// Set lang id
    pub lang_id: Option<i64>,
}

impl Route {
    pub fn default_index() -> Route {
        Route {
            module: "index".to_owned(),
            class: "index".to_owned(),
            action: "index".to_owned(),
            module_id: fnv1a_64!("index"),
            class_id: fnv1a_64!("index"),
            action_id: fnv1a_64!("index"),
            param: None,
            lang_id: None,
        }
    }
    pub fn default_not_found() -> Route {
        Route {
            module: "index".to_owned(),
            class: "index".to_owned(),
            action: "not_found".to_owned(),
            module_id: fnv1a_64!("index"),
            class_id: fnv1a_64!("index"),
            action_id: fnv1a_64!("not_found"),
            param: None,
            lang_id: None,
        }
    }
    pub fn default_err() -> Route {
        Route {
            module: "index".to_owned(),
            class: "index".to_owned(),
            action: "err".to_owned(),
            module_id: fnv1a_64!("index"),
            class_id: fnv1a_64!("index"),
            action_id: fnv1a_64!("err"),
            param: None,
            lang_id: None,
        }
    }
    pub fn default_install() -> Route {
        Route {
            module: "index".to_owned(),
            class: "install".to_owned(),
            action: "index".to_owned(),
            module_id: fnv1a_64!("index"),
            class_id: fnv1a_64!("install"),
            action_id: fnv1a_64!("index"),
            param: None,
            lang_id: None,
        }
    }

    pub fn parse(val: &str) -> Option<Route> {
        let vec: Vec<&str> = val.split('/').collect();
        if vec.len() < 4 || vec.len() > 5 {
            return None;
        }
        if !unsafe { *vec.get_unchecked(0) }.is_empty() {
            return None;
        }
        let module = unsafe { *vec.get_unchecked(1) };
        if module.is_empty() {
            return None;
        }
        let class = unsafe { *vec.get_unchecked(2) };
        if class.is_empty() {
            return None;
        }
        let action = unsafe { *vec.get_unchecked(3) };
        if action.is_empty() {
            return None;
        }
        let param = if vec.len() == 5 {
            let param = unsafe { *vec.get_unchecked(4) };
            if param.is_empty() {
                return None;
            }
            Some(param.to_owned())
        } else {
            None
        };
        let module = module.to_owned();
        let module_id = fnv1a_64(module.as_bytes());
        let class: String = class.to_owned();
        let class_id = fnv1a_64(class.as_bytes());
        let action = action.to_owned();
        let action_id = fnv1a_64(action.as_bytes());
        Some(Route {
            module,
            class,
            action,
            module_id,
            class_id,
            action_id,
            param,
            lang_id: None,
        })
    }
}
