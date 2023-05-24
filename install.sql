-- ----------------------------
-- Table structure for access
-- ----------------------------
DROP TABLE IF EXISTS "public"."access";
CREATE TABLE "public"."access" (
  "access_id" int8 NOT NULL GENERATED BY DEFAULT AS IDENTITY,
  "role_id" int8 NOT NULL,
  "access" bool NOT NULL,
  "controller_id" int8 NOT NULL
);
COMMENT ON COLUMN "public"."access"."access_id" IS 'Identifier';
COMMENT ON COLUMN "public"."access"."role_id" IS 'Role ID';
COMMENT ON COLUMN "public"."access"."access" IS 'Access flag';
COMMENT ON COLUMN "public"."access"."controller_id" IS 'Controller ID';
COMMENT ON TABLE "public"."access" IS 'Access to controllers';

INSERT INTO "public"."access" VALUES (1, 0, 't', 1);

-- ----------------------------
-- Table structure for controller
-- ----------------------------
DROP TABLE IF EXISTS "public"."controller";
CREATE TABLE "public"."controller" (
  "controller_id" int8 NOT NULL GENERATED BY DEFAULT AS IDENTITY,
  "module" text COLLATE "pg_catalog"."default" NOT NULL,
  "class" text COLLATE "pg_catalog"."default" NOT NULL,
  "action" text COLLATE "pg_catalog"."default" NOT NULL,
  "desc" jsonb NOT NULL,
  "module_id" int8 NOT NULL,
  "class_id" int8 NOT NULL,
  "action_id" int8 NOT NULL
);
COMMENT ON COLUMN "public"."controller"."controller_id" IS 'Identifier';
COMMENT ON COLUMN "public"."controller"."module" IS 'Module';
COMMENT ON COLUMN "public"."controller"."class" IS 'Class';
COMMENT ON COLUMN "public"."controller"."action" IS 'Action (controller)';
COMMENT ON COLUMN "public"."controller"."desc" IS 'Description';
COMMENT ON COLUMN "public"."controller"."module_id" IS 'fnv1a_64 hash from module';
COMMENT ON COLUMN "public"."controller"."class_id" IS 'fnv1a_64 hash from class';
COMMENT ON COLUMN "public"."controller"."action_id" IS 'fnv1a_64 hash from action';
COMMENT ON TABLE "public"."controller" IS 'Controllers list';

INSERT INTO "public"."controller" VALUES (1, 'index', '', '', '[]', -8948777187306027381, -3750763034362895579, -3750763034362895579);
INSERT INTO "public"."controller" VALUES (2, 'index', 'index', 'index', '[]', -8948777187306027381, -8948777187306027381, -8948777187306027381);
INSERT INTO "public"."controller" VALUES (4, 'index', 'article', 'index', '[]', -8948777187306027381, -6149118718490150151, -8948777187306027381);
INSERT INTO "public"."controller" VALUES (3, 'index', 'index', 'not_found', '[]', -8948777187306027381, -8948777187306027381, -1573091631220776463);

-- ----------------------------
-- Table structure for lang
-- ----------------------------
DROP TABLE IF EXISTS "public"."lang";
CREATE TABLE "public"."lang" (
  "lang_id" int8 NOT NULL GENERATED BY DEFAULT AS IDENTITY,
  "name" text COLLATE "pg_catalog"."default" NOT NULL,
  "enable" bool NOT NULL DEFAULT true,
  "lang" text COLLATE "pg_catalog"."default" NOT NULL,
  "sort" int8 NOT NULL,
  "code" text COLLATE "pg_catalog"."default" NOT NULL
);
COMMENT ON COLUMN "public"."lang"."lang_id" IS 'Identifier';
COMMENT ON COLUMN "public"."lang"."name" IS 'Language name';
COMMENT ON COLUMN "public"."lang"."enable" IS 'Enable';
COMMENT ON COLUMN "public"."lang"."lang" IS 'ISO 639-1 : uk - ukrainian, en - english';
COMMENT ON COLUMN "public"."lang"."sort" IS 'Order';
COMMENT ON COLUMN "public"."lang"."code" IS 'ISO 3166 alpha-2: ua - Ukraine, us - USA, gb - United Kingdom';
COMMENT ON TABLE "public"."lang" IS 'Languages';

