SET NAMES utf8mb4;
SET FOREIGN_KEY_CHECKS = 0;

-- ----------------------------
-- Table structure for access
-- ----------------------------
CREATE TABLE `access` (
  `access_id` bigint NOT NULL AUTO_INCREMENT COMMENT 'Identifier',
  `role_id` bigint NOT NULL COMMENT 'Role ID',
  `access` tinyint(1) NOT NULL COMMENT 'Access flag',
  `controller_id` bigint NOT NULL COMMENT 'Controller ID',
  PRIMARY KEY (`access_id`),
  UNIQUE KEY `role_id` (`role_id`,`controller_id`),
  KEY `access` (`access`),
  KEY `controller_id` (`controller_id`),
  CONSTRAINT `access_ibfk_1` FOREIGN KEY (`role_id`) REFERENCES `role` (`role_id`),
  CONSTRAINT `access_ibfk_2` FOREIGN KEY (`controller_id`) REFERENCES `controller` (`controller_id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci COMMENT='Access to controllers';

-- ----------------------------
-- Records of access
-- ----------------------------
INSERT INTO `access` VALUES (1, 0, 1, 1);
INSERT INTO `access` VALUES (2, 0, 1, 4);
INSERT INTO `access` VALUES (3, 0, 1, 5);

-- ----------------------------
-- Table structure for controller
-- ----------------------------
CREATE TABLE `controller` (
  `controller_id` bigint NOT NULL AUTO_INCREMENT COMMENT 'Identifier',
  `module` varchar(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci NOT NULL COMMENT 'Module',
  `class` varchar(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci NOT NULL COMMENT 'Class',
  `action` varchar(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci NOT NULL COMMENT 'Action (controller)',
  `description` json NOT NULL COMMENT 'Description',
  `module_id` bigint NOT NULL COMMENT 'fnv1a_64 hash from module',
  `class_id` bigint NOT NULL COMMENT 'fnv1a_64 hash from class',
  `action_id` bigint NOT NULL COMMENT 'fnv1a_64 hash from action',
  PRIMARY KEY (`controller_id`),
  UNIQUE KEY `module_id_2` (`module_id`,`class_id`,`action_id`),
  KEY `module_id` (`module_id`),
  KEY `class_id` (`class_id`),
  KEY `action_id` (`action_id`),
  CONSTRAINT `controller_expr_ch` CHECK ((((length(`module`) = 0) and (length(`class`) = 0) and (length(`action`) = 0)) or ((length(`module`) > 0) and (length(`class`) = 0) and (length(`action`) = 0)) or ((length(`module`) > 0) and (length(`class`) > 0) and (length(`action`) = 0)) or ((length(`module`) > 0) and (length(`class`) > 0) and (length(`action`) > 0))))
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci COMMENT='Controllers list';

-- ----------------------------
-- Records of controller
-- ----------------------------
INSERT INTO `controller` (`controller_id`, `module`, `class`, `action`, `description`, `module_id`, `class_id`, `action_id`) VALUES (1, 'index', '', '', '{}', -8948777187306027381, -3750763034362895579, -3750763034362895579);
INSERT INTO `controller` (`controller_id`, `module`, `class`, `action`, `description`, `module_id`, `class_id`, `action_id`) VALUES (2, 'index', 'index', 'index', '{}', -8948777187306027381, -8948777187306027381, -8948777187306027381);
INSERT INTO `controller` (`controller_id`, `module`, `class`, `action`, `description`, `module_id`, `class_id`, `action_id`) VALUES (3, 'index', 'index', 'not_found', '{}', -8948777187306027381, -8948777187306027381, -1573091631220776463);
INSERT INTO `controller` (`controller_id`, `module`, `class`, `action`, `description`, `module_id`, `class_id`, `action_id`) VALUES (4, 'admin', 'index', '', '{}', -1887597591324883884, -8948777187306027381, -3750763034362895579);
INSERT INTO `controller` (`controller_id`, `module`, `class`, `action`, `description`, `module_id`, `class_id`, `action_id`) VALUES (5, 'admin', 'login', '', '{}', -1887597591324883884, 272289342528891346, -3750763034362895579);

-- ----------------------------
-- Table structure for lang
-- ----------------------------
CREATE TABLE `lang` (
  `lang_id` bigint NOT NULL AUTO_INCREMENT COMMENT 'Identifier',
  `name` varchar(255) NOT NULL COMMENT 'Language name',
  `enable` tinyint(1) NOT NULL COMMENT 'Enable',
  `lang` varchar(2) NOT NULL COMMENT 'ISO 639-1 : uk - ukrainian, en - english',
  `sort` bigint NOT NULL COMMENT 'Sort order',
  `index` bigint DEFAULT NULL COMMENT 'Index in JSON type field db',
  PRIMARY KEY (`lang_id`),
  KEY `enable` (`enable`),
  KEY `index` (`index`),
  KEY `lang` (`lang`),
  KEY `name` (`name`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci COMMENT='Languages';

-- ----------------------------
-- Records of lang
-- ----------------------------
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (0, 'English', 1, 'en', 0, NULL);
UPDATE `lang` SET `lang_id`= 0 WHERE `lang_id` = 1; 
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (1, 'Ukrainian (Українська)', 1, 'uk', 1, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (2, 'Afar (Afaraf)', 0, 'aa', 2, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (3, 'Abkhaz (аҧсуа бызшәа, аҧсшәа)', 0, 'ab', 3, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (4, 'Avestan (avesta)', 0, 'ae', 4, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (5, 'Afrikaans', 0, 'af', 5, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (6, 'Akan', 0, 'ak', 6, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (7, 'Amharic (አማርኛ)', 0, 'am', 7, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (8, 'Aragonese (aragonés)', 0, 'an', 8, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (9, 'Arabic (العربية)', 0, 'ar', 9, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (10, 'Assamese (অসমীয়া)', 0, 'as', 10, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (11, 'Avaric (авар мацӀ, магӀарул мацӀ)', 0, 'av', 11, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (12, 'Aymara (aymar aru)', 0, 'ay', 12, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (13, 'Azerbaijani (azərbaycan dili)', 0, 'az', 13, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (14, 'Bashkir (башҡорт теле)', 0, 'ba', 14, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (15, 'Bulgarian (български език)', 0, 'bg', 15, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (16, 'Bihari (भोजपुरी)', 0, 'bh', 16, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (17, 'Bislama', 0, 'bi', 17, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (18, 'Bambara (bamanankan)', 0, 'bm', 18, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (19, 'Bengali, Bangla (বাংলা)', 0, 'bn', 19, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (20, 'Tibetan Standard, Tibetan, Central (བོད་ཡིག)', 0, 'bo', 20, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (21, 'Breton (brezhoneg)', 0, 'br', 21, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (22, 'Bosnian (bosanski jezik)', 0, 'bs', 22, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (23, 'Catalan (català)', 0, 'ca', 23, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (24, 'Chechen (нохчийн мотт)', 0, 'ce', 24, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (25, 'Chamorro (Chamoru)', 0, 'ch', 25, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (26, 'Corsican (corsu, lingua corsa)', 0, 'co', 26, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (27, 'Cree (ᓀᐦᐃᔭᐍᐏᐣ)', 0, 'cr', 27, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (28, 'Czech (čeština, český jazyk)', 0, 'cs', 28, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (29, 'Old Church Slavonic, Church Slavonic, Old Bulgarian (ѩзыкъ словѣньскъ)', 0, 'cu', 29, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (30, 'Chuvash (чӑваш чӗлхи)', 0, 'cv', 30, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (31, 'Welsh (Cymraeg)', 0, 'cy', 31, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (32, 'Danish (dansk)', 0, 'da', 32, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (33, 'German (Deutsch)', 0, 'de', 33, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (34, 'Divehi, Dhivehi, Maldivian (ދިވެހި)', 0, 'dv', 34, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (35, 'Dzongkha (རྫོང་ཁ)', 0, 'dz', 35, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (36, 'Ewe (Eʋegbe)', 0, 'ee', 36, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (37, 'Greek (modern) (ελληνικά)', 0, 'el', 37, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (38, 'Esperanto', 0, 'eo', 38, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (39, 'Spanish (Español)', 0, 'es', 39, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (40, 'Estonian (eesti, eesti keel)', 0, 'et', 40, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (41, 'Basque (euskara, euskera)', 0, 'eu', 41, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (42, 'Persian (Farsi) (فارسی)', 0, 'fa', 42, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (43, 'Fula, Fulah, Pulaar, Pular (Fulfulde, Pulaar, Pular)', 0, 'ff', 43, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (44, 'Finnish (suomi, suomen kieli)', 0, 'fi', 44, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (45, 'Fijian (vosa Vakaviti)', 0, 'fj', 45, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (46, 'Faroese (føroyskt)', 0, 'fo', 46, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (47, 'French (français, langue française)', 0, 'fr', 47, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (48, 'Western Frisian (Frysk)', 0, 'fy', 48, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (49, 'Irish (Gaeilge)', 0, 'ga', 49, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (50, 'Scottish Gaelic, Gaelic (Gàidhlig)', 0, 'gd', 50, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (51, 'Galician (galego)', 0, 'gl', 51, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (52, 'Guaraní (Avañe''ẽ)', 0, 'gn', 52, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (53, 'Gujarati (ગુજરાતી)', 0, 'gu', 53, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (54, 'Manx (Gaelg, Gailck)', 0, 'gv', 54, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (55, 'Hausa ((Hausa) هَوُسَ)', 0, 'ha', 55, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (56, 'Hebrew (modern) (עברית)', 0, 'he', 56, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (57, 'Hindi (हिन्दी, हिंदी)', 0, 'hi', 57, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (58, 'Hiri Motu', 0, 'ho', 58, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (59, 'Croatian (hrvatski jezik)', 0, 'hr', 59, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (60, 'Haitian, Haitian Creole (Kreyòl ayisyen)', 0, 'ht', 60, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (61, 'Hungarian (magyar)', 0, 'hu', 61, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (62, 'Armenian (Հայերեն)', 0, 'hy', 62, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (63, 'Herero (Otjiherero)', 0, 'hz', 63, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (64, 'Interlingua', 0, 'ia', 64, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (65, 'Indonesian (Bahasa Indonesia)', 0, 'id', 65, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (66, 'Interlingue (Originally called Occidental; then Interlingue after WWII)', 0, 'ie', 66, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (67, 'Igbo (Asụsụ Igbo)', 0, 'ig', 67, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (68, 'Nuosu (ꆈꌠ꒿ Nuosuhxop)', 0, 'ii', 68, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (69, 'Inupiaq (Iñupiaq, Iñupiatun)', 0, 'ik', 69, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (70, 'Ido', 0, 'io', 70, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (71, 'Icelandic (Íslenska)', 0, 'is', 71, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (72, 'Italian (Italiano)', 0, 'it', 72, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (73, 'Inuktitut (ᐃᓄᒃᑎᑐᑦ)', 0, 'iu', 73, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (74, 'Japanese (日本語 (にほんご))', 0, 'ja', 74, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (75, 'Javanese (ꦧꦱꦗꦮ, Basa Jawa)', 0, 'jv', 75, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (76, 'Georgian (ქართული)', 0, 'ka', 76, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (77, 'Kongo (Kikongo)', 0, 'kg', 77, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (78, 'Kikuyu, Gikuyu (Gĩkũyũ)', 0, 'ki', 78, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (79, 'Kwanyama, Kuanyama (Kuanyama)', 0, 'kj', 79, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (80, 'Kazakh (қазақ тілі)', 0, 'kk', 80, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (81, 'Kalaallisut, Greenlandic (kalaallisut, kalaallit oqaasii)', 0, 'kl', 81, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (82, 'Khmer (ខ្មែរ, ខេមរភាសា, ភាសាខ្មែរ)', 0, 'km', 82, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (83, 'Kannada (ಕನ್ನಡ)', 0, 'kn', 83, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (84, 'Korean (한국어)', 0, 'ko', 84, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (85, 'Kanuri', 0, 'kr', 85, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (86, 'Kashmiri (कश्मीरी, کشمیری)', 0, 'ks', 86, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (87, 'Kurdish (Kurdî, كوردی)', 0, 'ku', 87, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (88, 'Komi (коми кыв)', 0, 'kv', 88, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (89, 'Cornish (Kernewek)', 0, 'kw', 89, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (90, 'Kyrgyz (Кыргызча, Кыргыз тили)', 0, 'ky', 90, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (91, 'Latin (latine, lingua latina)', 0, 'la', 91, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (92, 'Luxembourgish, Letzeburgesch (Lëtzebuergesch)', 0, 'lb', 92, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (93, 'Ganda (Luganda)', 0, 'lg', 93, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (94, 'Limburgish, Limburgan, Limburger (Limburgs)', 0, 'li', 94, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (95, 'Lingala (Lingála)', 0, 'ln', 95, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (96, 'Lao (ພາສາລາວ)', 0, 'lo', 96, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (97, 'Lithuanian (lietuvių kalba)', 0, 'lt', 97, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (98, 'Luba-Katanga (Tshiluba)', 0, 'lu', 98, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (99, 'Latvian (latviešu valoda)', 0, 'lv', 99, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (100, 'Malagasy (fiteny malagasy)', 0, 'mg', 100, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (101, 'Marshallese (Kajin M̧ajeļ)', 0, 'mh', 101, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (102, 'Māori (te reo Māori)', 0, 'mi', 102, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (103, 'Macedonian (македонски јазик)', 0, 'mk', 103, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (104, 'Malayalam (മലയാളം)', 0, 'ml', 104, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (105, 'Mongolian (Монгол хэл)', 0, 'mn', 105, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (106, 'Marathi (Marāṭhī) (मराठी)', 0, 'mr', 106, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (107, 'Malay (bahasa Melayu, بهاس ملايو)', 0, 'ms', 107, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (108, 'Maltese (Malti)', 0, 'mt', 108, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (109, 'Burmese (ဗမာစာ)', 0, 'my', 109, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (110, 'Nauruan (Dorerin Naoero)', 0, 'na', 110, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (111, 'Norwegian Bokmål (Norsk bokmål)', 0, 'nb', 111, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (112, 'Northern Ndebele (isiNdebele)', 0, 'nd', 112, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (113, 'Nepali (नेपाली)', 0, 'ne', 113, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (114, 'Ndonga (Owambo)', 0, 'ng', 114, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (115, 'Dutch (Nederlands, Vlaams)', 0, 'nl', 115, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (116, 'Norwegian Nynorsk (Norsk nynorsk)', 0, 'nn', 116, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (117, 'Norwegian (Norsk)', 0, 'no', 117, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (118, 'Southern Ndebele (isiNdebele)', 0, 'nr', 118, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (119, 'Navajo, Navaho (Diné bizaad)', 0, 'nv', 119, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (120, 'Chichewa, Chewa, Nyanja (chiCheŵa, chinyanja)', 0, 'ny', 120, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (121, 'Occitan (occitan, lenga d''òc)', 0, 'oc', 121, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (122, 'Ojibwe, Ojibwa (ᐊᓂᔑᓈᐯᒧᐎᓐ)', 0, 'oj', 122, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (123, 'Oromo (Afaan Oromoo)', 0, 'om', 123, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (124, 'Oriya (ଓଡ଼ିଆ)', 0, 'or', 124, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (125, 'Ossetian, Ossetic (ирон æвзаг)', 0, 'os', 125, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (126, '(Eastern) Punjabi (ਪੰਜਾਬੀ)', 0, 'pa', 126, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (127, 'Pāli (पाऴि)', 0, 'pi', 127, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (128, 'Polish (język polski, polszczyzna)', 0, 'pl', 128, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (129, 'Pashto, Pushto (پښتو)', 0, 'ps', 129, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (130, 'Portuguese (Português)', 0, 'pt', 130, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (131, 'Quechua (Runa Simi, Kichwa)', 0, 'qu', 131, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (132, 'Romansh (rumantsch grischun)', 0, 'rm', 132, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (133, 'Kirundi (Ikirundi)', 0, 'rn', 133, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (134, 'Romanian (Română)', 0, 'ro', 134, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (135, 'Kinyarwanda (Ikinyarwanda)', 0, 'rw', 135, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (136, 'Sanskrit (Saṁskṛta) (संस्कृतम्)', 0, 'sa', 136, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (137, 'Sardinian (sardu)', 0, 'sc', 137, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (138, 'Sindhi (सिन्धी, سنڌي، سندھی)', 0, 'sd', 138, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (139, 'Northern Sami (Davvisámegiella)', 0, 'se', 139, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (140, 'Sango (yângâ tî sängö)', 0, 'sg', 140, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (141, 'Sinhalese, Sinhala (සිංහල)', 0, 'si', 141, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (142, 'Slovak (slovenčina, slovenský jazyk)', 0, 'sk', 142, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (143, 'Slovene (slovenski jezik, slovenščina)', 0, 'sl', 143, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (144, 'Samoan (gagana fa''a Samoa)', 0, 'sm', 144, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (145, 'Shona (chiShona)', 0, 'sn', 145, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (146, 'Somali (Soomaaliga, af Soomaali)', 0, 'so', 146, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (147, 'Albanian (Shqip)', 0, 'sq', 147, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (148, 'Serbian (српски језик)', 0, 'sr', 148, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (149, 'Swati (SiSwati)', 0, 'ss', 149, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (150, 'Southern Sotho (Sesotho)', 0, 'st', 150, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (151, 'Sundanese (Basa Sunda)', 0, 'su', 151, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (152, 'Swedish (svenska)', 0, 'sv', 152, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (153, 'Swahili (Kiswahili)', 0, 'sw', 153, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (154, 'Tamil (தமிழ்)', 0, 'ta', 154, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (155, 'Telugu (తెలుగు)', 0, 'te', 155, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (156, 'Tajik (тоҷикӣ, toçikī, تاجیکی)', 0, 'tg', 156, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (157, 'Thai (ไทย)', 0, 'th', 157, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (158, 'Tigrinya (ትግርኛ)', 0, 'ti', 158, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (159, 'Turkmen (Türkmen, Түркмен)', 0, 'tk', 159, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (160, 'Tagalog (Wikang Tagalog)', 0, 'tl', 160, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (161, 'Tswana (Setswana)', 0, 'tn', 161, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (162, 'Tonga (Tonga Islands) (faka Tonga)', 0, 'to', 162, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (163, 'Turkish (Türkçe)', 0, 'tr', 163, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (164, 'Tsonga (Xitsonga)', 0, 'ts', 164, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (165, 'Tatar (татар теле, tatar tele)', 0, 'tt', 165, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (166, 'Twi', 0, 'tw', 166, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (167, 'Tahitian (Reo Tahiti)', 0, 'ty', 167, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (168, 'Uyghur (ئۇيغۇرچە, Uyghurche)', 0, 'ug', 168, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (169, 'Urdu (اردو)', 0, 'ur', 169, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (170, 'Uzbek (Oʻzbek, Ўзбек, أۇزبېك)', 0, 'uz', 170, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (171, 'Venda (Tshivenḓa)', 0, 've', 171, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (172, 'Vietnamese (Tiếng Việt)', 0, 'vi', 172, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (173, 'Volapük', 0, 'vo', 173, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (174, 'Walloon (walon)', 0, 'wa', 174, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (175, 'Wolof (Wollof)', 0, 'wo', 175, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (176, 'Xhosa (isiXhosa)', 0, 'xh', 176, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (177, 'Yiddish (ייִדיש)', 0, 'yi', 177, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (178, 'Yoruba (Yorùbá)', 0, 'yo', 178, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (179, 'Zhuang, Chuang (Saɯ cueŋƅ, Saw cuengh)', 0, 'za', 179, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (180, 'Chinese (中文 (Zhōngwén), 汉语, 漢語)', 0, 'zh', 180, NULL);
INSERT INTO `lang` (`lang_id`, `name`, `enable`, `lang`, `sort`, `index`) VALUES (181, 'Zulu (isiZulu)', 0, 'zu', 181, NULL);

-- ----------------------------
-- Table structure for mail
-- ----------------------------
CREATE TABLE `mail` (
  `mail_id` bigint NOT NULL AUTO_INCREMENT COMMENT 'Identifier',
  `user_id` bigint NOT NULL COMMENT 'User',
  `mail` json NOT NULL COMMENT 'Message',
  `create` timestamp NOT NULL COMMENT 'Date created',
  `send` timestamp NULL DEFAULT NULL COMMENT 'Date sended',
  `err` tinyint(1) NOT NULL COMMENT 'Is error',
  `err_text` longtext CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci COMMENT 'Error message',
  `transport` varchar(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci NOT NULL COMMENT 'Transport',
  PRIMARY KEY (`mail_id`),
  KEY `err` (`err`),
  KEY `send` (`send`),
  KEY `user_id` (`user_id`),
  CONSTRAINT `mail_ibfk_1` FOREIGN KEY (`user_id`) REFERENCES `user` (`user_id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci COMMENT='Email';


-- ----------------------------
-- Table structure for provider
-- ----------------------------
CREATE TABLE `provider` (
  `provider_id` bigint NOT NULL AUTO_INCREMENT COMMENT 'Identifier',
  `name` varchar(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci NOT NULL COMMENT 'Name',
  `enable` tinyint(1) NOT NULL COMMENT 'Profile enabled',
  `master` tinyint(1) NOT NULL COMMENT 'Can be used for the primary login',
  `slave` tinyint(1) NOT NULL COMMENT 'Can be used for the two-factor login',
  `config` json NOT NULL COMMENT 'Provider config',
  PRIMARY KEY (`provider_id`),
  UNIQUE KEY `name` (`name`),
  KEY `enable` (`enable`),
  KEY `master` (`master`),
  KEY `slave` (`slave`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci COMMENT='Login provider';

-- ----------------------------
-- Table structure for redirect
-- ----------------------------
CREATE TABLE `redirect` (
  `redirect_id` bigint NOT NULL AUTO_INCREMENT COMMENT 'Identifier',
  `url` varchar(768) CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci NOT NULL COMMENT 'Request URL',
  `permanently` tinyint(1) NOT NULL COMMENT '301 or 302 http code',
  `redirect` varchar(768) CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci NOT NULL COMMENT 'New URL',
  PRIMARY KEY (`redirect_id`),
  UNIQUE KEY `url` (`url`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci COMMENT='Redirect url';

-- ----------------------------
-- Table structure for role
-- ----------------------------
CREATE TABLE `role` (
  `role_id` bigint NOT NULL AUTO_INCREMENT COMMENT 'Identifier',
  `name` json NOT NULL COMMENT 'Name',
  `description` json NOT NULL COMMENT 'Description',
  PRIMARY KEY (`role_id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci COMMENT='Roles list';

-- ----------------------------
-- Records of role
-- ----------------------------
INSERT INTO `role` (`role_id`, `name`, `description`) VALUES (0, '{}', '{}');
UPDATE `role` SET `role_id` = 0 WHERE `role_id` = 1;
INSERT INTO `role` (`role_id`, `name`, `description`) VALUES (1, '{}', '{}');
INSERT INTO `role` (`role_id`, `name`, `description`) VALUES (2, '{}', '{}');

-- ----------------------------
-- Table structure for route
-- ----------------------------
CREATE TABLE `route` (
  `route_id` bigint NOT NULL AUTO_INCREMENT COMMENT 'Identifier',
  `url` varchar(768) CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci NOT NULL COMMENT 'Request URL',
  `controller_id` bigint NOT NULL COMMENT 'Controller ID',
  `params` varchar(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci DEFAULT NULL COMMENT 'Params',
  `lang_id` bigint NOT NULL COMMENT 'Language',
  PRIMARY KEY (`route_id`),
  UNIQUE KEY `url` (`url`),
  KEY `params` (`params`),
  KEY `controller_id` (`controller_id`),
  KEY `lang_id` (`lang_id`),
  CONSTRAINT `route_ibfk_1` FOREIGN KEY (`controller_id`) REFERENCES `controller` (`controller_id`),
  CONSTRAINT `route_ibfk_2` FOREIGN KEY (`lang_id`) REFERENCES `lang` (`lang_id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci COMMENT='Route map';

-- ----------------------------
-- Table structure for session
-- ----------------------------
CREATE TABLE `session` (
  `session_id` bigint NOT NULL AUTO_INCREMENT COMMENT 'Identifier',
  `user_id` bigint NOT NULL COMMENT 'User ID',
  `lang_id` bigint NOT NULL COMMENT 'Language',
  `session` varchar(512) CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci NOT NULL COMMENT 'Session key',
  `data` longblob NOT NULL COMMENT 'Session data',
  `created` timestamp NOT NULL COMMENT 'Creation time',
  `last` timestamp NOT NULL COMMENT 'Last change time',
  `ip` varchar(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci NOT NULL COMMENT 'Last IP client address',
  `user_agent` longtext CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci NOT NULL COMMENT 'Last UserAgent client',
  PRIMARY KEY (`session_id`) USING BTREE,
  UNIQUE KEY `session` (`session`),
  KEY `user_id` (`user_id`),
  CONSTRAINT `session_ibfk_1` FOREIGN KEY (`user_id`) REFERENCES `user` (`user_id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci COMMENT='Users session';

-- ----------------------------
-- Table structure for setting
-- ----------------------------
CREATE TABLE `setting` (
  `setting_id` bigint NOT NULL AUTO_INCREMENT COMMENT 'Identifier',
  `key` bigint NOT NULL COMMENT 'fnv1a_64(Key)',
  `data` longtext CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci NOT NULL COMMENT 'Data',
  `key_text` varchar(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci NOT NULL COMMENT 'Key',
  `strict` longtext CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci NOT NULL COMMENT 'Limits on data',
  PRIMARY KEY (`setting_id`),
  KEY `key` (`key`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci COMMENT='General settings';

-- ----------------------------
-- Records of setting
-- ----------------------------
INSERT INTO `setting` (`setting_id`, `key`, `data`, `key_text`, `strict`) VALUES (1, 1441962092377564137, 'None', 'mail:provider', 'None|Sendmail|SMTP|File');
INSERT INTO `setting` (`setting_id`, `key`, `data`, `key_text`, `strict`) VALUES (2, -3979813852156915759, 'sendmail', 'mail:sendmail', '');
INSERT INTO `setting` (`setting_id`, `key`, `data`, `key_text`, `strict`) VALUES (3, -4738603782623769110, 'email', 'mail:file', '');
INSERT INTO `setting` (`setting_id`, `key`, `data`, `key_text`, `strict`) VALUES (4, -390595084051732771, 'localhost', 'mail:smtp:server', '');
INSERT INTO `setting` (`setting_id`, `key`, `data`, `key_text`, `strict`) VALUES (5, -1521500012746197243, '465', 'mail:smtp:port', '');
INSERT INTO `setting` (`setting_id`, `key`, `data`, `key_text`, `strict`) VALUES (6, 4706107683829871299, 'SSL/TLS', 'mail:smtp:tls', 'None|STARTTLS|SSL/TLS');
INSERT INTO `setting` (`setting_id`, `key`, `data`, `key_text`, `strict`) VALUES (7, -8449193462972437408, 'PLAIN', 'mail:smtp:auth', 'None|PLAIN|LOGIN|XOAUTH2');
INSERT INTO `setting` (`setting_id`, `key`, `data`, `key_text`, `strict`) VALUES (8, 1199393424318567565, '', 'mail:smtp:user', '');
INSERT INTO `setting` (`setting_id`, `key`, `data`, `key_text`, `strict`) VALUES (9, 2346365514808828621, '', 'mail:smtp:pwd', '');

-- ----------------------------
-- Table structure for user
-- ----------------------------
CREATE TABLE `user` (
  `user_id` bigint NOT NULL AUTO_INCREMENT COMMENT 'Identifier',
  `enable` tinyint(1) NOT NULL COMMENT 'User enable',
  `lang_id` bigint NOT NULL COMMENT 'Language',
  `create` timestamp NOT NULL COMMENT 'Creation time',
  `protect` tinyint(1) NOT NULL COMMENT 'Protect account',
  `role_id` bigint NOT NULL COMMENT 'User role',
  `data` json NOT NULL COMMENT 'Profile data',
  PRIMARY KEY (`user_id`),
  KEY `enable` (`enable`),
  KEY `protect` (`protect`),
  KEY `role_id` (`role_id`),
  KEY `lang_id` (`lang_id`),
  CONSTRAINT `user_ibfk_2` FOREIGN KEY (`role_id`) REFERENCES `role` (`role_id`),
  CONSTRAINT `user_ibfk_3` FOREIGN KEY (`lang_id`) REFERENCES `lang` (`lang_id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci COMMENT='Users list';

-- ----------------------------
-- Records of user
-- ----------------------------
INSERT INTO `user` (`user_id`, `enable`, `lang_id`, `create`, `protect`, `role_id`, `data`) VALUES (0, 1, 0, '2023-01-01 02:00:00', 1, 0, '{}');
UPDATE `user` SET `user_id` = 0 WHERE `user_id` = 1;
ALTER TABLE `user` AUTO_INCREMENT = 1;
-- ----------------------------
-- Table structure for user_provider
-- ----------------------------
CREATE TABLE `user_provider` (
  `user_provider_id` bigint NOT NULL AUTO_INCREMENT COMMENT 'Identifier',
  `user_id` bigint NOT NULL COMMENT 'User',
  `provider_id` bigint NOT NULL COMMENT 'Provider',
  `enable` tinyint(1) NOT NULL COMMENT 'Enable',
  `data` json NOT NULL COMMENT 'Data',
  `update` timestamp NOT NULL COMMENT 'DateTime of update',
  `expire` timestamp NOT NULL COMMENT 'Expires DateTime',
  PRIMARY KEY (`user_provider_id`) USING BTREE,
  UNIQUE KEY `user_id` (`user_id`,`provider_id`),
  KEY `enable` (`enable`),
  KEY `provider_id` (`provider_id`),
  CONSTRAINT `user_provider_ibfk_1` FOREIGN KEY (`user_id`) REFERENCES `user` (`user_id`),
  CONSTRAINT `user_provider_ibfk_2` FOREIGN KEY (`provider_id`) REFERENCES `provider` (`provider_id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci COMMENT='Use of the provider for the user';

-- ----------------------------
-- Triggers structure for table lang
-- ----------------------------
delimiter ;;
CREATE TRIGGER `tiny`.`trigger_lang_insert_row` BEFORE INSERT ON `lang` FOR EACH ROW BEGIN
    SET NEW.`index` = NULL;
END
;;
delimiter ;

-- ----------------------------
-- Triggers structure for table lang
-- ----------------------------
delimiter ;;
CREATE TRIGGER `tiny`.`trigger_lang_update_row` BEFORE UPDATE ON `lang` FOR EACH ROW BEGIN
    IF OLD.`index` IS NOT NULL THEN
        SET NEW.`index` = OLD.`index`;
    ELSEIF OLD.`index` IS NULL AND NEW.`index` IS NOT NULL THEN
        SET NEW.`index` = (SELECT COALESCE(MAX(`index`), -1) + 1 FROM lang);
    END IF;
END
;;
delimiter ;

SET FOREIGN_KEY_CHECKS = 1;