-- ----------------------------
-- Table structure for access
-- ----------------------------
CREATE TABLE "access" (
  "access_id" int8 NOT NULL GENERATED BY DEFAULT AS IDENTITY,
  "role_id" int8 NOT NULL,
  "access" bool NOT NULL,
  "controller_id" int8 NOT NULL
);
COMMENT ON COLUMN "access"."access_id" IS 'Identifier';
COMMENT ON COLUMN "access"."role_id" IS 'Role ID';
COMMENT ON COLUMN "access"."access" IS 'Access flag';
COMMENT ON COLUMN "access"."controller_id" IS 'Controller ID';
COMMENT ON TABLE "access" IS 'Access to controllers';

INSERT INTO "access" VALUES (1, 0, 't', 1);
INSERT INTO "access" VALUES (2, 0, 't', 4);
INSERT INTO "access" VALUES (3, 0, 't', 5);

-- ----------------------------
-- Table structure for controller
-- ----------------------------
CREATE TABLE "controller" (
  "controller_id" int8 NOT NULL GENERATED BY DEFAULT AS IDENTITY,
  "module" text NOT NULL,
  "class" text NOT NULL,
  "action" text NOT NULL,
  "description" jsonb NOT NULL,
  "module_id" int8 NOT NULL,
  "class_id" int8 NOT NULL,
  "action_id" int8 NOT NULL
);
COMMENT ON COLUMN "controller"."controller_id" IS 'Identifier';
COMMENT ON COLUMN "controller"."module" IS 'Module';
COMMENT ON COLUMN "controller"."class" IS 'Class';
COMMENT ON COLUMN "controller"."action" IS 'Action (controller)';
COMMENT ON COLUMN "controller"."description" IS 'Description';
COMMENT ON COLUMN "controller"."module_id" IS 'fnv1a_64 hash from module';
COMMENT ON COLUMN "controller"."class_id" IS 'fnv1a_64 hash from class';
COMMENT ON COLUMN "controller"."action_id" IS 'fnv1a_64 hash from action';
COMMENT ON TABLE "controller" IS 'Controllers list';

INSERT INTO "controller" VALUES (1, 'index', '', '', '{}', -8948777187306027381, -3750763034362895579, -3750763034362895579);
INSERT INTO "controller" VALUES (2, 'index', 'index', 'index', '{}', -8948777187306027381, -8948777187306027381, -8948777187306027381);
INSERT INTO "controller" VALUES (3, 'index', 'index', 'not_found', '{}', -8948777187306027381, -8948777187306027381, -1573091631220776463);
INSERT INTO "controller" VALUES (4, 'admin', 'index', '', '{}', -1887597591324883884, -8948777187306027381, -3750763034362895579);
INSERT INTO "controller" VALUES (5, 'admin', 'login', '', '{}', -1887597591324883884, 272289342528891346, -3750763034362895579);

-- ----------------------------
-- Table structure for lang
-- ----------------------------
CREATE TABLE "lang" (
  "lang_id" int8 NOT NULL GENERATED BY DEFAULT AS IDENTITY,
  "name" text NOT NULL,
  "enable" bool NOT NULL DEFAULT false,
  "code" text NOT NULL,
  "sort" int8 NOT NULL,
  "index" int8 NULL
);
COMMENT ON COLUMN "lang"."lang_id" IS 'Identifier';
COMMENT ON COLUMN "lang"."name" IS 'Language name';
COMMENT ON COLUMN "lang"."enable" IS 'Enable';
COMMENT ON COLUMN "lang"."code" IS 'ISO 639-1 : uk - ukrainian, en - english';
COMMENT ON COLUMN "lang"."sort" IS 'Sort order';
COMMENT ON COLUMN "lang"."index" IS 'Index in JSON type field db';
COMMENT ON TABLE "lang" IS 'Languages';

