use std::{
    collections::{btree_map::Entry, BTreeMap},
    fs::{read_dir, read_to_string},
    path::PathBuf,
    sync::Arc,
};

#[cfg(debug_assertions)]
use std::time::SystemTime;

#[cfg(debug_assertions)]
use tokio::fs;

use toml::{Table, Value};

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
/// * `index: i64` - Index in JSON type field db.
#[derive(Debug, Clone)]
pub struct LangItem {
    /// Language ID
    pub id: i64,
    /// Languane name ISO 639-1: uk - ukrainian, en - english, en - english
    pub lang: String,
    /// Native name of the language
    pub name: String,
    /// Index in JSON type field db
    pub index: i64,
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
    pub langs: Arc<Vec<Arc<LangItem>>>,
    /// List of translations
    pub list: Arc<LangList>,
    /// Default language
    pub default: usize,
    /// SystemTime last modification
    #[cfg(debug_assertions)]
    last: SystemTime,
    /// Sum of all filename hashes
    #[cfg(debug_assertions)]
    hash: i128,
    /// Path to langs' files
    root: Arc<String>,
    /// List of lang codes
    codes: BTreeMap<String, i64>,
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
    pub async fn new(root: &str, default_lang: &str, langs: Vec<Arc<LangItem>>) -> Lang {
        #[cfg(debug_assertions)]
        let last_time = SystemTime::UNIX_EPOCH;
        let mut codes = BTreeMap::new();

        if langs.is_empty() {
            Log::warning(1151, None);
            return Lang {
                langs: Arc::new(Vec::new()),
                list: Arc::new(BTreeMap::new()),
                default: 0,
                #[cfg(debug_assertions)]
                last: last_time,
                #[cfg(debug_assertions)]
                hash: 0,
                root: Arc::new(root.to_owned()),
                codes,
            };
        }

        let mut default = 0;
        for item in &langs {
            codes.insert(item.lang.clone(), item.id);
            if item.lang == default_lang {
                default = item.id as usize;
            }
        }
        let mut lang = Lang {
            langs: Arc::new(langs),
            list: Arc::new(BTreeMap::new()),
            default,
            #[cfg(debug_assertions)]
            last: last_time,
            #[cfg(debug_assertions)]
            hash: 0,
            root: Arc::new(root.to_owned()),
            codes,
        };
        lang.load().await;
        lang
    }

    /// Load lang's files
    async fn get_files(root: &str) -> Vec<(PathBuf, String, String, String)> {
        let mut vec = Vec::new();

        let path = format!("{}/app/", root);
        let read_path = match read_dir(&path) {
            Ok(r) => r,
            Err(e) => {
                Log::warning(1100, Some(format!("Path: {}. Err: {}", path, e)));
                return vec;
            }
        };

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
                    if code.starts_with("lang.") && code.len() == 7 {
                        let code = code[5..7].to_owned();
                        vec.push((path, module.to_owned(), class.to_owned(), code));
                    }
                }
            }
        }
        vec
    }

    /// Check system time
    #[cfg(debug_assertions)]
    pub(crate) async fn check_time(&self) -> bool {
        let files = Lang::get_files(&self.root).await;
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
        #[cfg(debug_assertions)]
        let mut last_time = SystemTime::UNIX_EPOCH;
        #[cfg(debug_assertions)]
        let mut hash: i128 = 0;

        let files = Lang::get_files(&self.root).await;
        let mut list = BTreeMap::new();

        for (path, module, class, code) in files {
            if let Some(id) = self.codes.get(&code) {
                if let Ok(text) = read_to_string(&path) {
                    #[cfg(debug_assertions)]
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
                    if !text.is_empty() {
                        let text = match text.parse::<Table>() {
                            Ok(v) => v,
                            Err(e) => {
                                Log::warning(19, Some(format!("{:?} {} ", path.to_str(), e)));
                                continue;
                            }
                        };
                        for (key, value) in text {
                            if let Value::String(val) = value {
                                let l1 = match list.entry(*id) {
                                    Entry::Vacant(v) => v.insert(BTreeMap::new()),
                                    Entry::Occupied(o) => o.into_mut(),
                                };
                                // module
                                let l2 = match l1.entry(fnv1a_64(module.as_bytes())) {
                                    Entry::Vacant(v) => v.insert(BTreeMap::new()),
                                    Entry::Occupied(o) => o.into_mut(),
                                };
                                // class
                                let l3 = match l2.entry(fnv1a_64(class.as_bytes())) {
                                    Entry::Vacant(v) => v.insert(BTreeMap::new()),
                                    Entry::Occupied(o) => o.into_mut(),
                                };
                                l3.insert(fnv1a_64(key.as_bytes()), val);
                            } else {
                                Log::warning(20, Some(format!("{:?} {} ", path.to_str(), value)));
                                continue;
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
        self.list = Arc::new(list_lang);
        #[cfg(debug_assertions)]
        {
            self.last = last_time;
        }
        #[cfg(debug_assertions)]
        {
            self.hash = hash;
        }
    }
}
