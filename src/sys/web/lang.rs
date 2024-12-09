use std::{
    collections::{hash_map::Entry, HashMap},
    fs::{read_dir, read_to_string},
    path::PathBuf,
    sync::Arc,
};

#[cfg(feature = "lang-reload")]
use std::sync::atomic::Ordering;

#[cfg(feature = "lang-reload")]
use std::time::SystemTime;

use toml::{Table, Value};

#[cfg(feature = "session-db")]
use tiny_web_macro::fnv1a_64 as m_fnv1a_64;

#[cfg(feature = "lang-reload")]
use tokio::fs;

#[cfg(feature = "lang-reload")]
use tokio::sync::OnceCell;

#[cfg(feature = "lang-reload")]
use tokio::sync::RwLock;

use crate::{fnv1a_64, log};

#[cfg(feature = "lang-reload")]
use crate::sys::wrlock::WrLock;

#[cfg(feature = "session-db")]
use crate::sys::db::adapter::DB;

/// Describes a language element
#[derive(Debug, Clone)]
pub struct LangItem {
    /// Language ID
    pub id: usize,
    /// Languane name ISO 639-1: uk - ukrainian, en - english, en - english
    pub code: String,
    /// Native name of the language
    pub name: String,
    /// Index in JSON type field db
    pub index: usize,
}

struct LangFile {
    path: PathBuf,
    module: String,
    class: String,
    code: String,
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
type LangList = HashMap<usize, HashMap<i64, HashMap<i64, Arc<HashMap<i64, String>>>>>;

#[cfg(feature = "lang-reload")]
static WRLOCK: OnceCell<WrLock> = OnceCell::const_new();

pub(crate) struct LangParam {
    pub root: Arc<PathBuf>,
    pub default_lang: Arc<String>,
    #[cfg(feature = "session-db")]
    pub db: Arc<DB>,
}

/// Descrives all languages
#[derive(Debug)]
pub(crate) struct Lang {
    /// List of languages
    pub langs: Arc<Vec<Arc<LangItem>>>,
    /// List of translations
    pub list: Arc<LangList>,
    /// Default language
    pub default: usize,
    /// SystemTime last modification
    #[cfg(feature = "lang-reload")]
    last: SystemTime,
    /// Sum of all filename hashes
    #[cfg(feature = "lang-reload")]
    hash: i128,
    /// Path to langs' files
    #[cfg(feature = "lang-reload")]
    root: Arc<PathBuf>,

    codes: HashMap<String, usize>,
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
    pub async fn new(param: LangParam) -> Result<Lang, ()> {
        #[cfg(feature = "lang-reload")]
        if WRLOCK.set(WrLock::default()).is_err() {
            return Err(());
        }

        #[cfg(feature = "lang-reload")]
        let last_time = SystemTime::UNIX_EPOCH;

        let mut root = param.root.as_ref().clone();
        root.push("app");

        // List of files
        let files = Lang::get_files(Arc::clone(&param.root)).await;

        // List of available languages
        #[cfg(not(feature = "session-db"))]
        let langs = Lang::get_list();

        #[cfg(feature = "session-db")]
        let langs = Lang::get_list(param.db).await;

        if langs.is_empty() {
            log!(warning, 0);
            return Err(());
        }

        let mut default = None;
        let mut codes = HashMap::new();

        for item in &langs {
            codes.insert(item.code.clone(), item.id);
            if &item.code == param.default_lang.as_ref() {
                default = Some(item.id);
            }
        }
        let default = match default {
            Some(default) => default,
            None => {
                log!(warning, 0);
                return Err(());
            }
        };
        let mut lang = Lang {
            langs: Arc::new(langs),
            list: Arc::new(HashMap::new()),
            default,
            #[cfg(feature = "lang-reload")]
            last: last_time,
            #[cfg(feature = "lang-reload")]
            hash: 0,
            #[cfg(feature = "lang-reload")]
            root: Arc::new(root),
            codes,
        };
        lang.load(files).await;
        Ok(lang)
    }