INSERT INTO "lang" VALUES (0, 'English', 'f', 'en', 0, null);
INSERT INTO "lang" VALUES (1, 'Ukrainian (Українська)', 'f', 'uk', 1, null);
INSERT INTO "lang" VALUES (2, 'Afar (Afaraf)', 'f', 'aa', 2, null);
INSERT INTO "lang" VALUES (3, 'Abkhaz (аҧсуа бызшәа, аҧсшәа)', 'f', 'ab', 3, null);
INSERT INTO "lang" VALUES (4, 'Avestan (avesta)', 'f', 'ae', 4, null);
INSERT INTO "lang" VALUES (5, 'Afrikaans', 'f', 'af', 5, null);
INSERT INTO "lang" VALUES (6, 'Akan', 'f', 'ak', 6, null);
INSERT INTO "lang" VALUES (7, 'Amharic (አማርኛ)', 'f', 'am', 7, null);
INSERT INTO "lang" VALUES (8, 'Aragonese (aragonés)', 'f', 'an', 8, null);
INSERT INTO "lang" VALUES (9, 'Arabic (العربية)', 'f', 'ar', 9, null);
INSERT INTO "lang" VALUES (10, 'Assamese (অসমীয়া)', 'f', 'as', 10, null);
INSERT INTO "lang" VALUES (11, 'Avaric (авар мацӀ, магӀарул мацӀ)', 'f', 'av', 11, null);
INSERT INTO "lang" VALUES (12, 'Aymara (aymar aru)', 'f', 'ay', 12, null);
INSERT INTO "lang" VALUES (13, 'Azerbaijani (azərbaycan dili)', 'f', 'az', 13, null);
INSERT INTO "lang" VALUES (14, 'Bashkir (башҡорт теле)', 'f', 'ba', 14, null);
INSERT INTO "lang" VALUES (15, 'Bulgarian (български език)', 'f', 'bg', 15, null);
INSERT INTO "lang" VALUES (16, 'Bihari (भोजपुरी)', 'f', 'bh', 16, null);
INSERT INTO "lang" VALUES (17, 'Bislama', 'f', 'bi', 17, null);
INSERT INTO "lang" VALUES (18, 'Bambara (bamanankan)', 'f', 'bm', 18, null);
INSERT INTO "lang" VALUES (19, 'Bengali, Bangla (বাংলা)', 'f', 'bn', 19, null);
INSERT INTO "lang" VALUES (20, 'Tibetan Standard, Tibetan, Central (བོད་ཡིག)', 'f', 'bo', 20, null);
INSERT INTO "lang" VALUES (21, 'Breton (brezhoneg)', 'f', 'br', 21, null);
INSERT INTO "lang" VALUES (22, 'Bosnian (bosanski jezik)', 'f', 'bs', 22, null);
INSERT INTO "lang" VALUES (23, 'Catalan (català)', 'f', 'ca', 23, null);
INSERT INTO "lang" VALUES (24, 'Chechen (нохчийн мотт)', 'f', 'ce', 24, null);
INSERT INTO "lang" VALUES (25, 'Chamorro (Chamoru)', 'f', 'ch', 25, null);
INSERT INTO "lang" VALUES (26, 'Corsican (corsu, lingua corsa)', 'f', 'co', 26, null);
INSERT INTO "lang" VALUES (27, 'Cree (ᓀᐦᐃᔭᐍᐏᐣ)', 'f', 'cr', 27, null);
INSERT INTO "lang" VALUES (28, 'Czech (čeština, český jazyk)', 'f', 'cs', 28, null);
INSERT INTO "lang" VALUES (29, 'Old Church Slavonic, Church Slavonic, Old Bulgarian (ѩзыкъ словѣньскъ)', 'f', 'cu', 29, null);
INSERT INTO "lang" VALUES (30, 'Chuvash (чӑваш чӗлхи)', 'f', 'cv', 30, null);
INSERT INTO "lang" VALUES (31, 'Welsh (Cymraeg)', 'f', 'cy', 31, null);
INSERT INTO "lang" VALUES (32, 'Danish (dansk)', 'f', 'da', 32, null);
INSERT INTO "lang" VALUES (33, 'German (Deutsch)', 'f', 'de', 33, null);
INSERT INTO "lang" VALUES (34, 'Divehi, Dhivehi, Maldivian (ދިވެހި)', 'f', 'dv', 34, null);
INSERT INTO "lang" VALUES (35, 'Dzongkha (རྫོང་ཁ)', 'f', 'dz', 35, null);
INSERT INTO "lang" VALUES (36, 'Ewe (Eʋegbe)', 'f', 'ee', 36, null);
INSERT INTO "lang" VALUES (37, 'Greek (modern) (ελληνικά)', 'f', 'el', 37, null);
INSERT INTO "lang" VALUES (38, 'Esperanto', 'f', 'eo', 38, null);
INSERT INTO "lang" VALUES (39, 'Spanish (Español)', 'f', 'es', 39, null);
INSERT INTO "lang" VALUES (40, 'Estonian (eesti, eesti keel)', 'f', 'et', 40, null);
INSERT INTO "lang" VALUES (41, 'Basque (euskara, euskera)', 'f', 'eu', 41, null);
INSERT INTO "lang" VALUES (42, 'Persian (Farsi) (فارسی)', 'f', 'fa', 42, null);
INSERT INTO "lang" VALUES (43, 'Fula, Fulah, Pulaar, Pular (Fulfulde, Pulaar, Pular)', 'f', 'ff', 43, null);
INSERT INTO "lang" VALUES (44, 'Finnish (suomi, suomen kieli)', 'f', 'fi', 44, null);
INSERT INTO "lang" VALUES (45, 'Fijian (vosa Vakaviti)', 'f', 'fj', 45, null);
INSERT INTO "lang" VALUES (46, 'Faroese (føroyskt)', 'f', 'fo', 46, null);
INSERT INTO "lang" VALUES (47, 'French (français, langue française)', 'f', 'fr', 47, null);
INSERT INTO "lang" VALUES (48, 'Western Frisian (Frysk)', 'f', 'fy', 48, null);
INSERT INTO "lang" VALUES (49, 'Irish (Gaeilge)', 'f', 'ga', 49, null);
INSERT INTO "lang" VALUES (50, 'Scottish Gaelic, Gaelic (Gàidhlig)', 'f', 'gd', 50, null);
INSERT INTO "lang" VALUES (51, 'Galician (galego)', 'f', 'gl', 51, null);
INSERT INTO "lang" VALUES (52, 'Guaraní (Avañe''ẽ)', 'f', 'gn', 52, null);
INSERT INTO "lang" VALUES (53, 'Gujarati (ગુજરાતી)', 'f', 'gu', 53, null);
INSERT INTO "lang" VALUES (54, 'Manx (Gaelg, Gailck)', 'f', 'gv', 54, null);
INSERT INTO "lang" VALUES (55, 'Hausa ((Hausa) هَوُسَ)', 'f', 'ha', 55, null);
INSERT INTO "lang" VALUES (56, 'Hebrew (modern) (עברית)', 'f', 'he', 56, null);
INSERT INTO "lang" VALUES (57, 'Hindi (हिन्दी, हिंदी)', 'f', 'hi', 57, null);
INSERT INTO "lang" VALUES (58, 'Hiri Motu', 'f', 'ho', 58, null);
INSERT INTO "lang" VALUES (59, 'Croatian (hrvatski jezik)', 'f', 'hr', 59, null);
INSERT INTO "lang" VALUES (60, 'Haitian, Haitian Creole (Kreyòl ayisyen)', 'f', 'ht', 60, null);
INSERT INTO "lang" VALUES (61, 'Hungarian (magyar)', 'f', 'hu', 61, null);
INSERT INTO "lang" VALUES (62, 'Armenian (Հայերեն)', 'f', 'hy', 62, null);
INSERT INTO "lang" VALUES (63, 'Herero (Otjiherero)', 'f', 'hz', 63, null);
INSERT INTO "lang" VALUES (64, 'Interlingua', 'f', 'ia', 64, null);
INSERT INTO "lang" VALUES (65, 'Indonesian (Bahasa Indonesia)', 'f', 'id', 65, null);
INSERT INTO "lang" VALUES (66, 'Interlingue (Originally called Occidental; then Interlingue after WWII)', 'f', 'ie', 66, null);
INSERT INTO "lang" VALUES (67, 'Igbo (Asụsụ Igbo)', 'f', 'ig', 67, null);
INSERT INTO "lang" VALUES (68, 'Nuosu (ꆈꌠ꒿ Nuosuhxop)', 'f', 'ii', 68, null);
INSERT INTO "lang" VALUES (69, 'Inupiaq (Iñupiaq, Iñupiatun)', 'f', 'ik', 69, null);
INSERT INTO "lang" VALUES (70, 'Ido', 'f', 'io', 70, null);
INSERT INTO "lang" VALUES (71, 'Icelandic (Íslenska)', 'f', 'is', 71, null);
INSERT INTO "lang" VALUES (72, 'Italian (Italiano)', 'f', 'it', 72, null);
INSERT INTO "lang" VALUES (73, 'Inuktitut (ᐃᓄᒃᑎᑐᑦ)', 'f', 'iu', 73, null);
INSERT INTO "lang" VALUES (74, 'Japanese (日本語 (にほんご))', 'f', 'ja', 74, null);
INSERT INTO "lang" VALUES (75, 'Javanese (ꦧꦱꦗꦮ, Basa Jawa)', 'f', 'jv', 75, null);
INSERT INTO "lang" VALUES (76, 'Georgian (ქართული)', 'f', 'ka', 76, null);
INSERT INTO "lang" VALUES (77, 'Kongo (Kikongo)', 'f', 'kg', 77, null);
INSERT INTO "lang" VALUES (78, 'Kikuyu, Gikuyu (Gĩkũyũ)', 'f', 'ki', 78, null);
INSERT INTO "lang" VALUES (79, 'Kwanyama, Kuanyama (Kuanyama)', 'f', 'kj', 79, null);
INSERT INTO "lang" VALUES (80, 'Kazakh (қазақ тілі)', 'f', 'kk', 80, null);
INSERT INTO "lang" VALUES (81, 'Kalaallisut, Greenlandic (kalaallisut, kalaallit oqaasii)', 'f', 'kl', 81, null);
INSERT INTO "lang" VALUES (82, 'Khmer (ខ្មែរ, ខេមរភាសា, ភាសាខ្មែរ)', 'f', 'km', 82, null);
INSERT INTO "lang" VALUES (83, 'Kannada (ಕನ್ನಡ)', 'f', 'kn', 83, null);
INSERT INTO "lang" VALUES (84, 'Korean (한국어)', 'f', 'ko', 84, null);
INSERT INTO "lang" VALUES (85, 'Kanuri', 'f', 'kr', 85, null);
INSERT INTO "lang" VALUES (86, 'Kashmiri (कश्मीरी, کشمیری)', 'f', 'ks', 86, null);
INSERT INTO "lang" VALUES (87, 'Kurdish (Kurdî, كوردی)', 'f', 'ku', 87, null);
INSERT INTO "lang" VALUES (88, 'Komi (коми кыв)', 'f', 'kv', 88, null);
INSERT INTO "lang" VALUES (89, 'Cornish (Kernewek)', 'f', 'kw', 89, null);
INSERT INTO "lang" VALUES (90, 'Kyrgyz (Кыргызча, Кыргыз тили)', 'f', 'ky', 90, null);
INSERT INTO "lang" VALUES (91, 'Latin (latine, lingua latina)', 'f', 'la', 91, null);
INSERT INTO "lang" VALUES (92, 'Luxembourgish, Letzeburgesch (Lëtzebuergesch)', 'f', 'lb', 92, null);
INSERT INTO "lang" VALUES (93, 'Ganda (Luganda)', 'f', 'lg', 93, null);
INSERT INTO "lang" VALUES (94, 'Limburgish, Limburgan, Limburger (Limburgs)', 'f', 'li', 94, null);
INSERT INTO "lang" VALUES (95, 'Lingala (Lingála)', 'f', 'ln', 95, null);
INSERT INTO "lang" VALUES (96, 'Lao (ພາສາລາວ)', 'f', 'lo', 96, null);
INSERT INTO "lang" VALUES (97, 'Lithuanian (lietuvių kalba)', 'f', 'lt', 97, null);
INSERT INTO "lang" VALUES (98, 'Luba-Katanga (Tshiluba)', 'f', 'lu', 98, null);
INSERT INTO "lang" VALUES (99, 'Latvian (latviešu valoda)', 'f', 'lv', 99, null);
INSERT INTO "lang" VALUES (100, 'Malagasy (fiteny malagasy)', 'f', 'mg', 100, null);
INSERT INTO "lang" VALUES (101, 'Marshallese (Kajin M̧ajeļ)', 'f', 'mh', 101, null);
INSERT INTO "lang" VALUES (102, 'Māori (te reo Māori)', 'f', 'mi', 102, null);
INSERT INTO "lang" VALUES (103, 'Macedonian (македонски јазик)', 'f', 'mk', 103, null);
INSERT INTO "lang" VALUES (104, 'Malayalam (മലയാളം)', 'f', 'ml', 104, null);
INSERT INTO "lang" VALUES (105, 'Mongolian (Монгол хэл)', 'f', 'mn', 105, null);
INSERT INTO "lang" VALUES (106, 'Marathi (Marāṭhī) (मराठी)', 'f', 'mr', 106, null);
INSERT INTO "lang" VALUES (107, 'Malay (bahasa Melayu, بهاس ملايو)', 'f', 'ms', 107, null);
INSERT INTO "lang" VALUES (108, 'Maltese (Malti)', 'f', 'mt', 108, null);
INSERT INTO "lang" VALUES (109, 'Burmese (ဗမာစာ)', 'f', 'my', 109, null);
INSERT INTO "lang" VALUES (110, 'Nauruan (Dorerin Naoero)', 'f', 'na', 110, null);
INSERT INTO "lang" VALUES (111, 'Norwegian Bokmål (Norsk bokmål)', 'f', 'nb', 111, null);
INSERT INTO "lang" VALUES (112, 'Northern Ndebele (isiNdebele)', 'f', 'nd', 112, null);
INSERT INTO "lang" VALUES (113, 'Nepali (नेपाली)', 'f', 'ne', 113, null);
INSERT INTO "lang" VALUES (114, 'Ndonga (Owambo)', 'f', 'ng', 114, null);
INSERT INTO "lang" VALUES (115, 'Dutch (Nederlands, Vlaams)', 'f', 'nl', 115, null);
INSERT INTO "lang" VALUES (116, 'Norwegian Nynorsk (Norsk nynorsk)', 'f', 'nn', 116, null);
INSERT INTO "lang" VALUES (117, 'Norwegian (Norsk)', 'f', 'no', 117, null);
INSERT INTO "lang" VALUES (118, 'Southern Ndebele (isiNdebele)', 'f', 'nr', 118, null);
INSERT INTO "lang" VALUES (119, 'Navajo, Navaho (Diné bizaad)', 'f', 'nv', 119, null);
INSERT INTO "lang" VALUES (120, 'Chichewa, Chewa, Nyanja (chiCheŵa, chinyanja)', 'f', 'ny', 120, null);
INSERT INTO "lang" VALUES (121, 'Occitan (occitan, lenga d''òc)', 'f', 'oc', 121, null);
INSERT INTO "lang" VALUES (122, 'Ojibwe, Ojibwa (ᐊᓂᔑᓈᐯᒧᐎᓐ)', 'f', 'oj', 122, null);
INSERT INTO "lang" VALUES (123, 'Oromo (Afaan Oromoo)', 'f', 'om', 123, null);
INSERT INTO "lang" VALUES (124, 'Oriya (ଓଡ଼ିଆ)', 'f', 'or', 124, null);
INSERT INTO "lang" VALUES (125, 'Ossetian, Ossetic (ирон æвзаг)', 'f', 'os', 125, null);
INSERT INTO "lang" VALUES (126, '(Eastern) Punjabi (ਪੰਜਾਬੀ)', 'f', 'pa', 126, null);
INSERT INTO "lang" VALUES (127, 'Pāli (पाऴि)', 'f', 'pi', 127, null);
INSERT INTO "lang" VALUES (128, 'Polish (język polski, polszczyzna)', 'f', 'pl', 128, null);
INSERT INTO "lang" VALUES (129, 'Pashto, Pushto (پښتو)', 'f', 'ps', 129, null);
INSERT INTO "lang" VALUES (130, 'Portuguese (Português)', 'f', 'pt', 130, null);
INSERT INTO "lang" VALUES (131, 'Quechua (Runa Simi, Kichwa)', 'f', 'qu', 131, null);
INSERT INTO "lang" VALUES (132, 'Romansh (rumantsch grischun)', 'f', 'rm', 132, null);
INSERT INTO "lang" VALUES (133, 'Kirundi (Ikirundi)', 'f', 'rn', 133, null);
INSERT INTO "lang" VALUES (134, 'Romanian (Română)', 'f', 'ro', 134, null);
INSERT INTO "lang" VALUES (135, 'Kinyarwanda (Ikinyarwanda)', 'f', 'rw', 135, null);
INSERT INTO "lang" VALUES (136, 'Sanskrit (Saṁskṛta) (संस्कृतम्)', 'f', 'sa', 136, null);
INSERT INTO "lang" VALUES (137, 'Sardinian (sardu)', 'f', 'sc', 137, null);
INSERT INTO "lang" VALUES (138, 'Sindhi (सिन्धी, سنڌي، سندھی)', 'f', 'sd', 138, null);
INSERT INTO "lang" VALUES (139, 'Northern Sami (Davvisámegiella)', 'f', 'se', 139, null);
INSERT INTO "lang" VALUES (140, 'Sango (yângâ tî sängö)', 'f', 'sg', 140, null);
INSERT INTO "lang" VALUES (141, 'Sinhalese, Sinhala (සිංහල)', 'f', 'si', 141, null);
INSERT INTO "lang" VALUES (142, 'Slovak (slovenčina, slovenský jazyk)', 'f', 'sk', 142, null);
INSERT INTO "lang" VALUES (143, 'Slovene (slovenski jezik, slovenščina)', 'f', 'sl', 143, null);
INSERT INTO "lang" VALUES (144, 'Samoan (gagana fa''a Samoa)', 'f', 'sm', 144, null);
INSERT INTO "lang" VALUES (145, 'Shona (chiShona)', 'f', 'sn', 145, null);
INSERT INTO "lang" VALUES (146, 'Somali (Soomaaliga, af Soomaali)', 'f', 'so', 146, null);
INSERT INTO "lang" VALUES (147, 'Albanian (Shqip)', 'f', 'sq', 147, null);
INSERT INTO "lang" VALUES (148, 'Serbian (српски језик)', 'f', 'sr', 148, null);
INSERT INTO "lang" VALUES (149, 'Swati (SiSwati)', 'f', 'ss', 149, null);
INSERT INTO "lang" VALUES (150, 'Southern Sotho (Sesotho)', 'f', 'st', 150, null);
INSERT INTO "lang" VALUES (151, 'Sundanese (Basa Sunda)', 'f', 'su', 151, null);
INSERT INTO "lang" VALUES (152, 'Swedish (svenska)', 'f', 'sv', 152, null);
INSERT INTO "lang" VALUES (153, 'Swahili (Kiswahili)', 'f', 'sw', 153, null);
INSERT INTO "lang" VALUES (154, 'Tamil (தமிழ்)', 'f', 'ta', 154, null);
INSERT INTO "lang" VALUES (155, 'Telugu (తెలుగు)', 'f', 'te', 155, null);
INSERT INTO "lang" VALUES (156, 'Tajik (тоҷикӣ, toçikī, تاجیکی)', 'f', 'tg', 156, null);
INSERT INTO "lang" VALUES (157, 'Thai (ไทย)', 'f', 'th', 157, null);
INSERT INTO "lang" VALUES (158, 'Tigrinya (ትግርኛ)', 'f', 'ti', 158, null);
INSERT INTO "lang" VALUES (159, 'Turkmen (Türkmen, Түркмен)', 'f', 'tk', 159, null);
INSERT INTO "lang" VALUES (160, 'Tagalog (Wikang Tagalog)', 'f', 'tl', 160, null);
INSERT INTO "lang" VALUES (161, 'Tswana (Setswana)', 'f', 'tn', 161, null);
INSERT INTO "lang" VALUES (162, 'Tonga (Tonga Islands) (faka Tonga)', 'f', 'to', 162, null);
INSERT INTO "lang" VALUES (163, 'Turkish (Türkçe)', 'f', 'tr', 163, null);
INSERT INTO "lang" VALUES (164, 'Tsonga (Xitsonga)', 'f', 'ts', 164, null);
INSERT INTO "lang" VALUES (165, 'Tatar (татар теле, tatar tele)', 'f', 'tt', 165, null);
INSERT INTO "lang" VALUES (166, 'Twi', 'f', 'tw', 166, null);
INSERT INTO "lang" VALUES (167, 'Tahitian (Reo Tahiti)', 'f', 'ty', 167, null);
INSERT INTO "lang" VALUES (168, 'Uyghur (ئۇيغۇرچە, Uyghurche)', 'f', 'ug', 168, null);
INSERT INTO "lang" VALUES (169, 'Urdu (اردو)', 'f', 'ur', 169, null);
INSERT INTO "lang" VALUES (170, 'Uzbek (Oʻzbek, Ўзбек, أۇزبېك)', 'f', 'uz', 170, null);
INSERT INTO "lang" VALUES (171, 'Venda (Tshivenḓa)', 'f', 've', 171, null);
INSERT INTO "lang" VALUES (172, 'Vietnamese (Tiếng Việt)', 'f', 'vi', 172, null);
INSERT INTO "lang" VALUES (173, 'Volapük', 'f', 'vo', 173, null);
INSERT INTO "lang" VALUES (174, 'Walloon (walon)', 'f', 'wa', 174, null);
INSERT INTO "lang" VALUES (175, 'Wolof (Wollof)', 'f', 'wo', 175, null);
INSERT INTO "lang" VALUES (176, 'Xhosa (isiXhosa)', 'f', 'xh', 176, null);
INSERT INTO "lang" VALUES (177, 'Yiddish (ייִדיש)', 'f', 'yi', 177, null);
INSERT INTO "lang" VALUES (178, 'Yoruba (Yorùbá)', 'f', 'yo', 178, null);
INSERT INTO "lang" VALUES (179, 'Zhuang, Chuang (Saɯ cueŋƅ, Saw cuengh)', 'f', 'za', 179, null);
INSERT INTO "lang" VALUES (180, 'Chinese (中文 (Zhōngwén), 汉语, 漢語)', 'f', 'zh', 180, null);
INSERT INTO "lang" VALUES (181, 'Zulu (isiZulu)', 'f', 'zu', 181, null);