INSERT INTO "public"."lang" VALUES (0, 'English', 't', 'en', 0, 'us');
INSERT INTO "public"."lang" VALUES (1, 'Українська', 't', 'uk', 1, 'ua');

-- ----------------------------
-- Table structure for redirect
-- ----------------------------
DROP TABLE IF EXISTS "public"."redirect";
CREATE TABLE "public"."redirect" (
  "redirect_id" int8 NOT NULL GENERATED BY DEFAULT AS IDENTITY,
  "url" text COLLATE "pg_catalog"."default" NOT NULL,
  "permanently" bool NOT NULL,
  "redirect" text COLLATE "pg_catalog"."default" NOT NULL
);
COMMENT ON COLUMN "public"."redirect"."redirect_id" IS 'Identifier';
COMMENT ON COLUMN "public"."redirect"."url" IS 'Request URL';
COMMENT ON COLUMN "public"."redirect"."permanently" IS '301 or 302 http code';
COMMENT ON COLUMN "public"."redirect"."redirect" IS 'New URL';
COMMENT ON TABLE "public"."redirect" IS 'Redirect url';

-- ----------------------------
-- Table structure for role
-- ----------------------------
DROP TABLE IF EXISTS "public"."role";
CREATE TABLE "public"."role" (
  "role_id" int8 NOT NULL GENERATED BY DEFAULT AS IDENTITY,
  "name" jsonb NOT NULL,
  "desc" jsonb NOT NULL
);
COMMENT ON COLUMN "public"."role"."role_id" IS 'Identifier';
COMMENT ON COLUMN "public"."role"."name" IS 'Name';
COMMENT ON COLUMN "public"."role"."desc" IS 'Description';
COMMENT ON TABLE "public"."role" IS 'Roles list';

INSERT INTO "public"."role" VALUES (0, '["Guest", "Гість"]', '["Unregister user", "Незареєстрований користувач"]');
INSERT INTO "public"."role" VALUES (1, '["Administrator", "Адміністратор"]', '["Full rules", "Повні права"]');
INSERT INTO "public"."role" VALUES (2, '["Registered user", "Зареєстрований користувач"]', '["Restricted access", "Обмежений доступ"]');

-- ----------------------------
-- Table structure for route
-- ----------------------------
DROP TABLE IF EXISTS "public"."route";
CREATE TABLE "public"."route" (
  "route_id" int8 NOT NULL GENERATED BY DEFAULT AS IDENTITY,
  "url" text COLLATE "pg_catalog"."default" NOT NULL,
  "controller_id" int8 NOT NULL,
  "params" text COLLATE "pg_catalog"."default",
  "lang_id" int8 NOT NULL
);
COMMENT ON COLUMN "public"."route"."route_id" IS 'Identifier';
COMMENT ON COLUMN "public"."route"."url" IS 'Request URL';
COMMENT ON COLUMN "public"."route"."controller_id" IS 'Controller ID';
COMMENT ON COLUMN "public"."route"."params" IS 'Params';
COMMENT ON COLUMN "public"."route"."lang_id" IS 'Language';
COMMENT ON TABLE "public"."route" IS 'Route map';

