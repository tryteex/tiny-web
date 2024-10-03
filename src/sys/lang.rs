use std::{
    collections::{btree_map::Entry, BTreeMap},
    fs::{read_dir, read_to_string},
    path::PathBuf,
    sync::Arc,
};

#[cfg(debug_assertions)]
use std::time::SystemTime;

use crate::fnv1a_64;
use tiny_web_macro::fnv1a_64 as m_fnv1a_64;

#[cfg(debug_assertions)]
use tokio::fs;

use toml::{Table, Value};

use super::{data::Data, dbs::adapter::DB, log::Log};

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
    pub code: String,
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
    #[cfg(debug_assertions)]
    pub(crate) root: Arc<String>,
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
    pub async fn new(root: Arc<String>, default_lang: &str, db: &mut DB) -> Lang {
        #[cfg(debug_assertions)]
        let last_time = SystemTime::UNIX_EPOCH;
        let mut codes = BTreeMap::new();

        let files = Lang::get_files(Arc::clone(&root)).await;

        let langs = if db.in_use() { Lang::get_langs(db).await } else { Lang::get_langs_install(&files) };

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
                #[cfg(debug_assertions)]
                root,
                codes,
            };
        }

        let mut default = 0;
        for item in &langs {
            codes.insert(item.code.clone(), item.id);
            if item.code == default_lang {
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
            #[cfg(debug_assertions)]
            root,
            codes,
        };
        lang.load(files).await;
        lang
    }

    pub(crate) async fn get_all_langs(db: Arc<DB>) -> Vec<LangItem> {
        let mut vec = Vec::with_capacity(200);
        if db.in_use() {
            let res = match db.query_prepare(m_fnv1a_64!("lib_get_all_langs"), &[], false).await {
                Some(r) => r,
                None => {
                    Log::warning(1150, None);
                    return Vec::new();
                }
            };
            if res.is_empty() {
                Log::warning(1151, None);
                return Vec::new();
            }
            for row in res {
                if let Data::Vec(row) = row {
                    if row.len() != 4 {
                        Log::warning(1150, None);
                        return Vec::new();
                    }
                    let id = if let Data::I64(val) = unsafe { row.get_unchecked(0) } {
                        *val
                    } else {
                        Log::warning(1150, None);
                        return Vec::new();
                    };
                    let index = if let Data::I64(val) = unsafe { row.get_unchecked(3) } {
                        *val
                    } else {
                        Log::warning(1150, None);
                        return Vec::new();
                    };
                    let code = if let Data::String(val) = unsafe { row.get_unchecked(1) } {
                        val.to_owned()
                    } else {
                        Log::warning(1150, None);
                        return Vec::new();
                    };
                    let name = if let Data::String(val) = unsafe { row.get_unchecked(2) } {
                        val.to_owned()
                    } else {
                        Log::warning(1150, None);
                        return Vec::new();
                    };
                    vec.push(LangItem { id, code, name, index });
                } else {
                    Log::warning(1150, None);
                    return Vec::new();
                }
            }
        } else {
            return Lang::gelt_all_langs_install();
        }

        vec
    }

    /// Get list of enabled langs from database
    async fn get_langs(db: &mut DB) -> Vec<Arc<LangItem>> {
        let res = match db.query_prepare(m_fnv1a_64!("lib_get_langs"), &[], false).await {
            Some(r) => r,
            None => {
                Log::warning(1150, None);
                return Vec::new();
            }
        };
        if res.is_empty() {
            Log::warning(1151, None);
            return Vec::new();
        }
        let mut vec = Vec::with_capacity(res.len());
        for row in res {
            if let Data::Vec(row) = row {
                if row.len() != 4 {
                    Log::warning(1150, None);
                    return Vec::new();
                }
                let id = if let Data::I64(val) = unsafe { row.get_unchecked(0) } {
                    *val
                } else {
                    Log::warning(1150, None);
                    return Vec::new();
                };
                let index = if let Data::I64(val) = unsafe { row.get_unchecked(3) } {
                    *val
                } else {
                    Log::warning(1150, None);
                    return Vec::new();
                };
                let code = if let Data::String(val) = unsafe { row.get_unchecked(1) } {
                    val.to_owned()
                } else {
                    Log::warning(1150, None);
                    return Vec::new();
                };
                let name = if let Data::String(val) = unsafe { row.get_unchecked(2) } {
                    val.to_owned()
                } else {
                    Log::warning(1150, None);
                    return Vec::new();
                };
                vec.push(Arc::new(LangItem { id, code, name, index }));
            } else {
                Log::warning(1150, None);
                return Vec::new();
            }
        }
        vec
    }

    /// Other languages ​​will be added as quality translation is provided
    fn gelt_all_langs_install() -> Vec<LangItem> {
        let list = vec![
            LangItem {
                id: 0,
                code: "en".to_owned(),
                name: "English".to_string(),
                index: m_fnv1a_64!("en"),
            },
            LangItem {
                id: 1,
                code: "uk".to_owned(),
                name: "Ukrainian (Українська)".to_string(),
                index: m_fnv1a_64!("uk"),
            },
            LangItem {
                id: 28,
                code: "cs".to_owned(),
                name: "Czech (Čeština)".to_string(),
                index: m_fnv1a_64!("cs"),
            },
            LangItem {
                id: 40,
                code: "et".to_owned(),
                name: "Estonian (Eesti)".to_string(),
                index: m_fnv1a_64!("et"),
            },
            LangItem {
                id: 97,
                code: "lt".to_owned(),
                name: "Lithuanian (Lietuvių kalba)".to_string(),
                index: m_fnv1a_64!("lt"),
            },
            LangItem {
                id: 99,
                code: "lv".to_owned(),
                name: "Latvian (Latviešu valoda)".to_string(),
                index: m_fnv1a_64!("lv"),
            },
            LangItem {
                id: 117,
                code: "no".to_owned(),
                name: "Norwegian (Norsk)".to_string(),
                index: m_fnv1a_64!("no"),
            },
            LangItem {
                id: 128,
                code: "pl".to_owned(),
                name: "Polish (Język polski)".to_string(),
                index: m_fnv1a_64!("pl"),
            },
        ];
        list
    }

    /// Get list of enabled langs for install module
    pub(crate) fn get_langs_install(files: &Vec<(PathBuf, String, String, String)>) -> Vec<Arc<LangItem>> {
        let list = Lang::gelt_all_langs_install();

        let mut vec = Vec::with_capacity(files.len());
        let mut index = 0;
        for (_, module, class, code) in files {
            if module == "index" && class == "install" {
                for lang in &list {
                    if lang.index == fnv1a_64(code.as_bytes()) {
                        let mut l = lang.clone();
                        l.index = index;
                        vec.push(Arc::new(l));
                        index += 1;
                        break;
                    }
                }
            }
        }

        vec
    }

    /// Load lang's files
    pub(crate) async fn get_files(root: Arc<String>) -> Vec<(PathBuf, String, String, String)> {
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
        let files = Lang::get_files(Arc::clone(&self.root)).await;
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
    pub(crate) async fn load(&mut self, files: Vec<(PathBuf, String, String, String)>) {
        #[cfg(debug_assertions)]
        let mut last_time = SystemTime::UNIX_EPOCH;
        #[cfg(debug_assertions)]
        let mut hash: i128 = 0;

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