-- ----------------------------
-- Table structure for mail
-- ----------------------------
CREATE TABLE "mail" (
  "mail_id" int8 NOT NULL GENERATED BY DEFAULT AS IDENTITY,
  "user_id" int8 NOT NULL,
  "mail" jsonb NOT NULL,
  "create" timestamptz NOT NULL,
  "send" timestamptz,
  "err" bool NOT NULL,
  "err_text" text,
  "transport" text NOT NULL
);

COMMENT ON COLUMN "mail"."mail_id" IS 'Identifier';
COMMENT ON COLUMN "mail"."user_id" IS 'User';
COMMENT ON COLUMN "mail"."mail" IS 'Message';
COMMENT ON COLUMN "mail"."create" IS 'Date created';
COMMENT ON COLUMN "mail"."send" IS 'Date sended';
COMMENT ON COLUMN "mail"."err" IS 'Is error';
COMMENT ON COLUMN "mail"."err_text" IS 'Error message';
COMMENT ON COLUMN "mail"."transport" IS 'Transport';
COMMENT ON TABLE "mail" IS 'Email';

-- ----------------------------
-- Table structure for provider
-- ----------------------------
CREATE TABLE "provider" (
  "provider_id" int8 NOT NULL GENERATED BY DEFAULT AS IDENTITY,
  "name" text NOT NULL,
  "enable" bool NOT NULL,
  "master" bool NOT NULL,
  "slave" bool NOT NULL,
  "config" jsonb NOT NULL
);
COMMENT ON COLUMN "provider"."provider_id" IS 'Identifier';
COMMENT ON COLUMN "provider"."name" IS 'Name';
COMMENT ON COLUMN "provider"."enable" IS 'Profile enabled';
COMMENT ON COLUMN "provider"."master" IS 'Can be used for the primary login';
COMMENT ON COLUMN "provider"."slave" IS 'Can be used for the two-factor login';
COMMENT ON COLUMN "provider"."config" IS 'Provider config';
COMMENT ON TABLE "provider" IS 'Login provider';

