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
                root,
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
            root,
            codes,
        };
        lang.load(files).await;
        lang
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
                let lang = if let Data::String(val) = unsafe { row.get_unchecked(1) } {
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
                vec.push(Arc::new(LangItem { id, lang, name, index }));
            } else {
                Log::warning(1150, None);
                return Vec::new();
            }
        }
        vec
    }

    /// Get list of enabled langs for install module
    fn get_langs_install(files: &Vec<(PathBuf, String, String, String)>) -> Vec<Arc<LangItem>> {
        let mut list = BTreeMap::new();
        list.insert(m_fnv1a_64!("en"), (0, "English".to_string()));
        list.insert(m_fnv1a_64!("uk"), (1, "Ukrainian (Українська)".to_string()));
        list.insert(m_fnv1a_64!("aa"), (2, "Afar (Afaraf)".to_string()));
        list.insert(m_fnv1a_64!("ab"), (3, "Abkhaz (аҧсуа бызшәа, аҧсшәа)".to_string()));
        list.insert(m_fnv1a_64!("ae"), (4, "Avestan (avesta)".to_string()));
        list.insert(m_fnv1a_64!("af"), (5, "Afrikaans".to_string()));
        list.insert(m_fnv1a_64!("ak"), (6, "Akan".to_string()));
        list.insert(m_fnv1a_64!("am"), (7, "Amharic (አማርኛ)".to_string()));
        list.insert(m_fnv1a_64!("an"), (8, "Aragonese (aragonés)".to_string()));
        list.insert(m_fnv1a_64!("ar"), (9, "Arabic (العربية)".to_string()));
        list.insert(m_fnv1a_64!("as"), (10, "Assamese (অসমীয়া)".to_string()));
        list.insert(m_fnv1a_64!("av"), (11, "Avaric (авар мацӀ, магӀарул мацӀ)".to_string()));
        list.insert(m_fnv1a_64!("ay"), (12, "Aymara (aymar aru)".to_string()));
        list.insert(m_fnv1a_64!("az"), (13, "Azerbaijani (azərbaycan dili)".to_string()));
        list.insert(m_fnv1a_64!("ba"), (14, "Bashkir (башҡорт теле)".to_string()));
        list.insert(m_fnv1a_64!("bg"), (15, "Bulgarian (български език)".to_string()));
        list.insert(m_fnv1a_64!("bh"), (16, "Bihari (भोजपुरी)".to_string()));
        list.insert(m_fnv1a_64!("bi"), (17, "Bislama".to_string()));
        list.insert(m_fnv1a_64!("bm"), (18, "Bambara (bamanankan)".to_string()));
        list.insert(m_fnv1a_64!("bn"), (19, "Bengali, Bangla (বাংলা)".to_string()));
        list.insert(m_fnv1a_64!("bo"), (20, "Tibetan Standard, Tibetan, Central (བོད་ཡིག)".to_string()));
        list.insert(m_fnv1a_64!("br"), (21, "Breton (brezhoneg)".to_string()));
        list.insert(m_fnv1a_64!("bs"), (22, "Bosnian (bosanski jezik)".to_string()));
        list.insert(m_fnv1a_64!("ca"), (23, "Catalan (català)".to_string()));
        list.insert(m_fnv1a_64!("ce"), (24, "Chechen (нохчийн мотт)".to_string()));
        list.insert(m_fnv1a_64!("ch"), (25, "Chamorro (Chamoru)".to_string()));
        list.insert(m_fnv1a_64!("co"), (26, "Corsican (corsu, lingua corsa)".to_string()));
        list.insert(m_fnv1a_64!("cr"), (27, "Cree (ᓀᐦᐃᔭᐍᐏᐣ)".to_string()));
        list.insert(m_fnv1a_64!("cs"), (28, "Czech (čeština, český jazyk)".to_string()));
        list.insert(m_fnv1a_64!("cu"), (29, "Old Church Slavonic, Church Slavonic, Old Bulgarian (ѩзыкъ словѣньскъ)".to_string()));
        list.insert(m_fnv1a_64!("cv"), (30, "Chuvash (чӑваш чӗлхи)".to_string()));
        list.insert(m_fnv1a_64!("cy"), (31, "Welsh (Cymraeg)".to_string()));
        list.insert(m_fnv1a_64!("da"), (32, "Danish (dansk)".to_string()));
        list.insert(m_fnv1a_64!("de"), (33, "German (Deutsch)".to_string()));
        list.insert(m_fnv1a_64!("dv"), (34, "Divehi, Dhivehi, Maldivian (ދިވެހި)".to_string()));
        list.insert(m_fnv1a_64!("dz"), (35, "Dzongkha (རྫོང་ཁ)".to_string()));
        list.insert(m_fnv1a_64!("ee"), (36, "Ewe (Eʋegbe)".to_string()));
        list.insert(m_fnv1a_64!("el"), (37, "Greek (modern) (ελληνικά)".to_string()));
        list.insert(m_fnv1a_64!("eo"), (38, "Esperanto".to_string()));
        list.insert(m_fnv1a_64!("es"), (39, "Spanish (Español)".to_string()));
        list.insert(m_fnv1a_64!("et"), (40, "Estonian (eesti, eesti keel)".to_string()));
        list.insert(m_fnv1a_64!("eu"), (41, "Basque (euskara, euskera)".to_string()));
        list.insert(m_fnv1a_64!("fa"), (42, "Persian (Farsi) (فارسی)".to_string()));
        list.insert(m_fnv1a_64!("ff"), (43, "Fula, Fulah, Pulaar, Pular (Fulfulde, Pulaar, Pular)".to_string()));
        list.insert(m_fnv1a_64!("fi"), (44, "Finnish (suomi, suomen kieli)".to_string()));
        list.insert(m_fnv1a_64!("fj"), (45, "Fijian (vosa Vakaviti)".to_string()));
        list.insert(m_fnv1a_64!("fo"), (46, "Faroese (føroyskt)".to_string()));
        list.insert(m_fnv1a_64!("fr"), (47, "French (français, langue française)".to_string()));
        list.insert(m_fnv1a_64!("fy"), (48, "Western Frisian (Frysk)".to_string()));
        list.insert(m_fnv1a_64!("ga"), (49, "Irish (Gaeilge)".to_string()));
        list.insert(m_fnv1a_64!("gd"), (50, "Scottish Gaelic, Gaelic (Gàidhlig)".to_string()));
        list.insert(m_fnv1a_64!("gl"), (51, "Galician (galego)".to_string()));
        list.insert(m_fnv1a_64!("gn"), (52, "Guaraní (Avañe'ẽ)".to_string()));
        list.insert(m_fnv1a_64!("gu"), (53, "Gujarati (ગુજરાતી)".to_string()));
        list.insert(m_fnv1a_64!("gv"), (54, "Manx (Gaelg, Gailck)".to_string()));
        list.insert(m_fnv1a_64!("ha"), (55, "Hausa ((Hausa) هَوُسَ)".to_string()));
        list.insert(m_fnv1a_64!("he"), (56, "Hebrew (modern) (עברית)".to_string()));
        list.insert(m_fnv1a_64!("hi"), (57, "Hindi (हिन्दी, हिंदी)".to_string()));
        list.insert(m_fnv1a_64!("ho"), (58, "Hiri Motu".to_string()));
        list.insert(m_fnv1a_64!("hr"), (59, "Croatian (hrvatski jezik)".to_string()));
        list.insert(m_fnv1a_64!("ht"), (60, "Haitian, Haitian Creole (Kreyòl ayisyen)".to_string()));
        list.insert(m_fnv1a_64!("hu"), (61, "Hungarian (magyar)".to_string()));
        list.insert(m_fnv1a_64!("hy"), (62, "Armenian (Հայերեն)".to_string()));
        list.insert(m_fnv1a_64!("hz"), (63, "Herero (Otjiherero)".to_string()));
        list.insert(m_fnv1a_64!("ia"), (64, "Interlingua".to_string()));
        list.insert(m_fnv1a_64!("id"), (65, "Indonesian (Bahasa Indonesia)".to_string()));
        list.insert(m_fnv1a_64!("ie"), (66, "Interlingue (Originally called Occidental; then Interlingue after WWII)".to_string()));
        list.insert(m_fnv1a_64!("ig"), (67, "Igbo (Asụsụ Igbo)".to_string()));
        list.insert(m_fnv1a_64!("ii"), (68, "Nuosu (ꆈꌠ꒿ Nuosuhxop)".to_string()));
        list.insert(m_fnv1a_64!("ik"), (69, "Inupiaq (Iñupiaq, Iñupiatun)".to_string()));
        list.insert(m_fnv1a_64!("io"), (70, "Ido".to_string()));
        list.insert(m_fnv1a_64!("is"), (71, "Icelandic (Íslenska)".to_string()));
        list.insert(m_fnv1a_64!("it"), (72, "Italian (Italiano)".to_string()));
        list.insert(m_fnv1a_64!("iu"), (73, "Inuktitut (ᐃᓄᒃᑎᑐᑦ)".to_string()));
        list.insert(m_fnv1a_64!("ja"), (74, "Japanese (日本語 (にほんご))".to_string()));
        list.insert(m_fnv1a_64!("jv"), (75, "Javanese (ꦧꦱꦗꦮ, Basa Jawa)".to_string()));
        list.insert(m_fnv1a_64!("ka"), (76, "Georgian (ქართული)".to_string()));
        list.insert(m_fnv1a_64!("kg"), (77, "Kongo (Kikongo)".to_string()));
        list.insert(m_fnv1a_64!("ki"), (78, "Kikuyu, Gikuyu (Gĩkũyũ)".to_string()));
        list.insert(m_fnv1a_64!("kj"), (79, "Kwanyama, Kuanyama (Kuanyama)".to_string()));
        list.insert(m_fnv1a_64!("kk"), (80, "Kazakh (қазақ тілі)".to_string()));
        list.insert(m_fnv1a_64!("kl"), (81, "Kalaallisut, Greenlandic (kalaallisut, kalaallit oqaasii)".to_string()));
        list.insert(m_fnv1a_64!("km"), (82, "Khmer (ខ្មែរ, ខេមរភាសា, ភាសាខ្មែរ)".to_string()));
        list.insert(m_fnv1a_64!("kn"), (83, "Kannada (ಕನ್ನಡ)".to_string()));
        list.insert(m_fnv1a_64!("ko"), (84, "Korean (한국어)".to_string()));
        list.insert(m_fnv1a_64!("kr"), (85, "Kanuri".to_string()));
        list.insert(m_fnv1a_64!("ks"), (86, "Kashmiri (कश्मीरी, کشمیری)".to_string()));
        list.insert(m_fnv1a_64!("ku"), (87, "Kurdish (Kurdî, كوردی)".to_string()));
        list.insert(m_fnv1a_64!("kv"), (88, "Komi (коми кыв)".to_string()));
        list.insert(m_fnv1a_64!("kw"), (89, "Cornish (Kernewek)".to_string()));
        list.insert(m_fnv1a_64!("ky"), (90, "Kyrgyz (Кыргызча, Кыргыз тили)".to_string()));
        list.insert(m_fnv1a_64!("la"), (91, "Latin (latine, lingua latina)".to_string()));
        list.insert(m_fnv1a_64!("lb"), (92, "Luxembourgish, Letzeburgesch (Lëtzebuergesch)".to_string()));
        list.insert(m_fnv1a_64!("lg"), (93, "Ganda (Luganda)".to_string()));
        list.insert(m_fnv1a_64!("li"), (94, "Limburgish, Limburgan, Limburger (Limburgs)".to_string()));
        list.insert(m_fnv1a_64!("ln"), (95, "Lingala (Lingála)".to_string()));
        list.insert(m_fnv1a_64!("lo"), (96, "Lao (ພາສາລາວ)".to_string()));
        list.insert(m_fnv1a_64!("lt"), (97, "Lithuanian (lietuvių kalba)".to_string()));
        list.insert(m_fnv1a_64!("lu"), (98, "Luba-Katanga (Tshiluba)".to_string()));
        list.insert(m_fnv1a_64!("lv"), (99, "Latvian (latviešu valoda)".to_string()));
        list.insert(m_fnv1a_64!("mg"), (100, "Malagasy (fiteny malagasy)".to_string()));
        list.insert(m_fnv1a_64!("mh"), (101, "Marshallese (Kajin M̧ajeļ)".to_string()));
        list.insert(m_fnv1a_64!("mi"), (102, "Māori (te reo Māori)".to_string()));
        list.insert(m_fnv1a_64!("mk"), (103, "Macedonian (македонски јазик)".to_string()));
        list.insert(m_fnv1a_64!("ml"), (104, "Malayalam (മലയാളം)".to_string()));
        list.insert(m_fnv1a_64!("mn"), (105, "Mongolian (Монгол хэл)".to_string()));
        list.insert(m_fnv1a_64!("mr"), (106, "Marathi (Marāṭhī) (मराठी)".to_string()));
        list.insert(m_fnv1a_64!("ms"), (107, "Malay (bahasa Melayu, بهاس ملايو)".to_string()));
        list.insert(m_fnv1a_64!("mt"), (108, "Maltese (Malti)".to_string()));
        list.insert(m_fnv1a_64!("my"), (109, "Burmese (ဗမာစာ)".to_string()));
        list.insert(m_fnv1a_64!("na"), (110, "Nauruan (Dorerin Naoero)".to_string()));
        list.insert(m_fnv1a_64!("nb"), (111, "Norwegian Bokmål (Norsk bokmål)".to_string()));
        list.insert(m_fnv1a_64!("nd"), (112, "Northern Ndebele (isiNdebele)".to_string()));
        list.insert(m_fnv1a_64!("ne"), (113, "Nepali (नेपाली)".to_string()));
        list.insert(m_fnv1a_64!("ng"), (114, "Ndonga (Owambo)".to_string()));
        list.insert(m_fnv1a_64!("nl"), (115, "Dutch (Nederlands, Vlaams)".to_string()));
        list.insert(m_fnv1a_64!("nn"), (116, "Norwegian Nynorsk (Norsk nynorsk)".to_string()));
        list.insert(m_fnv1a_64!("no"), (117, "Norwegian (Norsk)".to_string()));
        list.insert(m_fnv1a_64!("nr"), (118, "Southern Ndebele (isiNdebele)".to_string()));
        list.insert(m_fnv1a_64!("nv"), (119, "Navajo, Navaho (Diné bizaad)".to_string()));
        list.insert(m_fnv1a_64!("ny"), (120, "Chichewa, Chewa, Nyanja (chiCheŵa, chinyanja)".to_string()));
        list.insert(m_fnv1a_64!("oc"), (121, "Occitan (occitan, lenga d'òc)".to_string()));
        list.insert(m_fnv1a_64!("oj"), (122, "Ojibwe, Ojibwa (ᐊᓂᔑᓈᐯᒧᐎᓐ)".to_string()));
        list.insert(m_fnv1a_64!("om"), (123, "Oromo (Afaan Oromoo)".to_string()));
        list.insert(m_fnv1a_64!("or"), (124, "Oriya (ଓଡ଼ିଆ)".to_string()));
        list.insert(m_fnv1a_64!("os"), (125, "Ossetian, Ossetic (ирон æвзаг)".to_string()));
        list.insert(m_fnv1a_64!("pa"), (126, "(Eastern) Punjabi (ਪੰਜਾਬੀ)".to_string()));
        list.insert(m_fnv1a_64!("pi"), (127, "Pāli (पाऴि)".to_string()));
        list.insert(m_fnv1a_64!("pl"), (128, "Polish (język polski, polszczyzna)".to_string()));
        list.insert(m_fnv1a_64!("ps"), (129, "Pashto, Pushto (پښتو)".to_string()));
        list.insert(m_fnv1a_64!("pt"), (130, "Portuguese (Português)".to_string()));
        list.insert(m_fnv1a_64!("qu"), (131, "Quechua (Runa Simi, Kichwa)".to_string()));
        list.insert(m_fnv1a_64!("rm"), (132, "Romansh (rumantsch grischun)".to_string()));
        list.insert(m_fnv1a_64!("rn"), (133, "Kirundi (Ikirundi)".to_string()));
        list.insert(m_fnv1a_64!("ro"), (134, "Romanian (Română)".to_string()));
        list.insert(m_fnv1a_64!("rw"), (135, "Kinyarwanda (Ikinyarwanda)".to_string()));
        list.insert(m_fnv1a_64!("sa"), (136, "Sanskrit (Saṁskṛta) (संस्कृतम्)".to_string()));
        list.insert(m_fnv1a_64!("sc"), (137, "Sardinian (sardu)".to_string()));
        list.insert(m_fnv1a_64!("sd"), (138, "Sindhi (सिन्धी, سنڌي، سندھی)".to_string()));
        list.insert(m_fnv1a_64!("se"), (139, "Northern Sami (Davvisámegiella)".to_string()));
        list.insert(m_fnv1a_64!("sg"), (140, "Sango (yângâ tî sängö)".to_string()));
        list.insert(m_fnv1a_64!("si"), (141, "Sinhalese, Sinhala (සිංහල)".to_string()));
        list.insert(m_fnv1a_64!("sk"), (142, "Slovak (slovenčina, slovenský jazyk)".to_string()));
        list.insert(m_fnv1a_64!("sl"), (143, "Slovene (slovenski jezik, slovenščina)".to_string()));
        list.insert(m_fnv1a_64!("sm"), (144, "Samoan (gagana fa'a Samoa)".to_string()));
        list.insert(m_fnv1a_64!("sn"), (145, "Shona (chiShona)".to_string()));
        list.insert(m_fnv1a_64!("so"), (146, "Somali (Soomaaliga, af Soomaali)".to_string()));
        list.insert(m_fnv1a_64!("sq"), (147, "Albanian (Shqip)".to_string()));
        list.insert(m_fnv1a_64!("sr"), (148, "Serbian (српски језик)".to_string()));
        list.insert(m_fnv1a_64!("ss"), (149, "Swati (SiSwati)".to_string()));
        list.insert(m_fnv1a_64!("st"), (150, "Southern Sotho (Sesotho)".to_string()));
        list.insert(m_fnv1a_64!("su"), (151, "Sundanese (Basa Sunda)".to_string()));
        list.insert(m_fnv1a_64!("sv"), (152, "Swedish (svenska)".to_string()));
        list.insert(m_fnv1a_64!("sw"), (153, "Swahili (Kiswahili)".to_string()));
        list.insert(m_fnv1a_64!("ta"), (154, "Tamil (தமிழ்)".to_string()));
        list.insert(m_fnv1a_64!("te"), (155, "Telugu (తెలుగు)".to_string()));
        list.insert(m_fnv1a_64!("tg"), (156, "Tajik (тоҷикӣ, toçikī, تاجیکی)".to_string()));
        list.insert(m_fnv1a_64!("th"), (157, "Thai (ไทย)".to_string()));
        list.insert(m_fnv1a_64!("ti"), (158, "Tigrinya (ትግርኛ)".to_string()));
        list.insert(m_fnv1a_64!("tk"), (159, "Turkmen (Türkmen, Түркмен)".to_string()));
        list.insert(m_fnv1a_64!("tl"), (160, "Tagalog (Wikang Tagalog)".to_string()));
        list.insert(m_fnv1a_64!("tn"), (161, "Tswana (Setswana)".to_string()));
        list.insert(m_fnv1a_64!("to"), (162, "Tonga (Tonga Islands) (faka Tonga)".to_string()));
        list.insert(m_fnv1a_64!("tr"), (163, "Turkish (Türkçe)".to_string()));
        list.insert(m_fnv1a_64!("ts"), (164, "Tsonga (Xitsonga)".to_string()));
        list.insert(m_fnv1a_64!("tt"), (165, "Tatar (татар теле, tatar tele)".to_string()));
        list.insert(m_fnv1a_64!("tw"), (166, "Twi".to_string()));
        list.insert(m_fnv1a_64!("ty"), (167, "Tahitian (Reo Tahiti)".to_string()));
        list.insert(m_fnv1a_64!("ug"), (168, "Uyghur (ئۇيغۇرچە, Uyghurche)".to_string()));
        list.insert(m_fnv1a_64!("ur"), (169, "Urdu (اردو)".to_string()));
        list.insert(m_fnv1a_64!("uz"), (170, "Uzbek (Oʻzbek, Ўзбек, أۇزبېك)".to_string()));
        list.insert(m_fnv1a_64!("ve"), (171, "Venda (Tshivenḓa)".to_string()));
        list.insert(m_fnv1a_64!("vi"), (172, "Vietnamese (Tiếng Việt)".to_string()));
        list.insert(m_fnv1a_64!("vo"), (173, "Volapük".to_string()));
        list.insert(m_fnv1a_64!("wa"), (174, "Walloon (walon)".to_string()));
        list.insert(m_fnv1a_64!("wo"), (175, "Wolof (Wollof)".to_string()));
        list.insert(m_fnv1a_64!("xh"), (176, "Xhosa (isiXhosa)".to_string()));
        list.insert(m_fnv1a_64!("yi"), (177, "Yiddish (ייִדיש)".to_string()));
        list.insert(m_fnv1a_64!("yo"), (178, "Yoruba (Yorùbá)".to_string()));
        list.insert(m_fnv1a_64!("za"), (179, "Zhuang, Chuang (Saɯ cueŋƅ, Saw cuengh)".to_string()));
        list.insert(m_fnv1a_64!("zh"), (180, "Chinese (中文 (Zhōngwén), 汉语, 漢語)".to_string()));
        list.insert(m_fnv1a_64!("zu"), (181, "Zulu (isiZulu)".to_string()));

        let mut vec = Vec::with_capacity(files.len());
        let mut index = 0;
        for (_, _, _, code) in files {
            if let Some((id, name)) = list.get(&fnv1a_64(code.as_bytes())) {
                vec.push(Arc::new(LangItem {
                    id: *id,
                    lang: code.to_owned(),
                    name: name.to_owned(),
                    index,
                }));
                index += 1;
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