-- ----------------------------
-- Table structure for session
-- ----------------------------
DROP TABLE IF EXISTS "public"."session";
CREATE TABLE "public"."session" (
  "session_id" int8 NOT NULL GENERATED BY DEFAULT AS IDENTITY,
  "user_id" int8 NOT NULL,
  "lang_id" int8 NOT NULL,
  "session" text COLLATE "pg_catalog"."default" NOT NULL,
  "data" bytea NOT NULL,
  "created" timestamptz(0) NOT NULL,
  "last" timestamptz(6) NOT NULL,
  "ip" text COLLATE "pg_catalog"."default" NOT NULL,
  "user_agent" text COLLATE "pg_catalog"."default" NOT NULL
);
COMMENT ON COLUMN "public"."session"."session_id" IS 'Identifier';
COMMENT ON COLUMN "public"."session"."user_id" IS 'User ID';
COMMENT ON COLUMN "public"."session"."lang_id" IS 'Language';
COMMENT ON COLUMN "public"."session"."session" IS 'Session key';
COMMENT ON COLUMN "public"."session"."data" IS 'Session data';
COMMENT ON COLUMN "public"."session"."created" IS 'Creation time';
COMMENT ON COLUMN "public"."session"."last" IS 'Last change time';
COMMENT ON COLUMN "public"."session"."ip" IS 'Last IP client address';
COMMENT ON COLUMN "public"."session"."user_agent" IS 'Last UserAgent client';
COMMENT ON TABLE "public"."session" IS 'Users session';

-- ----------------------------
-- Table structure for setting
-- ----------------------------
DROP TABLE IF EXISTS "public"."setting";
CREATE TABLE "public"."setting" (
  "setting_id" int8 NOT NULL GENERATED BY DEFAULT AS IDENTITY,
  "key" text COLLATE "pg_catalog"."default" NOT NULL,
  "data" jsonb NOT NULL
);
COMMENT ON COLUMN "public"."setting"."setting_id" IS 'Identifier';
COMMENT ON COLUMN "public"."setting"."key" IS 'Key';
COMMENT ON COLUMN "public"."setting"."data" IS 'Data';
COMMENT ON TABLE "public"."setting" IS 'General settings';

-- ----------------------------
-- Table structure for user
-- ----------------------------
DROP TABLE IF EXISTS "public"."user";
CREATE TABLE "public"."user" (
  "user_id" int8 NOT NULL GENERATED BY DEFAULT AS IDENTITY,
  "enable" bool NOT NULL DEFAULT false,
  "lang_id" int8 NOT NULL,
  "create" timestamptz(6) NOT NULL,
  "protect" bool NOT NULL,
  "role_id" int8 NOT NULL
);
COMMENT ON COLUMN "public"."user"."user_id" IS 'Identifier';
COMMENT ON COLUMN "public"."user"."enable" IS 'User enable';
COMMENT ON COLUMN "public"."user"."lang_id" IS 'Language';
COMMENT ON COLUMN "public"."user"."create" IS 'Creation time';
COMMENT ON COLUMN "public"."user"."protect" IS 'Protect account';
COMMENT ON COLUMN "public"."user"."role_id" IS 'User role';
COMMENT ON TABLE "public"."user" IS 'Users list';

INSERT INTO "public"."user" VALUES (0, 't', 0, '2023-01-01 00:00:00+02', 't', 0);

-- ----------------------------
-- Auto increment value
-- ----------------------------
SELECT setval('"public"."access_access_id_seq"', 1, true);
SELECT setval('"public"."controller_controller_id_seq"', 4, true);
SELECT setval('"public"."lang_lang_id_seq"', 1, true);
SELECT setval('"public"."role_role_id_seq"', 2, true);
SELECT setval('"public"."user_user_id_seq"', 1, false);

-- ----------------------------
-- Indexes structure for table access
-- ----------------------------
CREATE INDEX "access_access_idx" ON "public"."access" USING btree (
  "access" "pg_catalog"."bool_ops" ASC NULLS LAST
);
CREATE INDEX "access_controller_id_idx" ON "public"."access" USING btree (
  "controller_id" "pg_catalog"."int8_ops" ASC NULLS LAST
);
CREATE UNIQUE INDEX "access_role_id_controller_id_idx" ON "public"."access" USING btree (
  "role_id" "pg_catalog"."int8_ops" ASC NULLS LAST,
  "controller_id" "pg_catalog"."int8_ops" ASC NULLS LAST
);
CREATE INDEX "access_role_id_idx" ON "public"."access" USING btree (
  "role_id" "pg_catalog"."int8_ops" ASC NULLS LAST
);
ALTER TABLE "public"."access" ADD CONSTRAINT "access_pkey" PRIMARY KEY ("access_id");