-- ----------------------------
-- Table structure for redirect
-- ----------------------------
CREATE TABLE "redirect" (
  "redirect_id" int8 NOT NULL GENERATED BY DEFAULT AS IDENTITY,
  "url" text NOT NULL,
  "permanently" bool NOT NULL,
  "redirect" text NOT NULL
);
COMMENT ON COLUMN "redirect"."redirect_id" IS 'Identifier';
COMMENT ON COLUMN "redirect"."url" IS 'Request URL';
COMMENT ON COLUMN "redirect"."permanently" IS '301 or 302 http code';
COMMENT ON COLUMN "redirect"."redirect" IS 'New URL';
COMMENT ON TABLE "redirect" IS 'Redirect url';

-- ----------------------------
-- Table structure for role
-- ----------------------------
CREATE TABLE "role" (
  "role_id" int8 NOT NULL GENERATED BY DEFAULT AS IDENTITY,
  "name" jsonb NOT NULL,
  "description" jsonb NOT NULL
);
COMMENT ON COLUMN "role"."role_id" IS 'Identifier';
COMMENT ON COLUMN "role"."name" IS 'Name';
COMMENT ON COLUMN "role"."description" IS 'Description';
COMMENT ON TABLE "role" IS 'Roles list';

INSERT INTO "role" VALUES (0, '{}', '{}'); -- Unregistered user
INSERT INTO "role" VALUES (1, '{}', '{}'); -- Administrator
INSERT INTO "role" VALUES (2, '{}', '{}'); -- Registered user