    /// Load translates
    async fn load(&mut self, files: Vec<LangFile>) {
        #[cfg(feature = "lang-reload")]
        let mut last_time = SystemTime::UNIX_EPOCH;
        #[cfg(feature = "lang-reload")]
        let mut hash: i128 = 0;

        let mut list = HashMap::new();

        for file in files {
            if let Some(id) = self.codes.get(&file.code) {
                if let Ok(text) = read_to_string(&file.path) {
                    #[cfg(feature = "lang-reload")]
                    if let Ok(metadata) = fs::metadata(&file.path).await {
                        if let Ok(modified_time) = metadata.modified() {
                            if modified_time > last_time {
                                last_time = modified_time;
                            }
                            if let Some(s) = file.path.as_os_str().to_str() {
                                hash += fnv1a_64(s.as_bytes()) as i128;
                            }
                        }
                    }
                    if !text.is_empty() {
                        let text = match text.parse::<Table>() {
                            Ok(v) => v,
                            Err(_e) => {
                                log!(warning, 0, "{:?} {}", file.path, _e);
                                continue;
                            }
                        };
                        for (key, value) in text {
                            if let Value::String(val) = value {
                                let l1 = match list.entry(*id) {
                                    Entry::Vacant(v) => v.insert(HashMap::new()),
                                    Entry::Occupied(o) => o.into_mut(),
                                };
                                // module
                                let l2 = match l1.entry(fnv1a_64(file.module.as_bytes())) {
                                    Entry::Vacant(v) => v.insert(HashMap::new()),
                                    Entry::Occupied(o) => o.into_mut(),
                                };
                                // class
                                let l3 = match l2.entry(fnv1a_64(file.class.as_bytes())) {
                                    Entry::Vacant(v) => v.insert(HashMap::new()),
                                    Entry::Occupied(o) => o.into_mut(),
                                };
                                l3.insert(fnv1a_64(key.as_bytes()), val);
                            } else {
                                log!(warning, 0, "{:?} {} ", file.path, value);
                                continue;
                            }
                        }
                    }
                }
            }
        }

        // Add Arc to async operation
        let mut list_lang = HashMap::new();
        for (key_lang, item_lang) in list {
            let mut list_module = HashMap::new();
            for (key_module, item_module) in item_lang {
                let mut list_class = HashMap::new();
                for (key_class, item_class) in item_module {
                    list_class.insert(key_class, Arc::new(item_class));
                }
                list_module.insert(key_module, list_class);
            }
            list_lang.insert(key_lang, list_module);
        }
        self.list = Arc::new(list_lang);
        #[cfg(feature = "lang-reload")]
        {
            self.last = last_time;
        }
        #[cfg(feature = "lang-reload")]
        {
            self.hash = hash;
        }
    }

