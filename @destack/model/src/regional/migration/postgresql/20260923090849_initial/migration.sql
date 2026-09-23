CREATE TABLE "repository" (
	"id" text PRIMARY KEY,
	"created_at" bigint NOT NULL,
	"updated_at" bigint NOT NULL,
	"revision" bigint DEFAULT 1 NOT NULL,
	"tags" jsonb DEFAULT '{}' NOT NULL,
	"generation" bigint DEFAULT 1 NOT NULL,
	"observed_generation" bigint DEFAULT 0 NOT NULL,
	"conditions" jsonb DEFAULT '{}' NOT NULL,
	"deletion_requested_at" bigint,
	"finalizers" jsonb DEFAULT '[]' NOT NULL,
	"account_id" text NOT NULL,
	"hosting" text NOT NULL,
	"host_id" text,
	"provider" text,
	"provider_repository_id" text,
	"remote" text,
	"default_reference" text,
	"authentication" text,
	"connected_account_id" text,
	"secret_space_id" text,
	"secret_id" text,
	CONSTRAINT "repository_account_id" UNIQUE("account_id","id"),
	CONSTRAINT "repository_revision" CHECK ("revision" >= 1),
	CONSTRAINT "repository_generation" CHECK ("generation" >= 1),
	CONSTRAINT "repository_observed_generation" CHECK ("observed_generation" BETWEEN 0 AND "generation"),
	CONSTRAINT "repository_origin" CHECK (("hosting" = 'platform' AND "provider" IS NOT NULL AND "host_id" IS NULL)
            OR ("hosting" = 'external' AND "provider" IS NOT NULL AND "remote" IS NOT NULL AND "host_id" IS NULL)
            OR ("hosting" = 'host' AND "host_id" IS NOT NULL AND "provider" IS NULL AND "provider_repository_id" IS NULL AND "remote" IS NULL)),
	CONSTRAINT "repository_authentication" CHECK (("hosting" IN ('platform', 'host') AND "authentication" IS NULL AND "connected_account_id" IS NULL AND "secret_space_id" IS NULL AND "secret_id" IS NULL)
            OR ("hosting" = 'external' AND "authentication" IS NOT NULL AND (
                ("authentication" = 'anonymous' AND "connected_account_id" IS NULL AND "secret_space_id" IS NULL AND "secret_id" IS NULL)
                OR ("authentication" = 'connection' AND "connected_account_id" IS NOT NULL AND "secret_space_id" IS NULL AND "secret_id" IS NULL)
                OR ("authentication" = 'secret' AND "connected_account_id" IS NULL AND "secret_space_id" IS NOT NULL AND "secret_id" IS NOT NULL)))),
	CONSTRAINT "repository_default_reference" CHECK ("default_reference" IS NULL OR "default_reference" LIKE 'refs/heads/%'),
	CONSTRAINT "repository_remote" CHECK ("remote" IS NULL OR length("remote") > 0),
	CONSTRAINT "repository_provider" CHECK (("provider" IS NULL OR length("provider") > 0)
            AND ("provider_repository_id" IS NULL OR length("provider_repository_id") > 0))
);
--> statement-breakpoint
CREATE TABLE "repository_ref" (
	"repository_id" text,
	"name" text,
	"object" text NOT NULL,
	"commit" text,
	"revision" bigint DEFAULT 1 NOT NULL,
	"observed_at" bigint NOT NULL,
	"deleted_at" bigint,
	CONSTRAINT "repository_ref_pkey" PRIMARY KEY("repository_id","name"),
	CONSTRAINT "repository_ref_name" CHECK ("name" LIKE 'refs/%'),
	CONSTRAINT "repository_ref_object" CHECK (length("object") IN (40, 64) AND ("object" COLLATE "C") !~ '[^0-9a-f]'),
	CONSTRAINT "repository_ref_commit" CHECK ("commit" IS NULL OR (length("commit") IN (40, 64) AND ("commit" COLLATE "C") !~ '[^0-9a-f]')),
	CONSTRAINT "repository_ref_revision" CHECK ("revision" >= 1)
);
--> statement-breakpoint
CREATE TABLE "release" (
	"package_id" text,
	"version" text,
	"manifest" text NOT NULL,
	"repository_id" text NOT NULL,
	"directory" text NOT NULL,
	"commit" text NOT NULL,
	"created_at" bigint NOT NULL,
	CONSTRAINT "release_pkey" PRIMARY KEY("package_id","version"),
	CONSTRAINT "release_manifest_version" UNIQUE("package_id","version","manifest"),
	CONSTRAINT "release_commit" CHECK (length("commit") IN (40, 64) AND ("commit" COLLATE "C") !~ '[^0-9a-f]'),
	CONSTRAINT "release_directory" CHECK (length("directory") > 0 AND substr("directory", 1, 1) <> '/' AND "directory" <> '..' AND "directory" NOT LIKE '../%' AND "directory" NOT LIKE '%/../%' AND "directory" NOT LIKE '%/..'),
	CONSTRAINT "release_manifest" CHECK (length("manifest") = 64 AND ("manifest" COLLATE "C") !~ '[^0-9a-f]')
);
--> statement-breakpoint
CREATE TABLE "package" (
	"id" text PRIMARY KEY,
	"created_at" bigint NOT NULL,
	"updated_at" bigint NOT NULL,
	"revision" bigint DEFAULT 1 NOT NULL,
	"tags" jsonb DEFAULT '{}' NOT NULL,
	"account_id" text NOT NULL,
	"visibility" text NOT NULL,
	"repository_id" text NOT NULL,
	"directory" text NOT NULL,
	CONSTRAINT "package_repository_id" UNIQUE("repository_id","id"),
	CONSTRAINT "package_visibility" CHECK ("visibility" IN ('public', 'unlisted', 'private')),
	CONSTRAINT "package_directory" CHECK (length("directory") > 0 AND substr("directory", 1, 1) <> '/' AND "directory" NOT LIKE '../%' AND "directory" NOT LIKE '%/../%' AND "directory" <> '..' AND "directory" NOT LIKE '%/..')
);
--> statement-breakpoint
ALTER TABLE "repository_ref" ADD CONSTRAINT "repository_ref_repository_id_repository_id_fkey" FOREIGN KEY ("repository_id") REFERENCES "repository"("id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "release" ADD CONSTRAINT "release_repository_id_package_id_package_repository_id_id_fkey" FOREIGN KEY ("repository_id","package_id") REFERENCES "package"("repository_id","id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "release" ADD CONSTRAINT "release_package_id_package_id_fkey" FOREIGN KEY ("package_id") REFERENCES "package"("id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "release" ADD CONSTRAINT "release_repository_id_repository_id_fkey" FOREIGN KEY ("repository_id") REFERENCES "repository"("id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "package" ADD CONSTRAINT "package_account_id_repository_id_repository_account_id_id_fkey" FOREIGN KEY ("account_id","repository_id") REFERENCES "repository"("account_id","id") ON DELETE RESTRICT;