-- ----------------------------
-- Table structure for route
-- ----------------------------
CREATE TABLE "route" (
  "route_id" int8 NOT NULL GENERATED BY DEFAULT AS IDENTITY,
  "url" text NOT NULL,
  "controller_id" int8 NOT NULL,
  "params" text,
  "lang_id" int8
);
COMMENT ON COLUMN "route"."route_id" IS 'Identifier';
COMMENT ON COLUMN "route"."url" IS 'Request URL';
COMMENT ON COLUMN "route"."controller_id" IS 'Controller ID';
COMMENT ON COLUMN "route"."params" IS 'Params';
COMMENT ON COLUMN "route"."lang_id" IS 'Language';
COMMENT ON TABLE "route" IS 'Route map';

-- ----------------------------
-- Table structure for session
-- ----------------------------
CREATE TABLE "session" (
  "session_id" int8 NOT NULL GENERATED BY DEFAULT AS IDENTITY,
  "user_id" int8 NOT NULL,
  "lang_id" int8 NOT NULL,
  "session" text NOT NULL,
  "data" bytea NOT NULL,
  "created" timestamptz NOT NULL,
  "last" timestamptz NOT NULL,
  "ip" text NOT NULL,
  "user_agent" text NOT NULL
);
COMMENT ON COLUMN "session"."session_id" IS 'Identifier';
COMMENT ON COLUMN "session"."user_id" IS 'User ID';
COMMENT ON COLUMN "session"."lang_id" IS 'Language';
COMMENT ON COLUMN "session"."session" IS 'Session key';
COMMENT ON COLUMN "session"."data" IS 'Session data';
COMMENT ON COLUMN "session"."created" IS 'Creation time';
COMMENT ON COLUMN "session"."last" IS 'Last change time';
COMMENT ON COLUMN "session"."ip" IS 'Last IP client address';
COMMENT ON COLUMN "session"."user_agent" IS 'Last UserAgent client';
COMMENT ON TABLE "session" IS 'Users session';

