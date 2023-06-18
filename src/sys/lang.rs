use std::{
    collections::{btree_map::Entry, BTreeMap},
    fs::{read_dir, read_to_string},
    sync::Arc,
};

use crate::fnv1a_64;

use super::log::Log;

/// Describes a language element
///
/// # Values
///
/// * `id: i64` - Language ID.
/// * `lang: String` - Languane name ISO 639-1: uk - ukrainian, en - english, en - english.
/// * `code: String` - Languane code ISO 3166 alpha-2: ua - Ukraine, us - USA, gb - United Kingdom.
/// * `name: String` - Native name of the language.
#[derive(Debug, Clone)]
pub struct LangItem {
    /// Language ID
    pub id: i64,
    /// Languane name ISO 639-1: uk - ukrainian, en - english, en - english
    pub lang: String,
    /// Languane code ISO 3166 alpha-2: ua - Ukraine, us - USA, gb - United Kingdom
    pub code: String,
    /// Native name of the language
    pub name: String,
}

/// I18n
///
/// # Index
///
/// * 1 - language ID
/// * 2 - Module ID
/// * 3 - Class ID
/// * 4 - Key ID
/// * 5 - Key value
type LangList = BTreeMap<i64, BTreeMap<i64, BTreeMap<i64, Arc<BTreeMap<i64, String>>>>>;

/// Descrives all languages
///
/// # Values
///
/// * `langs: Vec<LangItem>` - List of languages
/// * `list: LangList` - List of translations
/// * `default: usize` - Default language
#[derive(Debug)]
pub struct Lang {
    /// List of languages
    pub langs: Vec<LangItem>,
    /// List of translations
    pub list: LangList,
    /// Default language
    pub default: usize,
}

impl Lang {
    /// Reads ./app/ and recognizes translations
    ///
    /// # Description
    ///
    /// In the root directory of the project (`Init::root_path`) the `app` directory is searched.  
    ///
    /// Translation files are logically located in this directory.  
    /// Each file must be named `LangItem::lang` and have the extension `.lang`
    ///
    /// ## Example:
    ///
    /// * English:   ./app/module_name/class_name/en.lang
    /// * Ukrainian: ./app/module_name/class_name/uk.lang
    ///
    /// module_name - Name of the module  <br />
    /// class_name - Class name  
    ///
    /// For all controllers in the same class - one translation file in one language is used.
    ///
    /// Each translation file is divided into lines.  
    /// Each line consists of a key and a translation.  
    ///
    /// ## Example:
    ///
    /// `en.lang`<br />
    /// about=About<br />
    /// articles=Articles<br />
    /// article=Article<br />
    /// contact=Contact<br />
    /// terms=Terms Conditions<br />
    /// policy=Privacy Policy<br />
    ///
    /// ## Use in the controller:
    ///
    /// To get a translation, it is enough to set the `this.lang("contact")` function,
    /// which will return the corresponding translation.<br />
    /// If no translation is found, the key will be returned.
    pub fn new(root: &str, default_lang: &str, langs: Vec<LangItem>) -> Lang {
        if langs.is_empty() {
            Log::warning(1151, None);
            return Lang {
                langs: Vec::new(),
                list: BTreeMap::new(),
                default: 0,
            };
        }

        let mut codes = BTreeMap::new();
        let mut default = 0;
        for item in &langs {
            codes.insert(item.code.clone(), item.id);
            if item.code == default_lang {
                default = item.id as usize;
            }
        }

        let path = format!("{}/app/", root);
        let read_path = match read_dir(&path) {
            Ok(r) => r,
            Err(e) => {
                Log::warning(1100, Some(format!("Path: {}. Err: {}", path, e)));
                return Lang {
                    langs,
                    list: BTreeMap::new(),
                    default,
                };
            }
        };

        // Read first level dir
        let mut list = BTreeMap::new();
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
                    let code = match path.file_name() {
                        Some(v) => match v.to_str() {
                            Some(view) => view,
                            None => continue,
                        },
                        None => continue,
                    };
                    if code.ends_with(".lang") && code.len() == 7 {
                        let code = &code[0..2];
                        if let Some(id) = codes.get(code) {
                            if let Ok(text) = read_to_string(&path) {
                                if !text.is_empty() {
                                    for part in text.split('\n') {
                                        let line = part.trim();
                                        if !line.is_empty() && line.contains('=') {
                                            let vals: Vec<&str> = line.splitn(2, '=').collect();
                                            if vals.len() == 2 {
                                                let key = vals[0].trim();
                                                let val = vals[1].trim();
                                                // lang_id
                                                let l1 = match list.entry(*id) {
                                                    Entry::Vacant(v) => v.insert(BTreeMap::new()),
                                                    Entry::Occupied(o) => o.into_mut(),
                                                };
                                                // module
                                                let l2 = match l1.entry(fnv1a_64(module)) {
                                                    Entry::Vacant(v) => v.insert(BTreeMap::new()),
                                                    Entry::Occupied(o) => o.into_mut(),
                                                };
                                                // class
                                                let l3 = match l2.entry(fnv1a_64(class)) {
                                                    Entry::Vacant(v) => v.insert(BTreeMap::new()),
                                                    Entry::Occupied(o) => o.into_mut(),
                                                };
                                                l3.insert(fnv1a_64(key), val.to_owned());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        // Add Arc to async operation
        let mut list_lang = BTreeMap::new();
        for (key_lang, item_lang) in list {
            let mut list_module = BTreeMap::new();
            for (key_module, item_module) in item_lang {
                let mut list_class = BTreeMap::new();
                for (key_class, item_class) in item_module {
                    list_class.insert(key_class, Arc::new(item_class));
                }
                list_module.insert(key_module, list_class);
            }
            list_lang.insert(key_lang, list_module);
        }
        Lang {
            langs,
            list: list_lang,
            default,
        }
    }
}
