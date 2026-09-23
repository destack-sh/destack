CREATE TABLE `repository` (
	`id` text PRIMARY KEY,
	`created_at` integer NOT NULL,
	`updated_at` integer NOT NULL,
	`revision` integer DEFAULT 1 NOT NULL,
	`tags` text DEFAULT '{}' NOT NULL,
	`generation` integer DEFAULT 1 NOT NULL,
	`observed_generation` integer DEFAULT 0 NOT NULL,
	`conditions` text DEFAULT '{}' NOT NULL,
	`deletion_requested_at` integer,
	`finalizers` text DEFAULT '[]' NOT NULL,
	`account_id` text NOT NULL,
	`hosting` text NOT NULL,
	`host_id` text,
	`provider` text,
	`provider_repository_id` text,
	`remote` text,
	`default_reference` text,
	`authentication` text,
	`connected_account_id` text,
	`secret_space_id` text,
	`secret_id` text,
	CONSTRAINT `repository_account_id` UNIQUE(`account_id`,`id`),
	CONSTRAINT "repository_revision" CHECK("revision" >= 1),
	CONSTRAINT "repository_generation" CHECK("generation" >= 1),
	CONSTRAINT "repository_observed_generation" CHECK("observed_generation" BETWEEN 0 AND "generation"),
	CONSTRAINT "repository_origin" CHECK(("hosting" = 'platform' AND "provider" IS NOT NULL AND "host_id" IS NULL)
            OR ("hosting" = 'external' AND "provider" IS NOT NULL AND "remote" IS NOT NULL AND "host_id" IS NULL)
            OR ("hosting" = 'host' AND "host_id" IS NOT NULL AND "provider" IS NULL AND "provider_repository_id" IS NULL AND "remote" IS NULL)),
	CONSTRAINT "repository_authentication" CHECK(("hosting" IN ('platform', 'host') AND "authentication" IS NULL AND "connected_account_id" IS NULL AND "secret_space_id" IS NULL AND "secret_id" IS NULL)
            OR ("hosting" = 'external' AND "authentication" IS NOT NULL AND (
                ("authentication" = 'anonymous' AND "connected_account_id" IS NULL AND "secret_space_id" IS NULL AND "secret_id" IS NULL)
                OR ("authentication" = 'connection' AND "connected_account_id" IS NOT NULL AND "secret_space_id" IS NULL AND "secret_id" IS NULL)
                OR ("authentication" = 'secret' AND "connected_account_id" IS NULL AND "secret_space_id" IS NOT NULL AND "secret_id" IS NOT NULL)))),
	CONSTRAINT "repository_default_reference" CHECK("default_reference" IS NULL OR "default_reference" LIKE 'refs/heads/%'),
	CONSTRAINT "repository_remote" CHECK("remote" IS NULL OR length("remote") > 0),
	CONSTRAINT "repository_provider" CHECK(("provider" IS NULL OR length("provider") > 0)
            AND ("provider_repository_id" IS NULL OR length("provider_repository_id") > 0)),
	CONSTRAINT "repository_id_not_null" CHECK("id" IS NOT NULL)
);
--> statement-breakpoint
CREATE TABLE `repository_ref` (
	`repository_id` text NOT NULL,
	`name` text NOT NULL,
	`object` text NOT NULL,
	`commit` text,
	`revision` integer DEFAULT 1 NOT NULL,
	`observed_at` integer NOT NULL,
	`deleted_at` integer,
	CONSTRAINT `repository_ref_pk` PRIMARY KEY(`repository_id`, `name`),
	CONSTRAINT `fk_repository_ref_repository_id_repository_id_fk` FOREIGN KEY (`repository_id`) REFERENCES `repository`(`id`) ON DELETE CASCADE,
	CONSTRAINT "repository_ref_name" CHECK("name" LIKE 'refs/%'),
	CONSTRAINT "repository_ref_object" CHECK(length("object") IN (40, 64) AND "object" NOT GLOB '*[^0-9a-f]*'),
	CONSTRAINT "repository_ref_commit" CHECK("commit" IS NULL OR (length("commit") IN (40, 64) AND "commit" NOT GLOB '*[^0-9a-f]*')),
	CONSTRAINT "repository_ref_revision" CHECK("revision" >= 1)
);
--> statement-breakpoint
CREATE TABLE `release` (
	`package_id` text NOT NULL,
	`version` text NOT NULL,
	`manifest` text NOT NULL,
	`repository_id` text NOT NULL,
	`directory` text NOT NULL,
	`commit` text NOT NULL,
	`created_at` integer NOT NULL,
	CONSTRAINT `release_pk` PRIMARY KEY(`package_id`, `version`),
	CONSTRAINT `fk_release_repository_id_package_id_package_repository_id_id_fk` FOREIGN KEY (`repository_id`,`package_id`) REFERENCES `package`(`repository_id`,`id`) ON DELETE RESTRICT,
	CONSTRAINT `fk_release_package_id_package_id_fk` FOREIGN KEY (`package_id`) REFERENCES `package`(`id`) ON DELETE RESTRICT,
	CONSTRAINT `fk_release_repository_id_repository_id_fk` FOREIGN KEY (`repository_id`) REFERENCES `repository`(`id`) ON DELETE RESTRICT,
	CONSTRAINT `release_manifest_version` UNIQUE(`package_id`,`version`,`manifest`),
	CONSTRAINT "release_commit" CHECK(length("commit") IN (40, 64) AND "commit" NOT GLOB '*[^0-9a-f]*'),
	CONSTRAINT "release_directory" CHECK(length("directory") > 0 AND substr("directory", 1, 1) <> '/' AND "directory" <> '..' AND "directory" NOT LIKE '../%' AND "directory" NOT LIKE '%/../%' AND "directory" NOT LIKE '%/..'),
	CONSTRAINT "release_manifest" CHECK(length("manifest") = 64 AND "manifest" NOT GLOB '*[^0-9a-f]*')
);
--> statement-breakpoint
CREATE TABLE `package` (
	`id` text PRIMARY KEY,
	`created_at` integer NOT NULL,
	`updated_at` integer NOT NULL,
	`revision` integer DEFAULT 1 NOT NULL,
	`tags` text DEFAULT '{}' NOT NULL,
	`account_id` text NOT NULL,
	`visibility` text NOT NULL,
	`repository_id` text NOT NULL,
	`directory` text NOT NULL,
	CONSTRAINT `fk_package_account_id_repository_id_repository_account_id_id_fk` FOREIGN KEY (`account_id`,`repository_id`) REFERENCES `repository`(`account_id`,`id`) ON DELETE RESTRICT,
	CONSTRAINT `package_repository_id` UNIQUE(`repository_id`,`id`),
	CONSTRAINT "package_visibility" CHECK("visibility" IN ('public', 'unlisted', 'private')),
	CONSTRAINT "package_directory" CHECK(length("directory") > 0 AND substr("directory", 1, 1) <> '/' AND "directory" NOT LIKE '../%' AND "directory" NOT LIKE '%/../%' AND "directory" <> '..' AND "directory" NOT LIKE '%/..'),
	CONSTRAINT "package_id_not_null" CHECK("id" IS NOT NULL)
);