-- ----------------------------
-- Table structure for setting
-- ----------------------------
CREATE TABLE "setting" (
  "setting_id" int8 NOT NULL GENERATED BY DEFAULT AS IDENTITY,
  "key" int8 NOT NULL,
  "data" text NOT NULL,
  "key_text" text NOT NULL,
  "strict" text NOT NULL
);
COMMENT ON COLUMN "setting"."setting_id" IS 'Identifier';
COMMENT ON COLUMN "setting"."key" IS 'fnv1a_64(Key)';
COMMENT ON COLUMN "setting"."data" IS 'Data';
COMMENT ON COLUMN "setting"."key_text" IS 'Key';
COMMENT ON COLUMN "setting"."strict" IS 'Limits on data';
COMMENT ON TABLE "setting" IS 'General settings';

INSERT INTO "setting" VALUES (1, 1441962092377564137, 'None', 'mail:provider', 'None|Sendmail|SMTP|File');
INSERT INTO "setting" VALUES (2, -3979813852156915759, 'sendmail', 'mail:sendmail', '');
INSERT INTO "setting" VALUES (3, -4738603782623769110, 'email', 'mail:file', '');
INSERT INTO "setting" VALUES (4, -390595084051732771, 'localhost', 'mail:smtp:server', '');
INSERT INTO "setting" VALUES (5, -1521500012746197243, '465', 'mail:smtp:port', '');
INSERT INTO "setting" VALUES (6, 4706107683829871299, 'SSL/TLS', 'mail:smtp:tls', 'None|STARTTLS|SSL/TLS');
INSERT INTO "setting" VALUES (7, -8449193462972437408, 'PLAIN', 'mail:smtp:auth', 'None|PLAIN|LOGIN|XOAUTH2');
INSERT INTO "setting" VALUES (8, 1199393424318567565, '', 'mail:smtp:user', '');
INSERT INTO "setting" VALUES (9, 2346365514808828621, '', 'mail:smtp:pwd', '');