-- ----------------------------
-- Indexes structure for table controller
-- ----------------------------
CREATE INDEX "controller_action_id_idx" ON "public"."controller" USING btree (
  "action_id" "pg_catalog"."int8_ops" ASC NULLS LAST
);
CREATE INDEX "controller_class_id_idx" ON "public"."controller" USING btree (
  "class_id" "pg_catalog"."int8_ops" ASC NULLS LAST
);
CREATE UNIQUE INDEX "controller_module_id_class_id_action_id_idx" ON "public"."controller" USING btree (
  "module_id" "pg_catalog"."int8_ops" ASC NULLS LAST,
  "class_id" "pg_catalog"."int8_ops" ASC NULLS LAST,
  "action_id" "pg_catalog"."int8_ops" ASC NULLS LAST
);
CREATE INDEX "controller_module_id_idx" ON "public"."controller" USING btree (
  "module_id" "pg_catalog"."int8_ops" ASC NULLS LAST
);
ALTER TABLE "public"."controller" ADD CONSTRAINT "controller_expr_ch" CHECK (length(module) = 0 AND length(class) = 0 AND length(action) = 0 OR length(module) > 0 AND length(class) = 0 AND length(action) = 0 OR length(module) > 0 AND length(class) > 0 AND length(action) = 0 OR length(module) > 0 AND length(class) > 0 AND length(action) > 0);
ALTER TABLE "public"."controller" ADD CONSTRAINT "controller_pkey" PRIMARY KEY ("controller_id");

-- ----------------------------
-- Indexes structure for table lang
-- ----------------------------
CREATE INDEX "lang_code_idx" ON "public"."lang" USING btree (
  "code" COLLATE "pg_catalog"."default" "pg_catalog"."text_ops" ASC NULLS LAST
);
CREATE INDEX "lang_enable_idx" ON "public"."lang" USING btree (
  "enable" "pg_catalog"."bool_ops" ASC NULLS LAST
);
CREATE UNIQUE INDEX "lang_lang_code_idx" ON "public"."lang" USING btree (
  "lang" COLLATE "pg_catalog"."default" "pg_catalog"."text_ops" ASC NULLS LAST,
  "code" COLLATE "pg_catalog"."default" "pg_catalog"."text_ops" ASC NULLS LAST
);
CREATE INDEX "lang_lang_idx" ON "public"."lang" USING btree (
  "lang" COLLATE "pg_catalog"."default" "pg_catalog"."text_ops" ASC NULLS LAST
);
CREATE INDEX "lang_name_idx" ON "public"."lang" USING btree (
  "name" COLLATE "pg_catalog"."default" "pg_catalog"."text_ops" ASC NULLS LAST
);
ALTER TABLE "public"."lang" ADD CONSTRAINT "lang_pkey" PRIMARY KEY ("lang_id");

-- ----------------------------
-- Indexes structure for table redirect
-- ----------------------------
CREATE UNIQUE INDEX "redirect_url_idx" ON "public"."redirect" USING btree (
  "url" COLLATE "pg_catalog"."default" "pg_catalog"."text_ops" ASC NULLS LAST
);
ALTER TABLE "public"."redirect" ADD CONSTRAINT "redirect_pkey" PRIMARY KEY ("redirect_id");

-- ----------------------------
-- Indexes structure for table role
-- ----------------------------
CREATE UNIQUE INDEX "role_name_idx" ON "public"."role" USING btree (
  "name" "pg_catalog"."jsonb_ops" ASC NULLS LAST
);
ALTER TABLE "public"."role" ADD CONSTRAINT "role_pkey" PRIMARY KEY ("role_id");

-- ----------------------------
-- Indexes structure for table route
-- ----------------------------
CREATE INDEX "route_controller_id_idx" ON "public"."route" USING btree (
  "controller_id" "pg_catalog"."int8_ops" ASC NULLS LAST
);
CREATE INDEX "route_lang_id_idx" ON "public"."route" USING btree (
  "lang_id" "pg_catalog"."int8_ops" ASC NULLS LAST
);
CREATE INDEX "route_params_idx" ON "public"."route" USING btree (
  "params" COLLATE "pg_catalog"."default" "pg_catalog"."text_ops" ASC NULLS LAST
);
CREATE UNIQUE INDEX "route_url_idx" ON "public"."route" USING btree (
  "url" COLLATE "pg_catalog"."default" "pg_catalog"."text_ops" ASC NULLS LAST
);
ALTER TABLE "public"."route" ADD CONSTRAINT "route_pkey" PRIMARY KEY ("route_id");

