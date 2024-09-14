-- ----------------------------
-- Table structure for access
-- ----------------------------
CREATE TABLE [access] (
  [access_id] BIGINT IDENTITY NOT NULL,
  [role_id] BIGINT NOT NULL,
  [access] BIT NOT NULL,
  [controller_id] BIGINT NOT NULL,
  PRIMARY KEY CLUSTERED ([access_id])
);

EXEC sp_addextendedproperty
'MS_Description', N'Identifier',
'SCHEMA', N'dbo',
'TABLE', N'access',
'COLUMN', N'access_id';

EXEC sp_addextendedproperty
'MS_Description', N'Role ID',
'SCHEMA', N'dbo',
'TABLE', N'access',
'COLUMN', N'role_id';

EXEC sp_addextendedproperty
'MS_Description', N'Access flag',
'SCHEMA', N'dbo',
'TABLE', N'access',
'COLUMN', N'access';

EXEC sp_addextendedproperty
'MS_Description', N'Controller ID',
'SCHEMA', N'dbo',
'TABLE', N'access',
'COLUMN', N'controller_id';

EXEC sp_addextendedproperty
'MS_Description', N'Access to controllers',
'SCHEMA', N'dbo',
'TABLE', N'access';

SET IDENTITY_INSERT [access] ON;
INSERT INTO [access]([access_id], [role_id], [access], [controller_id]) VALUES (1, 0, 1, 1);
INSERT INTO [access]([access_id], [role_id], [access], [controller_id]) VALUES (2, 0, 1, 4);
INSERT INTO [access]([access_id], [role_id], [access], [controller_id]) VALUES (3, 0, 1, 5);
SET IDENTITY_INSERT [access] OFF;

-- ----------------------------
-- Table structure for controller
-- ----------------------------
CREATE TABLE [controller] (
  [controller_id] BIGINT IDENTITY NOT NULL,
  [module] VARCHAR(255) NOT NULL,
  [class] VARCHAR(255) NOT NULL,
  [action] VARCHAR(255) NOT NULL,
  [description] NVARCHAR(MAX) NOT NULL,
  [module_id] BIGINT NOT NULL,
  [class_id] BIGINT NOT NULL,
  [action_id] BIGINT NOT NULL,
  PRIMARY KEY CLUSTERED ([controller_id])
);
EXEC sp_addextendedproperty
'MS_Description', N'Identifier',
'SCHEMA', N'dbo',
'TABLE', N'controller',
'COLUMN', N'controller_id';

EXEC sp_addextendedproperty
'MS_Description', N'Module',
'SCHEMA', N'dbo',
'TABLE', N'controller',
'COLUMN', N'module';

EXEC sp_addextendedproperty
'MS_Description', N'Class',
'SCHEMA', N'dbo',
'TABLE', N'controller',
'COLUMN', N'class';

EXEC sp_addextendedproperty
'MS_Description', N'Action (controller)',
'SCHEMA', N'dbo',
'TABLE', N'controller',
'COLUMN', N'action';

EXEC sp_addextendedproperty
'MS_Description', N'Description',
'SCHEMA', N'dbo',
'TABLE', N'controller',
'COLUMN', N'description';

EXEC sp_addextendedproperty
'MS_Description', N'fnv1a_64 hash from module',
'SCHEMA', N'dbo',
'TABLE', N'controller',
'COLUMN', N'module_id';

EXEC sp_addextendedproperty
'MS_Description', N'fnv1a_64 hash from class',
'SCHEMA', N'dbo',
'TABLE', N'controller',
'COLUMN', N'class_id';

EXEC sp_addextendedproperty
'MS_Description', N'fnv1a_64 hash from action',
'SCHEMA', N'dbo',
'TABLE', N'controller',
'COLUMN', N'action_id';

EXEC sp_addextendedproperty
'MS_Description', N'Controllers list',
'SCHEMA', N'dbo',
'TABLE', N'controller';

SET IDENTITY_INSERT [controller] ON;
INSERT INTO [controller]([controller_id], [module], [class], [action], [description], [module_id], [class_id], [action_id]) VALUES (1, 'index', '', '', '{}', -8948777187306027381, -3750763034362895579, -3750763034362895579);
INSERT INTO [controller]([controller_id], [module], [class], [action], [description], [module_id], [class_id], [action_id]) VALUES (2, 'index', 'index', 'index', '{}', -8948777187306027381, -8948777187306027381, -8948777187306027381);
INSERT INTO [controller]([controller_id], [module], [class], [action], [description], [module_id], [class_id], [action_id]) VALUES (3, 'index', 'index', 'not_found', '{}', -8948777187306027381, -8948777187306027381, -1573091631220776463);
INSERT INTO [controller]([controller_id], [module], [class], [action], [description], [module_id], [class_id], [action_id]) VALUES (4, 'admin', 'index', '', '{}', -1887597591324883884, -8948777187306027381, -3750763034362895579);
INSERT INTO [controller]([controller_id], [module], [class], [action], [description], [module_id], [class_id], [action_id]) VALUES (5, 'admin', 'login', '', '{}', -1887597591324883884, 272289342528891346, -3750763034362895579);
SET IDENTITY_INSERT [controller] OFF;

-- ----------------------------
-- Table structure for lang
-- ----------------------------
CREATE TABLE [lang](
  [lang_id] BIGINT IDENTITY NOT NULL,
  [name] NVARCHAR(255) NOT NULL,
  [enable] BIT DEFAULT 0 NOT NULL,
  [code] VARCHAR(2) NOT NULL,
  [sort] BIGINT NOT NULL,
  [index] BIGINT,
  PRIMARY KEY CLUSTERED ([lang_id])
);

EXEC sp_addextendedproperty
'MS_Description', N'Identifier',
'SCHEMA', N'dbo',
'TABLE', N'lang',
'COLUMN', N'lang_id';

EXEC sp_addextendedproperty
'MS_Description', N'Language name',
'SCHEMA', N'dbo',
'TABLE', N'lang',
'COLUMN', N'name';