-- ----------------------------
-- Table structure for user
-- ----------------------------
CREATE TABLE "user" (
  "user_id" int8 NOT NULL GENERATED BY DEFAULT AS IDENTITY,
  "enable" bool NOT NULL DEFAULT false,
  "lang_id" int8 NOT NULL,
  "create" timestamptz NOT NULL,
  "protect" bool NOT NULL,
  "role_id" int8 NOT NULL,
  "data" jsonb NOT NULL
);
COMMENT ON COLUMN "user"."user_id" IS 'Identifier';
COMMENT ON COLUMN "user"."enable" IS 'User enable';
COMMENT ON COLUMN "user"."lang_id" IS 'Language';
COMMENT ON COLUMN "user"."create" IS 'Creation time';
COMMENT ON COLUMN "user"."protect" IS 'Protect account';
COMMENT ON COLUMN "user"."role_id" IS 'User role';
COMMENT ON COLUMN "user"."data" IS 'Profile data';
COMMENT ON TABLE "user" IS 'Users list';

INSERT INTO "user" VALUES (0, 't', 0, '2023-01-01 00:00:00+00', 't', 0, '{}');

-- ----------------------------
-- Table structure for user
-- ----------------------------
CREATE TABLE "user_provider" (
  "user_provider_id" int8 NOT NULL GENERATED BY DEFAULT AS IDENTITY,
  "user_id" int8 NOT NULL,
  "provider_id" int8 NOT NULL,
  "enable" bool NOT NULL,
  "data" jsonb NOT NULL,
  "update" timestamptz NOT NULL,
  "expire" timestamptz NOT NULL
);
COMMENT ON COLUMN "user_provider"."user_provider_id" IS 'Identifier';
COMMENT ON COLUMN "user_provider"."user_id" IS 'User';
COMMENT ON COLUMN "user_provider"."provider_id" IS 'Provider';
COMMENT ON COLUMN "user_provider"."enable" IS 'Enable';
COMMENT ON COLUMN "user_provider"."data" IS 'Data';
COMMENT ON COLUMN "user_provider"."update" IS 'DateTime of update';
COMMENT ON COLUMN "user_provider"."expire" IS 'Expires DateTime';
COMMENT ON TABLE "user_provider" IS 'Use of the provider for the user';

-- ----------------------------
-- Auto increment value
-- ----------------------------
SELECT setval('"access_access_id_seq"', 3, true);
SELECT setval('"controller_controller_id_seq"', 5, true);
SELECT setval('"lang_lang_id_seq"', 181, true);
SELECT setval('"role_role_id_seq"', 2, true);
SELECT setval('"setting_setting_id_seq"', 9, true);
SELECT setval('"user_user_id_seq"', 1, false);

-- ----------------------------
-- Indexes structure for table access
-- ----------------------------
CREATE INDEX ON "access" USING btree ("access");
CREATE INDEX ON "access" USING btree ("controller_id");
CREATE UNIQUE INDEX ON "access" USING btree ("role_id", "controller_id");
CREATE INDEX ON "access" USING btree ("role_id");
ALTER TABLE "access" ADD CONSTRAINT "access_pkey" PRIMARY KEY ("access_id");

-- ----------------------------
-- Indexes structure for table controller
-- ----------------------------
CREATE INDEX ON "controller" USING btree ("action_id");
CREATE INDEX ON "controller" USING btree ("class_id");
CREATE UNIQUE INDEX ON "controller" USING btree ("module_id", "class_id", "action_id");
CREATE INDEX ON "controller" USING btree ("module_id");
ALTER TABLE "controller" ADD CONSTRAINT "controller_expr_ch" CHECK (length("module") = 0 AND length("class") = 0 AND length("action") = 0 OR length("module") > 0 AND length("class") = 0 AND length("action") = 0 OR length("module") > 0 AND length("class") > 0 AND length("action") = 0 OR length("module") > 0 AND length("class") > 0 AND length("action") > 0);
ALTER TABLE "controller" ADD CONSTRAINT "controller_pkey" PRIMARY KEY ("controller_id");

-- ----------------------------
-- Indexes structure for table lang
-- ----------------------------
CREATE INDEX ON "lang" USING btree ("enable");
CREATE INDEX ON "lang" USING btree ("code");
CREATE INDEX ON "lang" USING btree ("name");
CREATE INDEX ON "lang" USING btree ("index");
ALTER TABLE "lang" ADD CONSTRAINT "lang_pkey" PRIMARY KEY ("lang_id");

-- ----------------------------
-- Indexes structure for table mail
-- ----------------------------
CREATE INDEX ON "mail" USING btree ("err");
CREATE INDEX ON "mail" USING btree ("send");
CREATE INDEX ON "mail" USING btree ("user_id");
ALTER TABLE "mail" ADD CONSTRAINT "mail_pkey" PRIMARY KEY ("mail_id");