-- ----------------------------
-- Indexes structure for table session
-- ----------------------------
CREATE UNIQUE INDEX "session_session_idx" ON "public"."session" USING btree (
  "session" COLLATE "pg_catalog"."default" "pg_catalog"."text_ops" ASC NULLS LAST
);
CREATE INDEX "session_user_id_idx" ON "public"."session" USING btree (
  "user_id" "pg_catalog"."int8_ops" ASC NULLS LAST
);
ALTER TABLE "public"."session" ADD CONSTRAINT "session_pkey" PRIMARY KEY ("session_id");

-- ----------------------------
-- Indexes structure for table setting
-- ----------------------------
CREATE UNIQUE INDEX "setting_key_idx" ON "public"."setting" USING btree (
  "key" COLLATE "pg_catalog"."default" "pg_catalog"."text_ops" ASC NULLS LAST
);
ALTER TABLE "public"."setting" ADD CONSTRAINT "setting_pkey" PRIMARY KEY ("setting_id");

-- ----------------------------
-- Indexes structure for table user
-- ----------------------------
CREATE INDEX "user_enable_idx" ON "public"."user" USING btree (
  "enable" "pg_catalog"."bool_ops" ASC NULLS LAST
);
CREATE INDEX "user_lang_id_idx" ON "public"."user" USING btree (
  "lang_id" "pg_catalog"."int8_ops" ASC NULLS LAST
);
CREATE INDEX "user_protect_idx" ON "public"."user" USING btree (
  "protect" "pg_catalog"."bool_ops" ASC NULLS LAST
);
CREATE INDEX "user_role_id_idx" ON "public"."user" USING btree (
  "role_id" "pg_catalog"."int8_ops" ASC NULLS LAST
);
ALTER TABLE "public"."user" ADD CONSTRAINT "user_pkey" PRIMARY KEY ("user_id");

-- ----------------------------
-- Foreign Keys structure
-- ----------------------------
ALTER TABLE "public"."access" ADD CONSTRAINT "access_controller_id_fkey" FOREIGN KEY ("controller_id") REFERENCES "public"."controller" ("controller_id") ON DELETE NO ACTION ON UPDATE NO ACTION;
ALTER TABLE "public"."access" ADD CONSTRAINT "access_role_id_fkey" FOREIGN KEY ("role_id") REFERENCES "public"."role" ("role_id") ON DELETE NO ACTION ON UPDATE NO ACTION;
ALTER TABLE "public"."route" ADD CONSTRAINT "route_controller_id_fkey" FOREIGN KEY ("controller_id") REFERENCES "public"."controller" ("controller_id") ON DELETE NO ACTION ON UPDATE NO ACTION;
ALTER TABLE "public"."route" ADD CONSTRAINT "route_lang_id_fkey" FOREIGN KEY ("lang_id") REFERENCES "public"."lang" ("lang_id") ON DELETE NO ACTION ON UPDATE NO ACTION;
ALTER TABLE "public"."session" ADD CONSTRAINT "session_user_id_fkey" FOREIGN KEY ("user_id") REFERENCES "public"."user" ("user_id") ON DELETE NO ACTION ON UPDATE NO ACTION;
ALTER TABLE "public"."user" ADD CONSTRAINT "user_lang_id_fkey" FOREIGN KEY ("lang_id") REFERENCES "public"."lang" ("lang_id") ON DELETE NO ACTION ON UPDATE NO ACTION;
ALTER TABLE "public"."user" ADD CONSTRAINT "user_role_id_fkey" FOREIGN KEY ("role_id") REFERENCES "public"."role" ("role_id") ON DELETE NO ACTION ON UPDATE NO ACTION;