EXEC sp_addextendedproperty
'MS_Description', N'Enable',
'SCHEMA', N'dbo',
'TABLE', N'lang',
'COLUMN', N'enable';

EXEC sp_addextendedproperty
'MS_Description', N'ISO 639-1 : uk - ukrainian, en - english',
'SCHEMA', N'dbo',
'TABLE', N'lang',
'COLUMN', N'code';

EXEC sp_addextendedproperty
'MS_Description', N'Sort order',
'SCHEMA', N'dbo',
'TABLE', N'lang',
'COLUMN', N'sort';

EXEC sp_addextendedproperty
'MS_Description', N'Index in JSON type field db',
'SCHEMA', N'dbo',
'TABLE', N'lang',
'COLUMN', N'index';

EXEC sp_addextendedproperty
'MS_Description', N'Languages',
'SCHEMA', N'dbo',
'TABLE', N'lang';

SET IDENTITY_INSERT [lang] ON;
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (0, 'English', 0, 'en', 0, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (1, 'Ukrainian (Українська)', 0, 'uk', 1, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (2, 'Afar (Afaraf)', 0, 'aa', 2, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (3, 'Abkhaz (аҧсуа бызшәа, аҧсшәа)', 0, 'ab', 3, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (4, 'Avestan (avesta)', 0, 'ae', 4, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (5, 'Afrikaans', 0, 'af', 5, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (6, 'Akan', 0, 'ak', 6, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (7, 'Amharic (አማርኛ)', 0, 'am', 7, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (8, 'Aragonese (aragonés)', 0, 'an', 8, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (9, 'Arabic (العربية)', 0, 'ar', 9, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (10, 'Assamese (অসমীয়া)', 0, 'as', 10, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (11, 'Avaric (авар мацӀ, магӀарул мацӀ)', 0, 'av', 11, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (12, 'Aymara (aymar aru)', 0, 'ay', 12, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (13, 'Azerbaijani (azərbaycan dili)', 0, 'az', 13, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (14, 'Bashkir (башҡорт теле)', 0, 'ba', 14, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (15, 'Bulgarian (български език)', 0, 'bg', 15, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (16, 'Bihari (भोजपुरी)', 0, 'bh', 16, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (17, 'Bislama', 0, 'bi', 17, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (18, 'Bambara (bamanankan)', 0, 'bm', 18, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (19, 'Bengali, Bangla (বাংলা)', 0, 'bn', 19, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (20, 'Tibetan Standard, Tibetan, Central (བོད་ཡིག)', 0, 'bo', 20, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (21, 'Breton (brezhoneg)', 0, 'br', 21, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (22, 'Bosnian (bosanski jezik)', 0, 'bs', 22, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (23, 'Catalan (català)', 0, 'ca', 23, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (24, 'Chechen (нохчийн мотт)', 0, 'ce', 24, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (25, 'Chamorro (Chamoru)', 0, 'ch', 25, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (26, 'Corsican (corsu, lingua corsa)', 0, 'co', 26, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (27, 'Cree (ᓀᐦᐃᔭᐍᐏᐣ)', 0, 'cr', 27, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (28, 'Czech (čeština, český jazyk)', 0, 'cs', 28, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (29, 'Old Church Slavonic, Church Slavonic, Old Bulgarian (ѩзыкъ словѣньскъ)', 0, 'cu', 29, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (30, 'Chuvash (чӑваш чӗлхи)', 0, 'cv', 30, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (31, 'Welsh (Cymraeg)', 0, 'cy', 31, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (32, 'Danish (dansk)', 0, 'da', 32, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (33, 'German (Deutsch)', 0, 'de', 33, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (34, 'Divehi, Dhivehi, Maldivian (ދިވެހި)', 0, 'dv', 34, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (35, 'Dzongkha (རྫོང་ཁ)', 0, 'dz', 35, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (36, 'Ewe (Eʋegbe)', 0, 'ee', 36, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (37, 'Greek (modern) (ελληνικά)', 0, 'el', 37, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (38, 'Esperanto', 0, 'eo', 38, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (39, 'Spanish (Español)', 0, 'es', 39, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (40, 'Estonian (eesti, eesti keel)', 0, 'et', 40, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (41, 'Basque (euskara, euskera)', 0, 'eu', 41, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (42, 'Persian (Farsi) (فارسی)', 0, 'fa', 42, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (43, 'Fula, Fulah, Pulaar, Pular (Fulfulde, Pulaar, Pular)', 0, 'ff', 43, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (44, 'Finnish (suomi, suomen kieli)', 0, 'fi', 44, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (45, 'Fijian (vosa Vakaviti)', 0, 'fj', 45, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (46, 'Faroese (føroyskt)', 0, 'fo', 46, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (47, 'French (français, langue française)', 0, 'fr', 47, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (48, 'Western Frisian (Frysk)', 0, 'fy', 48, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (49, 'Irish (Gaeilge)', 0, 'ga', 49, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (50, 'Scottish Gaelic, Gaelic (Gàidhlig)', 0, 'gd', 50, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (51, 'Galician (galego)', 0, 'gl', 51, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (52, 'Guaraní (Avañe''ẽ)', 0, 'gn', 52, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (53, 'Gujarati (ગુજરાતી)', 0, 'gu', 53, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (54, 'Manx (Gaelg, Gailck)', 0, 'gv', 54, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (55, 'Hausa ((Hausa) هَوُسَ)', 0, 'ha', 55, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (56, 'Hebrew (modern) (עברית)', 0, 'he', 56, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (57, 'Hindi (हिन्दी, हिंदी)', 0, 'hi', 57, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (58, 'Hiri Motu', 0, 'ho', 58, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (59, 'Croatian (hrvatski jezik)', 0, 'hr', 59, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (60, 'Haitian, Haitian Creole (Kreyòl ayisyen)', 0, 'ht', 60, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (61, 'Hungarian (magyar)', 0, 'hu', 61, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (62, 'Armenian (Հայերեն)', 0, 'hy', 62, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (63, 'Herero (Otjiherero)', 0, 'hz', 63, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (64, 'Interlingua', 0, 'ia', 64, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (65, 'Indonesian (Bahasa Indonesia)', 0, 'id', 65, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (66, 'Interlingue (Originally called Occidental; then Interlingue after WWII)', 0, 'ie', 66, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (67, 'Igbo (Asụsụ Igbo)', 0, 'ig', 67, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (68, 'Nuosu (ꆈꌠ꒿ Nuosuhxop)', 0, 'ii', 68, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (69, 'Inupiaq (Iñupiaq, Iñupiatun)', 0, 'ik', 69, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (70, 'Ido', 0, 'io', 70, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (71, 'Icelandic (Íslenska)', 0, 'is', 71, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (72, 'Italian (Italiano)', 0, 'it', 72, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (73, 'Inuktitut (ᐃᓄᒃᑎᑐᑦ)', 0, 'iu', 73, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (74, 'Japanese (日本語 (にほんご))', 0, 'ja', 74, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (75, 'Javanese (ꦧꦱꦗꦮ, Basa Jawa)', 0, 'jv', 75, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (76, 'Georgian (ქართული)', 0, 'ka', 76, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (77, 'Kongo (Kikongo)', 0, 'kg', 77, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (78, 'Kikuyu, Gikuyu (Gĩkũyũ)', 0, 'ki', 78, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (79, 'Kwanyama, Kuanyama (Kuanyama)', 0, 'kj', 79, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (80, 'Kazakh (қазақ тілі)', 0, 'kk', 80, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (81, 'Kalaallisut, Greenlandic (kalaallisut, kalaallit oqaasii)', 0, 'kl', 81, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (82, 'Khmer (ខ្មែរ, ខេមរភាសា, ភាសាខ្មែរ)', 0, 'km', 82, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (83, 'Kannada (ಕನ್ನಡ)', 0, 'kn', 83, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (84, 'Korean (한국어)', 0, 'ko', 84, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (85, 'Kanuri', 0, 'kr', 85, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (86, 'Kashmiri (कश्मीरी, کشمیری)', 0, 'ks', 86, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (87, 'Kurdish (Kurdî, كوردی)', 0, 'ku', 87, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (88, 'Komi (коми кыв)', 0, 'kv', 88, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (89, 'Cornish (Kernewek)', 0, 'kw', 89, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (90, 'Kyrgyz (Кыргызча, Кыргыз тили)', 0, 'ky', 90, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (91, 'Latin (latine, lingua latina)', 0, 'la', 91, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (92, 'Luxembourgish, Letzeburgesch (Lëtzebuergesch)', 0, 'lb', 92, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (93, 'Ganda (Luganda)', 0, 'lg', 93, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (94, 'Limburgish, Limburgan, Limburger (Limburgs)', 0, 'li', 94, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (95, 'Lingala (Lingála)', 0, 'ln', 95, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (96, 'Lao (ພາສາລາວ)', 0, 'lo', 96, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (97, 'Lithuanian (lietuvių kalba)', 0, 'lt', 97, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (98, 'Luba-Katanga (Tshiluba)', 0, 'lu', 98, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (99, 'Latvian (latviešu valoda)', 0, 'lv', 99, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (100, 'Malagasy (fiteny malagasy)', 0, 'mg', 100, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (101, 'Marshallese (Kajin M̧ajeļ)', 0, 'mh', 101, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (102, 'Māori (te reo Māori)', 0, 'mi', 102, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (103, 'Macedonian (македонски јазик)', 0, 'mk', 103, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (104, 'Malayalam (മലയാളം)', 0, 'ml', 104, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (105, 'Mongolian (Монгол хэл)', 0, 'mn', 105, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (106, 'Marathi (Marāṭhī) (मराठी)', 0, 'mr', 106, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (107, 'Malay (bahasa Melayu, بهاس ملايو)', 0, 'ms', 107, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (108, 'Maltese (Malti)', 0, 'mt', 108, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (109, 'Burmese (ဗမာစာ)', 0, 'my', 109, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (110, 'Nauruan (Dorerin Naoero)', 0, 'na', 110, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (111, 'Norwegian Bokmål (Norsk bokmål)', 0, 'nb', 111, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (112, 'Northern Ndebele (isiNdebele)', 0, 'nd', 112, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (113, 'Nepali (नेपाली)', 0, 'ne', 113, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (114, 'Ndonga (Owambo)', 0, 'ng', 114, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (115, 'Dutch (Nederlands, Vlaams)', 0, 'nl', 115, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (116, 'Norwegian Nynorsk (Norsk nynorsk)', 0, 'nn', 116, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (117, 'Norwegian (Norsk)', 0, 'no', 117, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (118, 'Southern Ndebele (isiNdebele)', 0, 'nr', 118, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (119, 'Navajo, Navaho (Diné bizaad)', 0, 'nv', 119, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (120, 'Chichewa, Chewa, Nyanja (chiCheŵa, chinyanja)', 0, 'ny', 120, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (121, 'Occitan (occitan, lenga d''òc)', 0, 'oc', 121, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (122, 'Ojibwe, Ojibwa (ᐊᓂᔑᓈᐯᒧᐎᓐ)', 0, 'oj', 122, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (123, 'Oromo (Afaan Oromoo)', 0, 'om', 123, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (124, 'Oriya (ଓଡ଼ିଆ)', 0, 'or', 124, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (125, 'Ossetian, Ossetic (ирон æвзаг)', 0, 'os', 125, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (126, '(Eastern) Punjabi (ਪੰਜਾਬੀ)', 0, 'pa', 126, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (127, 'Pāli (पाऴि)', 0, 'pi', 127, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (128, 'Polish (język polski, polszczyzna)', 0, 'pl', 128, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (129, 'Pashto, Pushto (پښتو)', 0, 'ps', 129, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (130, 'Portuguese (Português)', 0, 'pt', 130, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (131, 'Quechua (Runa Simi, Kichwa)', 0, 'qu', 131, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (132, 'Romansh (rumantsch grischun)', 0, 'rm', 132, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (133, 'Kirundi (Ikirundi)', 0, 'rn', 133, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (134, 'Romanian (Română)', 0, 'ro', 134, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (135, 'Kinyarwanda (Ikinyarwanda)', 0, 'rw', 135, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (136, 'Sanskrit (Saṁskṛta) (संस्कृतम्)', 0, 'sa', 136, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (137, 'Sardinian (sardu)', 0, 'sc', 137, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (138, 'Sindhi (सिन्धी, سنڌي، سندھی)', 0, 'sd', 138, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (139, 'Northern Sami (Davvisámegiella)', 0, 'se', 139, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (140, 'Sango (yângâ tî sängö)', 0, 'sg', 140, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (141, 'Sinhalese, Sinhala (සිංහල)', 0, 'si', 141, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (142, 'Slovak (slovenčina, slovenský jazyk)', 0, 'sk', 142, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (143, 'Slovene (slovenski jezik, slovenščina)', 0, 'sl', 143, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (144, 'Samoan (gagana fa''a Samoa)', 0, 'sm', 144, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (145, 'Shona (chiShona)', 0, 'sn', 145, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (146, 'Somali (Soomaaliga, af Soomaali)', 0, 'so', 146, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (147, 'Albanian (Shqip)', 0, 'sq', 147, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (148, 'Serbian (српски језик)', 0, 'sr', 148, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (149, 'Swati (SiSwati)', 0, 'ss', 149, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (150, 'Southern Sotho (Sesotho)', 0, 'st', 150, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (151, 'Sundanese (Basa Sunda)', 0, 'su', 151, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (152, 'Swedish (svenska)', 0, 'sv', 152, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (153, 'Swahili (Kiswahili)', 0, 'sw', 153, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (154, 'Tamil (தமிழ்)', 0, 'ta', 154, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (155, 'Telugu (తెలుగు)', 0, 'te', 155, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (156, 'Tajik (тоҷикӣ, toçikī, تاجیکی)', 0, 'tg', 156, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (157, 'Thai (ไทย)', 0, 'th', 157, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (158, 'Tigrinya (ትግርኛ)', 0, 'ti', 158, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (159, 'Turkmen (Türkmen, Түркмен)', 0, 'tk', 159, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (160, 'Tagalog (Wikang Tagalog)', 0, 'tl', 160, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (161, 'Tswana (Setswana)', 0, 'tn', 161, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (162, 'Tonga (Tonga Islands) (faka Tonga)', 0, 'to', 162, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (163, 'Turkish (Türkçe)', 0, 'tr', 163, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (164, 'Tsonga (Xitsonga)', 0, 'ts', 164, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (165, 'Tatar (татар теле, tatar tele)', 0, 'tt', 165, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (166, 'Twi', 0, 'tw', 166, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (167, 'Tahitian (Reo Tahiti)', 0, 'ty', 167, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (168, 'Uyghur (ئۇيغۇرچە, Uyghurche)', 0, 'ug', 168, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (169, 'Urdu (اردو)', 0, 'ur', 169, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (170, 'Uzbek (Oʻzbek, Ўзбек, أۇزبېك)', 0, 'uz', 170, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (171, 'Venda (Tshivenḓa)', 0, 've', 171, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (172, 'Vietnamese (Tiếng Việt)', 0, 'vi', 172, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (173, 'Volapük', 0, 'vo', 173, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (174, 'Walloon (walon)', 0, 'wa', 174, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (175, 'Wolof (Wollof)', 0, 'wo', 175, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (176, 'Xhosa (isiXhosa)', 0, 'xh', 176, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (177, 'Yiddish (ייִדיש)', 0, 'yi', 177, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (178, 'Yoruba (Yorùbá)', 0, 'yo', 178, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (179, 'Zhuang, Chuang (Saɯ cueŋƅ, Saw cuengh)', 0, 'za', 179, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (180, 'Chinese (中文 (Zhōngwén), 汉语, 漢語)', 0, 'zh', 180, null);
INSERT INTO [lang] ([lang_id], [name], [enable], [code], [sort], [index]) VALUES (181, 'Zulu (isiZulu)', 0, 'zu', 181, null);
SET IDENTITY_INSERT [lang] OFF;

-- ----------------------------
-- Table structure for mail
-- ----------------------------
CREATE TABLE [mail] (
  [mail_id] BIGINT IDENTITY NOT NULL,
  [user_id] BIGINT NOT NULL,
  [mail] NVARCHAR(MAX) NOT NULL,
  [create] DATETIMEOFFSET NOT NULL,
  [send] DATETIMEOFFSET,
  [err] BIT NOT NULL,
  [err_text] NVARCHAR(MAX),
  [transport] VARCHAR(255) NOT NULL,
  PRIMARY KEY CLUSTERED ([mail_id])
);

EXEC sp_addextendedproperty
'MS_Description', N'Identifier',
'SCHEMA', N'dbo',
'TABLE', N'mail',
'COLUMN', N'mail_id';

EXEC sp_addextendedproperty
'MS_Description', N'User',
'SCHEMA', N'dbo',
'TABLE', N'mail',
'COLUMN', N'user_id';

EXEC sp_addextendedproperty
'MS_Description', N'Message',
'SCHEMA', N'dbo',
'TABLE', N'mail',
'COLUMN', N'mail';

EXEC sp_addextendedproperty
'MS_Description', N'Date created',
'SCHEMA', N'dbo',
'TABLE', N'mail',
'COLUMN', N'create';

EXEC sp_addextendedproperty
'MS_Description', N'Date sended',
'SCHEMA', N'dbo',
'TABLE', N'mail',
'COLUMN', N'send';

EXEC sp_addextendedproperty
'MS_Description', N'Is error',
'SCHEMA', N'dbo',
'TABLE', N'mail',
'COLUMN', N'err';

EXEC sp_addextendedproperty
'MS_Description', N'Error message',
'SCHEMA', N'dbo',
'TABLE', N'mail',
'COLUMN', N'err_text';

EXEC sp_addextendedproperty
'MS_Description', N'Transport',
'SCHEMA', N'dbo',
'TABLE', N'mail',
'COLUMN', N'transport';

EXEC sp_addextendedproperty
'MS_Description', N'Email',
'SCHEMA', N'dbo',
'TABLE', N'mail';

-- ----------------------------
-- Table structure for provider
-- ----------------------------
CREATE TABLE [provider] (
  [provider_id] BIGINT IDENTITY NOT NULL,
  [name] VARCHAR(255) NOT NULL,
  [enable] BIT NOT NULL,
  [master] BIT NOT NULL,
  [slave] BIT NOT NULL,
  [config] NVARCHAR(MAX) NOT NULL,
  PRIMARY KEY CLUSTERED ([provider_id])
);

EXEC sp_addextendedproperty
'MS_Description', N'Identifier',
'SCHEMA', N'dbo',
'TABLE', N'provider',
'COLUMN', N'provider_id';

EXEC sp_addextendedproperty
'MS_Description', N'Name',
'SCHEMA', N'dbo',
'TABLE', N'provider',
'COLUMN', N'name';

EXEC sp_addextendedproperty
'MS_Description', N'Profile enabled',
'SCHEMA', N'dbo',
'TABLE', N'provider',
'COLUMN', N'enable';

EXEC sp_addextendedproperty
'MS_Description', N'Can be used for the primary login',
'SCHEMA', N'dbo',
'TABLE', N'provider',
'COLUMN', N'master';

EXEC sp_addextendedproperty
'MS_Description', N'Can be used for the two-factor login',
'SCHEMA', N'dbo',
'TABLE', N'provider',
'COLUMN', N'slave';

EXEC sp_addextendedproperty
'MS_Description', N'Provider config',
'SCHEMA', N'dbo',
'TABLE', N'provider',
'COLUMN', N'config';

EXEC sp_addextendedproperty
'MS_Description', N'Login provider',
'SCHEMA', N'dbo',
'TABLE', N'provider';

-- ----------------------------
-- Table structure for redirect
-- ----------------------------
CREATE TABLE [redirect] (
  [redirect_id] BIGINT IDENTITY NOT NULL,
  [url] VARCHAR(4000) NOT NULL,
  [permanently] BIT NOT NULL,
  [redirect] VARCHAR(4000) NOT NULL,
  PRIMARY KEY CLUSTERED ([redirect_id])
);

EXEC sp_addextendedproperty
'MS_Description', N'Identifier',
'SCHEMA', N'dbo',
'TABLE', N'redirect',
'COLUMN', N'redirect_id';

EXEC sp_addextendedproperty
'MS_Description', N'Request URL',
'SCHEMA', N'dbo',
'TABLE', N'redirect',
'COLUMN', N'url';

EXEC sp_addextendedproperty
'MS_Description', N'301 or 302 http code',
'SCHEMA', N'dbo',
'TABLE', N'redirect',
'COLUMN', N'permanently';

EXEC sp_addextendedproperty
'MS_Description', N'New URL',
'SCHEMA', N'dbo',
'TABLE', N'redirect',
'COLUMN', N'redirect';

EXEC sp_addextendedproperty
'MS_Description', N'Redirect url',
'SCHEMA', N'dbo',
'TABLE', N'redirect';

-- ----------------------------
-- Table structure for role
-- ----------------------------
CREATE TABLE [role] (
  [role_id] BIGINT IDENTITY NOT NULL,
  [name] NVARCHAR(MAX) NOT NULL,
  [description] NVARCHAR(MAX) NOT NULL,
  PRIMARY KEY CLUSTERED ([role_id])
);

EXEC sp_addextendedproperty
'MS_Description', N'Identifier',
'SCHEMA', N'dbo',
'TABLE', N'role',
'COLUMN', N'role_id';

EXEC sp_addextendedproperty
'MS_Description', N'Name',
'SCHEMA', N'dbo',
'TABLE', N'role',
'COLUMN', N'name';

EXEC sp_addextendedproperty
'MS_Description', N'Description',
'SCHEMA', N'dbo',
'TABLE', N'role',
'COLUMN', N'description';

EXEC sp_addextendedproperty
'MS_Description', N'Roles list',
'SCHEMA', N'dbo',
'TABLE', N'role';

SET IDENTITY_INSERT [role] ON;
INSERT INTO [role] ([role_id], [name], [description]) VALUES (0, '{}', '{}');  -- Unregistered user
INSERT INTO [role] ([role_id], [name], [description]) VALUES (1, '{}', '{}'); -- Administrator
INSERT INTO [role] ([role_id], [name], [description]) VALUES (2, '{}', '{}'); -- Registered user
SET IDENTITY_INSERT [role] OFF;

-- ----------------------------
-- Table structure for route
-- ----------------------------
CREATE TABLE [route] (
  [route_id] BIGINT IDENTITY NOT NULL,
  [url] VARCHAR(4000) NOT NULL,
  [controller_id] BIGINT NOT NULL,
  [params] VARCHAR(255),
  [lang_id] BIGINT,
  PRIMARY KEY CLUSTERED ([route_id])
);

EXEC sp_addextendedproperty
'MS_Description', N'Identifier',
'SCHEMA', N'dbo',
'TABLE', N'route',
'COLUMN', N'route_id';

EXEC sp_addextendedproperty
'MS_Description', N'Request URL',
'SCHEMA', N'dbo',
'TABLE', N'route',
'COLUMN', N'url';

EXEC sp_addextendedproperty
'MS_Description', N'Controller ID',
'SCHEMA', N'dbo',
'TABLE', N'route',
'COLUMN', N'controller_id';

EXEC sp_addextendedproperty
'MS_Description', N'Params',
'SCHEMA', N'dbo',
'TABLE', N'route',
'COLUMN', N'params';

EXEC sp_addextendedproperty
'MS_Description', N'Language',
'SCHEMA', N'dbo',
'TABLE', N'route',
'COLUMN', N'lang_id';

EXEC sp_addextendedproperty
'MS_Description', N'Route map',
'SCHEMA', N'dbo',
'TABLE', N'route';

-- ----------------------------
-- Table structure for session
-- ----------------------------
CREATE TABLE [session] (
  [session_id] BIGINT IDENTITY NOT NULL,
  [user_id] BIGINT NOT NULL,
  [lang_id] BIGINT NOT NULL,
  [session] VARCHAR(512) NOT NULL,
  [data] VARBINARY(MAX) NOT NULL,
  [created] DATETIMEOFFSET NOT NULL,
  [last] DATETIMEOFFSET NOT NULL,
  [ip] VARCHAR(255) NOT NULL,
  [user_agent] NVARCHAR(MAX) NOT NULL,
  PRIMARY KEY CLUSTERED ([session_id])
);

EXEC sp_addextendedproperty
'MS_Description', N'Identifier',
'SCHEMA', N'dbo',
'TABLE', N'session',
'COLUMN', N'session_id';

EXEC sp_addextendedproperty
'MS_Description', N'User ID',
'SCHEMA', N'dbo',
'TABLE', N'session',
'COLUMN', N'user_id';

EXEC sp_addextendedproperty
'MS_Description', N'Language',
'SCHEMA', N'dbo',
'TABLE', N'session',
'COLUMN', N'lang_id';

EXEC sp_addextendedproperty
'MS_Description', N'Session key',
'SCHEMA', N'dbo',
'TABLE', N'session',
'COLUMN', N'session';

EXEC sp_addextendedproperty
'MS_Description', N'Session data',
'SCHEMA', N'dbo',
'TABLE', N'session',
'COLUMN', N'data';

EXEC sp_addextendedproperty
'MS_Description', N'Creation time',
'SCHEMA', N'dbo',
'TABLE', N'session',
'COLUMN', N'created';

EXEC sp_addextendedproperty
'MS_Description', N'Last change time',
'SCHEMA', N'dbo',
'TABLE', N'session',
'COLUMN', N'last';

EXEC sp_addextendedproperty
'MS_Description', N'Last IP client address',
'SCHEMA', N'dbo',
'TABLE', N'session',
'COLUMN', N'ip';

EXEC sp_addextendedproperty
'MS_Description', N'Last UserAgent client',
'SCHEMA', N'dbo',
'TABLE', N'session',
'COLUMN', N'user_agent';

EXEC sp_addextendedproperty
'MS_Description', N'Users session',
'SCHEMA', N'dbo',
'TABLE', N'session';

-- ----------------------------
-- Table structure for setting
-- ----------------------------
CREATE TABLE [setting] (
  [setting_id] BIGINT IDENTITY NOT NULL,
  [key] BIGINT NOT NULL,
  [data] NVARCHAR(MAX) NOT NULL,
  [key_text] VARCHAR(255) NOT NULL,
  [strict] NVARCHAR(MAX) NOT NULL,
  PRIMARY KEY CLUSTERED ([setting_id])
);

EXEC sp_addextendedproperty
'MS_Description', N'Identifier',
'SCHEMA', N'dbo',
'TABLE', N'setting',
'COLUMN', N'setting_id';

EXEC sp_addextendedproperty
'MS_Description', N'fnv1a_64(Key)',
'SCHEMA', N'dbo',
'TABLE', N'setting',
'COLUMN', N'key';

EXEC sp_addextendedproperty
'MS_Description', N'Data',
'SCHEMA', N'dbo',
'TABLE', N'setting',
'COLUMN', N'data';

EXEC sp_addextendedproperty
'MS_Description', N'Key',
'SCHEMA', N'dbo',
'TABLE', N'setting',
'COLUMN', N'key_text';

EXEC sp_addextendedproperty
'MS_Description', N'Limits on data',
'SCHEMA', N'dbo',
'TABLE', N'setting',
'COLUMN', N'strict';

EXEC sp_addextendedproperty
'MS_Description', N'General settings',
'SCHEMA', N'dbo',
'TABLE', N'setting';

SET IDENTITY_INSERT [setting] ON;
INSERT INTO [setting] ([setting_id], [key], [data], [key_text], [strict]) VALUES (1, 1441962092377564137, 'None', 'mail:provider', 'None|Sendmail|SMTP|File');
INSERT INTO [setting] ([setting_id], [key], [data], [key_text], [strict]) VALUES (2, -3979813852156915759, 'sendmail', 'mail:sendmail', '');
INSERT INTO [setting] ([setting_id], [key], [data], [key_text], [strict]) VALUES (3, -4738603782623769110, 'email', 'mail:file', '');
INSERT INTO [setting] ([setting_id], [key], [data], [key_text], [strict]) VALUES (4, -390595084051732771, 'localhost', 'mail:smtp:server', '');
INSERT INTO [setting] ([setting_id], [key], [data], [key_text], [strict]) VALUES (5, -1521500012746197243, '465', 'mail:smtp:port', '');
INSERT INTO [setting] ([setting_id], [key], [data], [key_text], [strict]) VALUES (6, 4706107683829871299, 'SSL/TLS', 'mail:smtp:tls', 'None|STARTTLS|SSL/TLS');
INSERT INTO [setting] ([setting_id], [key], [data], [key_text], [strict]) VALUES (7, -8449193462972437408, 'PLAIN', 'mail:smtp:auth', 'None|PLAIN|LOGIN|XOAUTH2');
INSERT INTO [setting] ([setting_id], [key], [data], [key_text], [strict]) VALUES (8, 1199393424318567565, '', 'mail:smtp:user', '');
INSERT INTO [setting] ([setting_id], [key], [data], [key_text], [strict]) VALUES (9, 2346365514808828621, '', 'mail:smtp:pwd', '');
SET IDENTITY_INSERT [setting] OFF;

-- ----------------------------
-- Table structure for user
-- ----------------------------
CREATE TABLE [user] (
  [user_id] BIGINT IDENTITY NOT NULL,
  [enable] BIT DEFAULT 0 NOT NULL,
  [lang_id] BIGINT NOT NULL,
  [create] DATETIMEOFFSET NOT NULL,
  [protect] BIT NOT NULL,
  [role_id] BIGINT NOT NULL,
  [data] NVARCHAR(MAX) NOT NULL,
  PRIMARY KEY CLUSTERED ([user_id])
);

EXEC sp_addextendedproperty
'MS_Description', N'Identifier',
'SCHEMA', N'dbo',
'TABLE', N'user',
'COLUMN', N'user_id';

EXEC sp_addextendedproperty
'MS_Description', N'User enable',
'SCHEMA', N'dbo',
'TABLE', N'user',
'COLUMN', N'enable';

EXEC sp_addextendedproperty
'MS_Description', N'Language',
'SCHEMA', N'dbo',
'TABLE', N'user',
'COLUMN', N'lang_id';

EXEC sp_addextendedproperty
'MS_Description', N'Creation time',
'SCHEMA', N'dbo',
'TABLE', N'user',
'COLUMN', N'create';

EXEC sp_addextendedproperty
'MS_Description', N'Protect account',
'SCHEMA', N'dbo',
'TABLE', N'user',
'COLUMN', N'protect';

EXEC sp_addextendedproperty
'MS_Description', N'User role',
'SCHEMA', N'dbo',
'TABLE', N'user',
'COLUMN', N'role_id';

EXEC sp_addextendedproperty
'MS_Description', N'Profile data',
'SCHEMA', N'dbo',
'TABLE', N'user',
'COLUMN', N'data';

EXEC sp_addextendedproperty
'MS_Description', N'Users list',
'SCHEMA', N'dbo',
'TABLE', N'user';

SET IDENTITY_INSERT [user] ON;
INSERT INTO [user] ([user_id], [enable], [lang_id], [create], [protect], [role_id], [data]) VALUES (0, 1, 0, '2023-01-01T00:00:00+00:00', 1, 0, '{}');
SET IDENTITY_INSERT [user] OFF;

-- ----------------------------
-- Table structure for user
-- ----------------------------
CREATE TABLE [user_provider] (
  [user_provider_id] BIGINT IDENTITY NOT NULL,
  [user_id] BIGINT NOT NULL,
  [provider_id] BIGINT NOT NULL,
  [enable] BIT NOT NULL,
  [data] NVARCHAR(MAX) NOT NULL,
  [update] DATETIMEOFFSET NOT NULL,
  [expire] DATETIMEOFFSET NOT NULL,
  PRIMARY KEY CLUSTERED ([user_provider_id])
);

EXEC sp_addextendedproperty
'MS_Description', N'Identifier',
'SCHEMA', N'dbo',
'TABLE', N'user_provider',
'COLUMN', N'user_provider_id';

EXEC sp_addextendedproperty
'MS_Description', N'User',
'SCHEMA', N'dbo',
'TABLE', N'user_provider',
'COLUMN', N'user_id';

EXEC sp_addextendedproperty
'MS_Description', N'Provider',
'SCHEMA', N'dbo',
'TABLE', N'user_provider',
'COLUMN', N'provider_id';

EXEC sp_addextendedproperty
'MS_Description', N'Enable',
'SCHEMA', N'dbo',
'TABLE', N'user_provider',
'COLUMN', N'enable';

EXEC sp_addextendedproperty
'MS_Description', N'Data',
'SCHEMA', N'dbo',
'TABLE', N'user_provider',
'COLUMN', N'data';

EXEC sp_addextendedproperty
'MS_Description', N'DateTime of update',
'SCHEMA', N'dbo',
'TABLE', N'user_provider',
'COLUMN', N'update';

EXEC sp_addextendedproperty
'MS_Description', N'Expires DateTime',
'SCHEMA', N'dbo',
'TABLE', N'user_provider',
'COLUMN', N'expire';

EXEC sp_addextendedproperty
'MS_Description', N'Use of the provider for the user',
'SCHEMA', N'dbo',
'TABLE', N'user_provider';

-- ----------------------------
-- Indexes structure for table access
-- ----------------------------
CREATE NONCLUSTERED INDEX [access_access_i] ON [access] ([access]);
CREATE NONCLUSTERED INDEX [access_controller_id_i] ON [access] ([controller_id]);
CREATE UNIQUE NONCLUSTERED INDEX [access_role_id_controller_id_u] ON [access] ([role_id], [controller_id]);
CREATE NONCLUSTERED INDEX [access_role_id_i] ON [access] ([role_id]);

-- ----------------------------
-- Indexes structure for table controller
-- ----------------------------
CREATE NONCLUSTERED INDEX [controller_action_id_i] ON [controller] ([action_id]);
CREATE NONCLUSTERED INDEX [controller_class_id_i] ON [controller] ([class_id]);
CREATE UNIQUE NONCLUSTERED INDEX [controller_module_id_class_id_action_id_u] ON [controller] ([module_id], [class_id], [action_id]);
CREATE NONCLUSTERED INDEX [controller_module_id_i] ON [controller] ([module_id]);
ALTER TABLE [controller] ADD CONSTRAINT [controller_expr_ch]
CHECK (
    (LEN([module]) = 0 AND LEN([class]) = 0 AND LEN([action]) = 0) OR
    (LEN([module]) > 0 AND LEN([class]) = 0 AND LEN([action]) = 0) OR
    (LEN([module]) > 0 AND LEN([class]) > 0 AND LEN([action]) = 0) OR
    (LEN([module]) > 0 AND LEN([class]) > 0 AND LEN([action]) > 0)
);

-- ----------------------------
-- Indexes structure for table lang
-- ----------------------------
CREATE NONCLUSTERED INDEX [lang_enable_i] ON [lang] ([enable]);
CREATE NONCLUSTERED INDEX [lang_code_i] ON [lang] ([code]);
CREATE NONCLUSTERED INDEX [lang_name_i] ON [lang] ([name]);
CREATE NONCLUSTERED INDEX [lang_index_i] ON [lang] ([index]);

-- ----------------------------
-- Indexes structure for table mail
-- ----------------------------
CREATE NONCLUSTERED INDEX [mail_err_i] ON [mail] ([err]);
CREATE NONCLUSTERED INDEX [mail_send_i] ON [mail] ([send]);
CREATE NONCLUSTERED INDEX [mail_user_id_i] ON [mail] ([user_id]);

-- ----------------------------
-- Indexes structure for table provider
-- ----------------------------
CREATE NONCLUSTERED INDEX [provider_enable_i] ON [provider] ([enable]);
CREATE NONCLUSTERED INDEX [provider_master_i] ON [provider] ([master]);
CREATE NONCLUSTERED INDEX [provider_slave_i] ON [provider] ([slave]);
CREATE UNIQUE NONCLUSTERED INDEX [provider_name_u] ON [provider] ([name]);

-- ----------------------------
-- Indexes structure for table redirect
-- ----------------------------
CREATE UNIQUE NONCLUSTERED INDEX [redirect_url_u] ON [redirect] ([url]);

-- ----------------------------
-- Indexes structure for table route
-- ----------------------------
CREATE NONCLUSTERED INDEX [route_controller_id_i] ON [route] ([controller_id]);
CREATE NONCLUSTERED INDEX [route_lang_id_i] ON [route] ([lang_id]);
CREATE NONCLUSTERED INDEX [route_params_i] ON [route] ([params]);
CREATE UNIQUE NONCLUSTERED INDEX [route_url_u] ON [route] ([url]);

-- ----------------------------
-- Indexes structure for table session
-- ----------------------------
CREATE UNIQUE NONCLUSTERED INDEX [session_session_u] ON [session] ([session]);
CREATE NONCLUSTERED INDEX [session_user_id_i] ON [session] ([user_id]);

-- ----------------------------
-- Indexes structure for table setting
-- ----------------------------
CREATE NONCLUSTERED INDEX [setting_key_i] ON [setting] ([key]);

-- ----------------------------
-- Indexes structure for table user
-- ----------------------------
CREATE NONCLUSTERED INDEX [user_enable_i] ON [user] ([enable]);
CREATE NONCLUSTERED INDEX [user_lang_id_i] ON [user] ([lang_id]);
CREATE NONCLUSTERED INDEX [user_protect_i] ON [user] ([protect]);
CREATE NONCLUSTERED INDEX [user_role_id_i] ON [user] ([role_id]);

-- ----------------------------
-- Indexes structure for table user_provider
-- ----------------------------
CREATE NONCLUSTERED INDEX [user_provider_enable_i] ON [user_provider] ([enable]);
CREATE NONCLUSTERED INDEX [user_provider_provider_id_i] ON [user_provider] ([provider_id]);
CREATE NONCLUSTERED INDEX [user_provider_user_id_i] ON [user_provider] ([user_id]);
CREATE UNIQUE NONCLUSTERED INDEX [user_provider_user_id_provider_id_u] ON [user_provider] ([user_id], [provider_id]);

-- ----------------------------
-- Foreign Keys structure
-- ----------------------------
ALTER TABLE [access] ADD FOREIGN KEY ([controller_id]) REFERENCES [controller] ([controller_id]);
ALTER TABLE [access] ADD FOREIGN KEY ([role_id]) REFERENCES [role] ([role_id]);
ALTER TABLE [mail] ADD FOREIGN KEY ([user_id]) REFERENCES [user] ([user_id]);
ALTER TABLE [route] ADD FOREIGN KEY ([controller_id]) REFERENCES [controller] ([controller_id]);
ALTER TABLE [route] ADD FOREIGN KEY ([lang_id]) REFERENCES [lang] ([lang_id]);
ALTER TABLE [session] ADD FOREIGN KEY ([user_id]) REFERENCES [user] ([user_id]);
ALTER TABLE [user] ADD FOREIGN KEY ([lang_id]) REFERENCES [lang] ([lang_id]);
ALTER TABLE [user] ADD FOREIGN KEY ([role_id]) REFERENCES [role] ([role_id]);
ALTER TABLE [user_provider] ADD FOREIGN KEY ([provider_id]) REFERENCES [provider] ([provider_id]);
ALTER TABLE [user_provider] ADD FOREIGN KEY ([user_id]) REFERENCES [user] ([user_id]);
GO
-- ----------------------------
-- Trigers for lang.index structure
-- ----------------------------
CREATE TRIGGER [lang_insert_t] ON [lang] INSTEAD OF INSERT AS
BEGIN
  SET NOCOUNT ON;
  INSERT INTO [lang] ([name], [enable], [code], [sort], [index])
  SELECT i.[name], 0, i.[code], i.[sort], NULL
  FROM INSERTED i;
END;
GO
CREATE TRIGGER [lang_update_t] ON [lang] INSTEAD OF UPDATE AS
BEGIN
  SET NOCOUNT ON;
	UPDATE [lang]
	SET
		[name] = i.[name],
		[enable] = i.[enable],
		[code] = i.[code],
		[sort] = i.[sort],
    [index] = 
		CASE 
      WHEN d.[index] IS NOT NULL THEN d.[index]
      ELSE 
				CASE
					WHEN i.[index] IS NULL THEN NULL
					ELSE ISNULL(n.[index], -1) + 1
				END
		END
	FROM 
		INSERTED i
		INNER JOIN DELETED d ON i.[lang_id]=d.[lang_id]
		LEFT JOIN (SELECT MAX([index]) AS [index] FROM [lang]) n ON 1=1
	WHERE [lang].[lang_id]=i.[lang_id];
END;
GO