-- ----------------------------
-- Indexes structure for table provider
-- ----------------------------
CREATE INDEX ON "provider" USING btree ("enable");
CREATE INDEX ON "provider" USING btree ("master");
CREATE UNIQUE INDEX ON "provider" USING btree ("name");
CREATE INDEX ON "provider" USING btree ("slave");
ALTER TABLE "provider" ADD CONSTRAINT "provider_pkey" PRIMARY KEY ("provider_id");

-- ----------------------------
-- Indexes structure for table redirect
-- ----------------------------
CREATE UNIQUE INDEX ON "redirect" USING btree ("url");
ALTER TABLE "redirect" ADD CONSTRAINT "redirect_pkey" PRIMARY KEY ("redirect_id");

-- ----------------------------
-- Indexes structure for table role
-- ----------------------------
ALTER TABLE "role" ADD CONSTRAINT "role_pkey" PRIMARY KEY ("role_id");

-- ----------------------------
-- Indexes structure for table route
-- ----------------------------
CREATE INDEX ON "route" USING btree ("controller_id");
CREATE INDEX ON "route" USING btree ("lang_id");
CREATE INDEX ON "route" USING btree ("params");
CREATE UNIQUE INDEX ON "route" USING btree ("url");
ALTER TABLE "route" ADD CONSTRAINT "route_pkey" PRIMARY KEY ("route_id");

-- ----------------------------
-- Indexes structure for table session
-- ----------------------------
CREATE UNIQUE INDEX ON "session" USING btree ("session");
CREATE INDEX ON "session" USING btree ("user_id");
ALTER TABLE "session" ADD CONSTRAINT "session_pkey" PRIMARY KEY ("session_id");

-- ----------------------------
-- Indexes structure for table setting
-- ----------------------------
CREATE UNIQUE INDEX ON "setting" USING btree ("key");
ALTER TABLE "setting" ADD CONSTRAINT "setting_pkey" PRIMARY KEY ("setting_id");

-- ----------------------------
-- Indexes structure for table user
-- ----------------------------
CREATE INDEX ON "user" USING btree ("enable");
CREATE INDEX ON "user" USING btree ("lang_id");
CREATE INDEX ON "user" USING btree ("protect");
CREATE INDEX ON "user" USING btree ("role_id");
ALTER TABLE "user" ADD CONSTRAINT "user_pkey" PRIMARY KEY ("user_id");

-- ----------------------------
-- Indexes structure for table user_provider
-- ----------------------------
CREATE INDEX ON "user_provider" USING btree ("enable");
CREATE INDEX ON "user_provider" USING btree ("provider_id");
CREATE INDEX ON "user_provider" USING btree ("user_id");
CREATE UNIQUE INDEX ON "user_provider" USING btree ("user_id", "provider_id");
ALTER TABLE "user_provider" ADD CONSTRAINT "user_provider_pkey" PRIMARY KEY ("user_provider_id");

-- ----------------------------
-- Foreign Keys structure
-- ----------------------------
ALTER TABLE "access" ADD CONSTRAINT "access_controller_id_fkey" FOREIGN KEY ("controller_id") REFERENCES "controller" ("controller_id");
ALTER TABLE "access" ADD CONSTRAINT "access_role_id_fkey" FOREIGN KEY ("role_id") REFERENCES "role" ("role_id");
ALTER TABLE "mail" ADD CONSTRAINT "mail_user_id_fkey" FOREIGN KEY ("user_id") REFERENCES "user" ("user_id");
ALTER TABLE "route" ADD CONSTRAINT "route_controller_id_fkey" FOREIGN KEY ("controller_id") REFERENCES "controller" ("controller_id");
ALTER TABLE "route" ADD CONSTRAINT "route_lang_id_fkey" FOREIGN KEY ("lang_id") REFERENCES "lang" ("lang_id");
ALTER TABLE "session" ADD CONSTRAINT "session_user_id_fkey" FOREIGN KEY ("user_id") REFERENCES "user" ("user_id");
ALTER TABLE "user" ADD CONSTRAINT "user_lang_id_fkey" FOREIGN KEY ("lang_id") REFERENCES "lang" ("lang_id");
ALTER TABLE "user" ADD CONSTRAINT "user_role_id_fkey" FOREIGN KEY ("role_id") REFERENCES "role" ("role_id");
ALTER TABLE "user_provider" ADD CONSTRAINT "user_provider_provider_id_fkey" FOREIGN KEY ("provider_id") REFERENCES "provider" ("provider_id");
ALTER TABLE "user_provider" ADD CONSTRAINT "user_provider_user_id_fkey" FOREIGN KEY ("user_id") REFERENCES "user" ("user_id");

-- ----------------------------
-- Trigers for lang.index structure
-- ----------------------------
CREATE FUNCTION lang_insert_row()
RETURNS TRIGGER AS $$
BEGIN
  NEW.index = NULL;
  NEW.enable = FALSE;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_lang_insert_row
BEFORE INSERT ON lang
FOR EACH ROW EXECUTE FUNCTION lang_insert_row();

CREATE FUNCTION lang_change_row()
RETURNS TRIGGER AS $$
BEGIN
  IF OLD.index IS NOT NULL THEN
    NEW.index = OLD.index;
  ELSEIF OLD.index IS NULL AND NEW.index IS NOT NULL THEN
    SELECT COALESCE(MAX(index), -1) + 1 INTO NEW.index FROM lang;
  END IF;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_lang_change_row
BEFORE UPDATE ON lang
FOR EACH ROW EXECUTE FUNCTION lang_change_row();