    /// Load lang's files
    async fn get_files(path: Arc<PathBuf>) -> Vec<LangFile> {
        let mut vec = Vec::new();

        let read_path = match read_dir(path.as_ref()) {
            Ok(r) => r,
            Err(_e) => {
                log!(warning, 0, "Path: {:?}. Err: {}", path, _e);
                return vec;
            }
        };

        // Read first level dir
        for entry in read_path {
            let path = match entry {
                Ok(e) => e.path(),
                Err(_e) => {
                    log!(warning, 0, "{} ({:?})", _e, path);
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
                    log!(warning, 0, "{} ({:?})", _e, path);
                    continue;
                }
            };

            // Read second level dir
            for entry in read_path {
                let path = match entry {
                    Ok(e) => e.path(),
                    Err(_e) => {
                        log!(warning, 0, "{} ({:?})", _e, path);
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
                        log!(warning, 0, "{} ({:?})", _e, path);
                        continue;
                    }
                };
                // Read third level dir
                for entry in read_path {
                    let path = match entry {
                        Ok(e) => e.path(),
                        Err(_e) => {
                            log!(warning, 0, "{} ({:?})", _e, path);
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
                    if code.starts_with("lang.") && code.ends_with(".toml") && code.len() == 12 {
                        let code = unsafe { code.get_unchecked(5..7) }.to_owned();
                        vec.push(LangFile {
                            path,
                            module: module.to_owned(),
                            class: class.to_owned(),
                            code,
                        });
                    }
                }
            }
        }
        vec
    }

    /// Check system time
    #[cfg(feature = "lang-reload")]
    pub(crate) async fn check_time(&self) -> bool {
        let files = Lang::get_files(Arc::clone(&self.root)).await;
        let mut last_time = SystemTime::UNIX_EPOCH;
        let mut hash: i128 = 0;

        for file in files {
            if let Ok(metadata) = fs::metadata(&file.path).await {
                if let Ok(modified_time) = metadata.modified() {
                    if modified_time > last_time {
                        last_time = modified_time;
                    }
                    if let Some(s) = file.path.as_os_str().to_str() {
                        hash += fnv1a_64(s.as_bytes()) as i128;
                    }
                }
            }
        }
        last_time != self.last || hash != self.hash
    }

    #[cfg(feature = "lang-reload")]
    pub(crate) async fn reload(lang: Arc<RwLock<Lang>>) {
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
            let reload = lang.read().await.check_time().await;
            if reload {
                let root = Arc::clone(&lang.read().await.root);
                let files = Lang::get_files(root).await;
                lang.write().await.load(files).await
            }
            wr.lock.store(false, Ordering::SeqCst);
            wr.notify.notify_waiters();
        }
    }

    #[cfg(feature = "session-db")]
    async fn get_list(db: Arc<DB>) -> Vec<Arc<LangItem>> {
        match db.query_prepare(m_fnv1a_64!("lib_get_langs"), &[]).await {
            Some(list) => {
                let mut vec = Vec::with_capacity(list.len());
                for row in list {
                    let id: i64 = row.get(0);
                    let code = row.get(1);
                    let name = row.get(2);
                    let index: i64 = match row.try_get(3) {
                        Ok(index) => index,
                        Err(_) => panic!("Not set lang_id index for lang {}", id),
                    };
                    let lang = LangItem {
                        id: id as usize,
                        code,
                        name,
                        index: index as usize,
                    };
                    vec.push(Arc::new(lang));
                }
                vec
            }
            None => Vec::new(),
        }
    }

    #[cfg(not(feature = "session-db"))]
    #[rustfmt::skip]
    fn get_list() -> Vec<Arc<LangItem>> {
        vec![
            Arc::new(LangItem { id: 0, code: "en".to_owned(), name: "English".to_owned(), index: 0}),
            Arc::new(LangItem { id: 1, code: "uk".to_owned(), name: "Ukrainian (Українська)".to_owned(), index: 1}),
            // Arc::new(LangItem { id: 2, code: "aa".to_owned(), name: "Afar (Afaraf)".to_owned(), index: 2}),
            // Arc::new(LangItem { id: 3, code: "ab".to_owned(), name: "Abkhaz (аҧсуа бызшәа, аҧсшәа)".to_owned(), index: 3}),
            // Arc::new(LangItem { id: 4, code: "ae".to_owned(), name: "Avestan (avesta)".to_owned(), index: 4}),
            // Arc::new(LangItem { id: 5, code: "af".to_owned(), name: "Afrikaans".to_owned(), index: 5}),
            // Arc::new(LangItem { id: 6, code: "ak".to_owned(), name: "Akan".to_owned(), index: 6}),
            // Arc::new(LangItem { id: 7, code: "am".to_owned(), name: "Amharic (አማርኛ)".to_owned(), index: 7}),
            // Arc::new(LangItem { id: 8, code: "an".to_owned(), name: "Aragonese (aragonés)".to_owned(), index: 8}),
            // Arc::new(LangItem { id: 9, code: "ar".to_owned(), name: "Arabic (العربية)".to_owned(), index: 9}),
            // Arc::new(LangItem { id: 10, code: "as".to_owned(), name: "Assamese (অসমীয়া)".to_owned(), index: 10}),
            // Arc::new(LangItem { id: 11, code: "av".to_owned(), name: "Avaric (авар мацӀ, магӀарул мацӀ)".to_owned(), index: 11}),
            // Arc::new(LangItem { id: 12, code: "ay".to_owned(), name: "Aymara (aymar aru)".to_owned(), index: 12}),
            // Arc::new(LangItem { id: 13, code: "az".to_owned(), name: "Azerbaijani (azərbaycan dili)".to_owned(), index: 13}),
            // Arc::new(LangItem { id: 14, code: "ba".to_owned(), name: "Bashkir (башҡорт теле)".to_owned(), index: 14}),
            // Arc::new(LangItem { id: 15, code: "bg".to_owned(), name: "Bulgarian (български език)".to_owned(), index: 15}),
            // Arc::new(LangItem { id: 16, code: "bh".to_owned(), name: "Bihari (भोजपुरी)".to_owned(), index: 16}),
            // Arc::new(LangItem { id: 17, code: "bi".to_owned(), name: "Bislama".to_owned(), index: 17}),
            // Arc::new(LangItem { id: 18, code: "bm".to_owned(), name: "Bambara (bamanankan)".to_owned(), index: 18}),
            // Arc::new(LangItem { id: 19, code: "bn".to_owned(), name: "Bengali, Bangla (বাংলা)".to_owned(), index: 19}),
            // Arc::new(LangItem { id: 20, code: "bo".to_owned(), name: "Tibetan Standard, Tibetan, Central (བོད་ཡིག)".to_owned(), index: 20}),
            // Arc::new(LangItem { id: 21, code: "br".to_owned(), name: "Breton (brezhoneg)".to_owned(), index: 21}),
            // Arc::new(LangItem { id: 22, code: "bs".to_owned(), name: "Bosnian (bosanski jezik)".to_owned(), index: 22}),
            // Arc::new(LangItem { id: 23, code: "ca".to_owned(), name: "Catalan (català)".to_owned(), index: 23}),
            // Arc::new(LangItem { id: 24, code: "ce".to_owned(), name: "Chechen (нохчийн мотт)".to_owned(), index: 24}),
            // Arc::new(LangItem { id: 25, code: "ch".to_owned(), name: "Chamorro (Chamoru)".to_owned(), index: 25}),
            // Arc::new(LangItem { id: 26, code: "co".to_owned(), name: "Corsican (corsu, lingua corsa)".to_owned(), index: 26}),
            // Arc::new(LangItem { id: 27, code: "cr".to_owned(), name: "Cree (ᓀᐦᐃᔭᐍᐏᐣ)".to_owned(), index: 27}),
            // Arc::new(LangItem { id: 28, code: "cs".to_owned(), name: "Czech (čeština, český jazyk)".to_owned(), index: 28}),
            // Arc::new(LangItem { id: 29, code: "cu".to_owned(), name: "Old Church Slavonic, Church Slavonic, Old Bulgarian (ѩзыкъ словѣньскъ)".to_owned(), index: 29}),
            // Arc::new(LangItem { id: 30, code: "cv".to_owned(), name: "Chuvash (чӑваш чӗлхи)".to_owned(), index: 30}),
            // Arc::new(LangItem { id: 31, code: "cy".to_owned(), name: "Welsh (Cymraeg)".to_owned(), index: 31}),
            // Arc::new(LangItem { id: 32, code: "da".to_owned(), name: "Danish (dansk)".to_owned(), index: 32}),
            // Arc::new(LangItem { id: 33, code: "de".to_owned(), name: "German (Deutsch)".to_owned(), index: 33}),
            // Arc::new(LangItem { id: 34, code: "dv".to_owned(), name: "Divehi, Dhivehi, Maldivian (ދިވެހި)".to_owned(), index: 34}),
            // Arc::new(LangItem { id: 35, code: "dz".to_owned(), name: "Dzongkha (རྫོང་ཁ)".to_owned(), index: 35}),
            // Arc::new(LangItem { id: 36, code: "ee".to_owned(), name: "Ewe (Eʋegbe)".to_owned(), index: 36}),
            // Arc::new(LangItem { id: 37, code: "el".to_owned(), name: "Greek (modern) (ελληνικά)".to_owned(), index: 37}),
            // Arc::new(LangItem { id: 38, code: "eo".to_owned(), name: "Esperanto".to_owned(), index: 38}),
            // Arc::new(LangItem { id: 39, code: "es".to_owned(), name: "Spanish (Español)".to_owned(), index: 39}),
            // Arc::new(LangItem { id: 40, code: "et".to_owned(), name: "Estonian (eesti, eesti keel)".to_owned(), index: 40}),
            // Arc::new(LangItem { id: 41, code: "eu".to_owned(), name: "Basque (euskara, euskera)".to_owned(), index: 41}),
            // Arc::new(LangItem { id: 42, code: "fa".to_owned(), name: "Persian (Farsi) (فارسی)".to_owned(), index: 42}),
            // Arc::new(LangItem { id: 43, code: "ff".to_owned(), name: "Fula, Fulah, Pulaar, Pular (Fulfulde, Pulaar, Pular)".to_owned(), index: 43}),
            // Arc::new(LangItem { id: 44, code: "fi".to_owned(), name: "Finnish (suomi, suomen kieli)".to_owned(), index: 44}),
            // Arc::new(LangItem { id: 45, code: "fj".to_owned(), name: "Fijian (vosa Vakaviti)".to_owned(), index: 45}),
            // Arc::new(LangItem { id: 46, code: "fo".to_owned(), name: "Faroese (føroyskt)".to_owned(), index: 46}),
            // Arc::new(LangItem { id: 47, code: "fr".to_owned(), name: "French (français, langue française)".to_owned(), index: 47}),
            // Arc::new(LangItem { id: 48, code: "fy".to_owned(), name: "Western Frisian (Frysk)".to_owned(), index: 48}),
            // Arc::new(LangItem { id: 49, code: "ga".to_owned(), name: "Irish (Gaeilge)".to_owned(), index: 49}),
            // Arc::new(LangItem { id: 50, code: "gd".to_owned(), name: "Scottish Gaelic, Gaelic (Gàidhlig)".to_owned(), index: 50}),
            // Arc::new(LangItem { id: 51, code: "gl".to_owned(), name: "Galician (galego)".to_owned(), index: 51}),
            // Arc::new(LangItem { id: 52, code: "gn".to_owned(), name: "Guaraní (Avañe'ẽ)".to_owned(), index: 52}),
            // Arc::new(LangItem { id: 53, code: "gu".to_owned(), name: "Gujarati (ગુજરાતી)".to_owned(), index: 53}),
            // Arc::new(LangItem { id: 54, code: "gv".to_owned(), name: "Manx (Gaelg, Gailck)".to_owned(), index: 54}),
            // Arc::new(LangItem { id: 55, code: "ha".to_owned(), name: "Hausa ((Hausa) هَوُسَ)".to_owned(), index: 55}),
            // Arc::new(LangItem { id: 56, code: "he".to_owned(), name: "Hebrew (modern) (עברית)".to_owned(), index: 56}),
            // Arc::new(LangItem { id: 57, code: "hi".to_owned(), name: "Hindi (हिन्दी, हिंदी)".to_owned(), index: 57}),
            // Arc::new(LangItem { id: 58, code: "ho".to_owned(), name: "Hiri Motu".to_owned(), index: 58}),
            // Arc::new(LangItem { id: 59, code: "hr".to_owned(), name: "Croatian (hrvatski jezik)".to_owned(), index: 59}),
            // Arc::new(LangItem { id: 60, code: "ht".to_owned(), name: "Haitian, Haitian Creole (Kreyòl ayisyen)".to_owned(), index: 60}),
            // Arc::new(LangItem { id: 61, code: "hu".to_owned(), name: "Hungarian (magyar)".to_owned(), index: 61}),
            // Arc::new(LangItem { id: 62, code: "hy".to_owned(), name: "Armenian (Հայերեն)".to_owned(), index: 62}),
            // Arc::new(LangItem { id: 63, code: "hz".to_owned(), name: "Herero (Otjiherero)".to_owned(), index: 63}),
            // Arc::new(LangItem { id: 64, code: "ia".to_owned(), name: "Interlingua".to_owned(), index: 64}),
            // Arc::new(LangItem { id: 65, code: "id".to_owned(), name: "Indonesian (Bahasa Indonesia)".to_owned(), index: 65}),
            // Arc::new(LangItem { id: 66, code: "ie".to_owned(), name: "Interlingue (Originally called Occidental; then Interlingue after WWII)".to_owned(), index: 66}),
            // Arc::new(LangItem { id: 67, code: "ig".to_owned(), name: "Igbo (Asụsụ Igbo)".to_owned(), index: 67}),
            // Arc::new(LangItem { id: 68, code: "ii".to_owned(), name: "Nuosu (ꆈꌠ꒿ Nuosuhxop)".to_owned(), index: 68}),
            // Arc::new(LangItem { id: 69, code: "ik".to_owned(), name: "Inupiaq (Iñupiaq, Iñupiatun)".to_owned(), index: 69}),
            // Arc::new(LangItem { id: 70, code: "io".to_owned(), name: "Ido".to_owned(), index: 70}),
            // Arc::new(LangItem { id: 71, code: "is".to_owned(), name: "Icelandic (Íslenska)".to_owned(), index: 71}),
            // Arc::new(LangItem { id: 72, code: "it".to_owned(), name: "Italian (Italiano)".to_owned(), index: 72}),
            // Arc::new(LangItem { id: 73, code: "iu".to_owned(), name: "Inuktitut (ᐃᓄᒃᑎᑐᑦ)".to_owned(), index: 73}),
            // Arc::new(LangItem { id: 74, code: "ja".to_owned(), name: "Japanese (日本語 (にほんご))".to_owned(), index: 74}),
            // Arc::new(LangItem { id: 75, code: "jv".to_owned(), name: "Javanese (ꦧꦱꦗꦮ, Basa Jawa)".to_owned(), index: 75}),
            // Arc::new(LangItem { id: 76, code: "ka".to_owned(), name: "Georgian (ქართული)".to_owned(), index: 76}),
            // Arc::new(LangItem { id: 77, code: "kg".to_owned(), name: "Kongo (Kikongo)".to_owned(), index: 77}),
            // Arc::new(LangItem { id: 78, code: "ki".to_owned(), name: "Kikuyu, Gikuyu (Gĩkũyũ)".to_owned(), index: 78}),
            // Arc::new(LangItem { id: 79, code: "kj".to_owned(), name: "Kwanyama, Kuanyama (Kuanyama)".to_owned(), index: 79}),
            // Arc::new(LangItem { id: 80, code: "kk".to_owned(), name: "Kazakh (қазақ тілі)".to_owned(), index: 80}),
            // Arc::new(LangItem { id: 81, code: "kl".to_owned(), name: "Kalaallisut, Greenlandic (kalaallisut, kalaallit oqaasii)".to_owned(), index: 81}),
            // Arc::new(LangItem { id: 82, code: "km".to_owned(), name: "Khmer (ខ្មែរ, ខេមរភាសា, ភាសាខ្មែរ)".to_owned(), index: 82}),
            // Arc::new(LangItem { id: 83, code: "kn".to_owned(), name: "Kannada (ಕನ್ನಡ)".to_owned(), index: 83}),
            // Arc::new(LangItem { id: 84, code: "ko".to_owned(), name: "Korean (한국어)".to_owned(), index: 84}),
            // Arc::new(LangItem { id: 85, code: "kr".to_owned(), name: "Kanuri".to_owned(), index: 85}),
            // Arc::new(LangItem { id: 86, code: "ks".to_owned(), name: "Kashmiri (कश्मीरी, کشمیری)".to_owned(), index: 86}),
            // Arc::new(LangItem { id: 87, code: "ku".to_owned(), name: "Kurdish (Kurdî, كوردی)".to_owned(), index: 87}),
            // Arc::new(LangItem { id: 88, code: "kv".to_owned(), name: "Komi (коми кыв)".to_owned(), index: 88}),
            // Arc::new(LangItem { id: 89, code: "kw".to_owned(), name: "Cornish (Kernewek)".to_owned(), index: 89}),
            // Arc::new(LangItem { id: 90, code: "ky".to_owned(), name: "Kyrgyz (Кыргызча, Кыргыз тили)".to_owned(), index: 90}),
            // Arc::new(LangItem { id: 91, code: "la".to_owned(), name: "Latin (latine, lingua latina)".to_owned(), index: 91}),
            // Arc::new(LangItem { id: 92, code: "lb".to_owned(), name: "Luxembourgish, Letzeburgesch (Lëtzebuergesch)".to_owned(), index: 92}),
            // Arc::new(LangItem { id: 93, code: "lg".to_owned(), name: "Ganda (Luganda)".to_owned(), index: 93}),
            // Arc::new(LangItem { id: 94, code: "li".to_owned(), name: "Limburgish, Limburgan, Limburger (Limburgs)".to_owned(), index: 94}),
            // Arc::new(LangItem { id: 95, code: "ln".to_owned(), name: "Lingala (Lingála)".to_owned(), index: 95}),
            // Arc::new(LangItem { id: 96, code: "lo".to_owned(), name: "Lao (ພາສາລາວ)".to_owned(), index: 96}),
            // Arc::new(LangItem { id: 97, code: "lt".to_owned(), name: "Lithuanian (lietuvių kalba)".to_owned(), index: 97}),
            // Arc::new(LangItem { id: 98, code: "lu".to_owned(), name: "Luba-Katanga (Tshiluba)".to_owned(), index: 98}),
            // Arc::new(LangItem { id: 99, code: "lv".to_owned(), name: "Latvian (latviešu valoda)".to_owned(), index: 99}),
            // Arc::new(LangItem { id: 100, code: "mg".to_owned(), name: "Malagasy (fiteny malagasy)".to_owned(), index: 100}),
            // Arc::new(LangItem { id: 101, code: "mh".to_owned(), name: "Marshallese (Kajin M̧ajeļ)".to_owned(), index: 101}),
            // Arc::new(LangItem { id: 102, code: "mi".to_owned(), name: "Māori (te reo Māori)".to_owned(), index: 102}),
            // Arc::new(LangItem { id: 103, code: "mk".to_owned(), name: "Macedonian (македонски јазик)".to_owned(), index: 103}),
            // Arc::new(LangItem { id: 104, code: "ml".to_owned(), name: "Malayalam (മലയാളം)".to_owned(), index: 104}),
            // Arc::new(LangItem { id: 105, code: "mn".to_owned(), name: "Mongolian (Монгол хэл)".to_owned(), index: 105}),
            // Arc::new(LangItem { id: 106, code: "mr".to_owned(), name: "Marathi (Marāṭhī) (मराठी)".to_owned(), index: 106}),
            // Arc::new(LangItem { id: 107, code: "ms".to_owned(), name: "Malay (bahasa Melayu, بهاس ملايو)".to_owned(), index: 107}),
            // Arc::new(LangItem { id: 108, code: "mt".to_owned(), name: "Maltese (Malti)".to_owned(), index: 108}),
            // Arc::new(LangItem { id: 109, code: "my".to_owned(), name: "Burmese (ဗမာစာ)".to_owned(), index: 109}),
            // Arc::new(LangItem { id: 110, code: "na".to_owned(), name: "Nauruan (Dorerin Naoero)".to_owned(), index: 110}),
            // Arc::new(LangItem { id: 111, code: "nb".to_owned(), name: "Norwegian Bokmål (Norsk bokmål)".to_owned(), index: 111}),
            // Arc::new(LangItem { id: 112, code: "nd".to_owned(), name: "Northern Ndebele (isiNdebele)".to_owned(), index: 112}),
            // Arc::new(LangItem { id: 113, code: "ne".to_owned(), name: "Nepali (नेपाली)".to_owned(), index: 113}),
            // Arc::new(LangItem { id: 114, code: "ng".to_owned(), name: "Ndonga (Owambo)".to_owned(), index: 114}),
            // Arc::new(LangItem { id: 115, code: "nl".to_owned(), name: "Dutch (Nederlands, Vlaams)".to_owned(), index: 115}),
            // Arc::new(LangItem { id: 116, code: "nn".to_owned(), name: "Norwegian Nynorsk (Norsk nynorsk)".to_owned(), index: 116}),
            // Arc::new(LangItem { id: 117, code: "no".to_owned(), name: "Norwegian (Norsk)".to_owned(), index: 117}),
            // Arc::new(LangItem { id: 118, code: "nr".to_owned(), name: "Southern Ndebele (isiNdebele)".to_owned(), index: 118}),
            // Arc::new(LangItem { id: 119, code: "nv".to_owned(), name: "Navajo, Navaho (Diné bizaad)".to_owned(), index: 119}),
            // Arc::new(LangItem { id: 120, code: "ny".to_owned(), name: "Chichewa, Chewa, Nyanja (chiCheŵa, chinyanja)".to_owned(), index: 120}),
            // Arc::new(LangItem { id: 121, code: "oc".to_owned(), name: "Occitan (occitan, lenga d'òc)".to_owned(), index: 121}),
            // Arc::new(LangItem { id: 122, code: "oj".to_owned(), name: "Ojibwe, Ojibwa (ᐊᓂᔑᓈᐯᒧᐎᓐ)".to_owned(), index: 122}),
            // Arc::new(LangItem { id: 123, code: "om".to_owned(), name: "Oromo (Afaan Oromoo)".to_owned(), index: 123}),
            // Arc::new(LangItem { id: 124, code: "or".to_owned(), name: "Oriya (ଓଡ଼ିଆ)".to_owned(), index: 124}),
            // Arc::new(LangItem { id: 125, code: "os".to_owned(), name: "Ossetian, Ossetic (ирон æвзаг)".to_owned(), index: 125}),
            // Arc::new(LangItem { id: 126, code: "pa".to_owned(), name: "(Eastern) Punjabi (ਪੰਜਾਬੀ)".to_owned(), index: 126}),
            // Arc::new(LangItem { id: 127, code: "pi".to_owned(), name: "Pāli (पाऴि)".to_owned(), index: 127}),
            // Arc::new(LangItem { id: 128, code: "pl".to_owned(), name: "Polish (język polski, polszczyzna)".to_owned(), index: 128}),
            // Arc::new(LangItem { id: 129, code: "ps".to_owned(), name: "Pashto, Pushto (پښتو)".to_owned(), index: 129}),
            // Arc::new(LangItem { id: 130, code: "pt".to_owned(), name: "Portuguese (Português)".to_owned(), index: 130}),
            // Arc::new(LangItem { id: 131, code: "qu".to_owned(), name: "Quechua (Runa Simi, Kichwa)".to_owned(), index: 131}),
            // Arc::new(LangItem { id: 132, code: "rm".to_owned(), name: "Romansh (rumantsch grischun)".to_owned(), index: 132}),
            // Arc::new(LangItem { id: 133, code: "rn".to_owned(), name: "Kirundi (Ikirundi)".to_owned(), index: 133}),
            // Arc::new(LangItem { id: 134, code: "ro".to_owned(), name: "Romanian (Română)".to_owned(), index: 134}),
            // Arc::new(LangItem { id: 135, code: "rw".to_owned(), name: "Kinyarwanda (Ikinyarwanda)".to_owned(), index: 135}),
            // Arc::new(LangItem { id: 136, code: "sa".to_owned(), name: "Sanskrit (Saṁskṛta) (संस्कृतम्)".to_owned(), index: 136}),
            // Arc::new(LangItem { id: 137, code: "sc".to_owned(), name: "Sardinian (sardu)".to_owned(), index: 137}),
            // Arc::new(LangItem { id: 138, code: "sd".to_owned(), name: "Sindhi (सिन्धी, سنڌي، سندھی)".to_owned(), index: 138}),
            // Arc::new(LangItem { id: 139, code: "se".to_owned(), name: "Northern Sami (Davvisámegiella)".to_owned(), index: 139}),
            // Arc::new(LangItem { id: 140, code: "sg".to_owned(), name: "Sango (yângâ tî sängö)".to_owned(), index: 140}),
            // Arc::new(LangItem { id: 141, code: "si".to_owned(), name: "Sinhalese, Sinhala (සිංහල)".to_owned(), index: 141}),
            // Arc::new(LangItem { id: 142, code: "sk".to_owned(), name: "Slovak (slovenčina, slovenský jazyk)".to_owned(), index: 142}),
            // Arc::new(LangItem { id: 143, code: "sl".to_owned(), name: "Slovene (slovenski jezik, slovenščina)".to_owned(), index: 143}),
            // Arc::new(LangItem { id: 144, code: "sm".to_owned(), name: "Samoan (gagana fa'a Samoa)".to_owned(), index: 144}),
            // Arc::new(LangItem { id: 145, code: "sn".to_owned(), name: "Shona (chiShona)".to_owned(), index: 145}),
            // Arc::new(LangItem { id: 146, code: "so".to_owned(), name: "Somali (Soomaaliga, af Soomaali)".to_owned(), index: 146}),
            // Arc::new(LangItem { id: 147, code: "sq".to_owned(), name: "Albanian (Shqip)".to_owned(), index: 147}),
            // Arc::new(LangItem { id: 148, code: "sr".to_owned(), name: "Serbian (српски језик)".to_owned(), index: 148}),
            // Arc::new(LangItem { id: 149, code: "ss".to_owned(), name: "Swati (SiSwati)".to_owned(), index: 149}),
            // Arc::new(LangItem { id: 150, code: "st".to_owned(), name: "Southern Sotho (Sesotho)".to_owned(), index: 150}),
            // Arc::new(LangItem { id: 151, code: "su".to_owned(), name: "Sundanese (Basa Sunda)".to_owned(), index: 151}),
            // Arc::new(LangItem { id: 152, code: "sv".to_owned(), name: "Swedish (svenska)".to_owned(), index: 152}),
            // Arc::new(LangItem { id: 153, code: "sw".to_owned(), name: "Swahili (Kiswahili)".to_owned(), index: 153}),
            // Arc::new(LangItem { id: 154, code: "ta".to_owned(), name: "Tamil (தமிழ்)".to_owned(), index: 154}),
            // Arc::new(LangItem { id: 155, code: "te".to_owned(), name: "Telugu (తెలుగు)".to_owned(), index: 155}),
            // Arc::new(LangItem { id: 156, code: "tg".to_owned(), name: "Tajik (тоҷикӣ, toçikī, تاجیکی)".to_owned(), index: 156}),
            // Arc::new(LangItem { id: 157, code: "th".to_owned(), name: "Thai (ไทย)".to_owned(), index: 157}),
            // Arc::new(LangItem { id: 158, code: "ti".to_owned(), name: "Tigrinya (ትግርኛ)".to_owned(), index: 158}),
            // Arc::new(LangItem { id: 159, code: "tk".to_owned(), name: "Turkmen (Türkmen, Түркмен)".to_owned(), index: 159}),
            // Arc::new(LangItem { id: 160, code: "tl".to_owned(), name: "Tagalog (Wikang Tagalog)".to_owned(), index: 160}),
            // Arc::new(LangItem { id: 161, code: "tn".to_owned(), name: "Tswana (Setswana)".to_owned(), index: 161}),
            // Arc::new(LangItem { id: 162, code: "to".to_owned(), name: "Tonga (Tonga Islands) (faka Tonga)".to_owned(), index: 162}),
            // Arc::new(LangItem { id: 163, code: "tr".to_owned(), name: "Turkish (Türkçe)".to_owned(), index: 163}),
            // Arc::new(LangItem { id: 164, code: "ts".to_owned(), name: "Tsonga (Xitsonga)".to_owned(), index: 164}),
            // Arc::new(LangItem { id: 165, code: "tt".to_owned(), name: "Tatar (татар теле, tatar tele)".to_owned(), index: 165}),
            // Arc::new(LangItem { id: 166, code: "tw".to_owned(), name: "Twi".to_owned(), index: 166}),
            // Arc::new(LangItem { id: 167, code: "ty".to_owned(), name: "Tahitian (Reo Tahiti)".to_owned(), index: 167}),
            // Arc::new(LangItem { id: 168, code: "ug".to_owned(), name: "Uyghur (ئۇيغۇرچە, Uyghurche)".to_owned(), index: 168}),
            // Arc::new(LangItem { id: 169, code: "ur".to_owned(), name: "Urdu (اردو)".to_owned(), index: 169}),
            // Arc::new(LangItem { id: 170, code: "uz".to_owned(), name: "Uzbek (Oʻzbek, Ўзбек, أۇزبېك)".to_owned(), index: 170}),
            // Arc::new(LangItem { id: 171, code: "ve".to_owned(), name: "Venda (Tshivenḓa)".to_owned(), index: 171}),
            // Arc::new(LangItem { id: 172, code: "vi".to_owned(), name: "Vietnamese (Tiếng Việt)".to_owned(), index: 172}),
            // Arc::new(LangItem { id: 173, code: "vo".to_owned(), name: "Volapük".to_owned(), index: 173}),
            // Arc::new(LangItem { id: 174, code: "wa".to_owned(), name: "Walloon (walon)".to_owned(), index: 174}),
            // Arc::new(LangItem { id: 175, code: "wo".to_owned(), name: "Wolof (Wollof)".to_owned(), index: 175}),
            // Arc::new(LangItem { id: 176, code: "xh".to_owned(), name: "Xhosa (isiXhosa)".to_owned(), index: 176}),
            // Arc::new(LangItem { id: 177, code: "yi".to_owned(), name: "Yiddish (ייִדיש)".to_owned(), index: 177}),
            // Arc::new(LangItem { id: 178, code: "yo".to_owned(), name: "Yoruba (Yorùbá)".to_owned(), index: 178}),
            // Arc::new(LangItem { id: 179, code: "za".to_owned(), name: "Zhuang, Chuang (Saɯ cueŋƅ, Saw cuengh)".to_owned(), index: 179}),
            // Arc::new(LangItem { id: 180, code: "zh".to_owned(), name: "Chinese (中文 (Zhōngwén), 汉语, 漢語)".to_owned(), index: 180}),
            // Arc::new(LangItem { id: 181, code: "zu".to_owned(), name: "Zulu (isiZulu)".to_owned(), index: 181}),
        ]
    }
}
