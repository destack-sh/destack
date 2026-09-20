CREATE TABLE "space_migration" (
	"id" text PRIMARY KEY,
	"account_id" text NOT NULL,
	"space_id" text NOT NULL,
	"generation" bigint NOT NULL,
	"source_host_id" text NOT NULL,
	"target_host_id" text NOT NULL,
	"source_epoch" bigint NOT NULL,
	"target_epoch" bigint,
	"requested_by" text,
	"created_at" bigint NOT NULL,
	"started_at" bigint,
	"cancellation_requested_at" bigint,
	"source_revoked_at" bigint,
	"activated_at" bigint,
	"outcome" text,
	"completed_at" bigint,
	"conditions" jsonb DEFAULT '{}' NOT NULL,
	CONSTRAINT "space_migration_space_id" UNIQUE("space_id","id"),
	CONSTRAINT "space_migration_generation" CHECK ("generation" >= 1),
	CONSTRAINT "space_migration_epoch" CHECK ("source_epoch" >= 1 AND ("target_epoch" IS NULL OR "target_epoch" > "source_epoch")),
	CONSTRAINT "space_migration_outcome" CHECK ("outcome" IS NULL OR "outcome" IN ('succeeded', 'failed', 'cancelled')),
	CONSTRAINT "space_migration_completion" CHECK (("outcome" IS NULL) = ("completed_at" IS NULL)),
	CONSTRAINT "space_migration_activation" CHECK (("activated_at" IS NULL) = ("target_epoch" IS NULL) AND ("activated_at" IS NULL OR ("source_revoked_at" IS NOT NULL AND "started_at" IS NOT NULL AND "activated_at" >= "source_revoked_at"))),
	CONSTRAINT "space_migration_success" CHECK ("outcome" IS NULL OR "outcome" <> 'succeeded' OR "activated_at" IS NOT NULL),
	CONSTRAINT "space_migration_cancel" CHECK ("outcome" IS NULL OR "outcome" <> 'cancelled' OR ("activated_at" IS NULL AND "cancellation_requested_at" IS NOT NULL)),
	CONSTRAINT "space_migration_time" CHECK (("started_at" IS NULL OR "started_at" >= "created_at") AND ("completed_at" IS NULL OR "completed_at" >= "created_at")),
	CONSTRAINT "space_migration_revoke_time" CHECK ("source_revoked_at" IS NULL OR ("started_at" IS NOT NULL AND "source_revoked_at" >= "started_at")),
	CONSTRAINT "space_migration_complete_time" CHECK ("completed_at" IS NULL OR (("started_at" IS NULL OR "completed_at" >= "started_at") AND ("activated_at" IS NULL OR "completed_at" >= "activated_at")))
);
--> statement-breakpoint
CREATE TABLE "space" (
	"id" text PRIMARY KEY,
	"created_at" bigint NOT NULL,
	"updated_at" bigint NOT NULL,
	"revision" bigint DEFAULT 1 NOT NULL,
	"tags" jsonb DEFAULT '{}' NOT NULL,
	"account_id" text NOT NULL,
	"name" text NOT NULL,
	"environment" text NOT NULL,
	"state" text DEFAULT 'enabled' NOT NULL,
	"placement" text DEFAULT 'automatic' NOT NULL,
	"requested_primary_host_id" text,
	"primary_host_id" text,
	"primary_host_epoch" bigint DEFAULT 0 NOT NULL,
	"generation" bigint DEFAULT 1 NOT NULL,
	"observed_generation" bigint DEFAULT 0 NOT NULL,
	"conditions" jsonb DEFAULT '{}' NOT NULL,
	"deletion_requested_at" bigint,
	"finalizers" jsonb DEFAULT '[]' NOT NULL,
	CONSTRAINT "space_account_id" UNIQUE("account_id","id"),
	CONSTRAINT "space_revision" CHECK ("revision" >= 1),
	CONSTRAINT "space_generation" CHECK ("generation" >= 1),
	CONSTRAINT "space_observed_generation" CHECK ("observed_generation" BETWEEN 0 AND "generation"),
	CONSTRAINT "space_state" CHECK ("state" IN ('enabled', 'suspended')),
	CONSTRAINT "space_placement" CHECK (("placement" = 'automatic' AND "requested_primary_host_id" IS NULL) OR ("placement" = 'host' AND "requested_primary_host_id" IS NOT NULL)),
	CONSTRAINT "space_host_epoch" CHECK ("primary_host_epoch" >= 0 AND ("primary_host_id" IS NULL OR "primary_host_epoch" > 0)),
	CONSTRAINT "space_environment" CHECK ("environment" IN ('development', 'preview', 'production'))
);
--> statement-breakpoint
CREATE TABLE "resource_binding" (
	"space_id" text NOT NULL,
	"installation_id" text,
	"package_id" text,
	"name" text,
	"resource_id" text NOT NULL,
	CONSTRAINT "resource_binding_pkey" PRIMARY KEY("installation_id","package_id","name")
);
--> statement-breakpoint
CREATE TABLE "resource_migration" (
	"id" text PRIMARY KEY,
	"space_migration_id" text,
	"account_id" text NOT NULL,
	"space_id" text NOT NULL,
	"resource_id" text NOT NULL,
	"created_at" bigint NOT NULL,
	"started_at" bigint,
	"cancellation_requested_at" bigint,
	"outcome" text,
	"completed_at" bigint,
	"source_provider" text NOT NULL,
	"source_reference" text NOT NULL,
	"source_host_id" text,
	"target_provider" text NOT NULL,
	"target_host_id" text,
	"target_reference" text,
	"protocol" text NOT NULL,
	"snapshot_id" text,
	"source_position" text,
	"target_position" text,
	"verified_at" bigint,
	"activated_at" bigint,
	"source_removed_at" bigint,
	"conditions" jsonb DEFAULT '{}' NOT NULL,
	CONSTRAINT "resource_migration_outcome" CHECK ("outcome" IS NULL OR "outcome" IN ('succeeded', 'failed', 'cancelled')),
	CONSTRAINT "resource_migration_completion" CHECK (("outcome" IS NULL) = ("completed_at" IS NULL)),
	CONSTRAINT "resource_migration_success" CHECK ("outcome" IS NULL OR "outcome" <> 'succeeded' OR "activated_at" IS NOT NULL),
	CONSTRAINT "resource_migration_cancel" CHECK ("outcome" IS NULL OR "outcome" <> 'cancelled' OR ("activated_at" IS NULL AND "cancellation_requested_at" IS NOT NULL)),
	CONSTRAINT "resource_migration_time" CHECK (("started_at" IS NULL OR "started_at" >= "created_at") AND ("completed_at" IS NULL OR ("completed_at" >= "created_at" AND ("started_at" IS NULL OR "completed_at" >= "started_at") AND ("activated_at" IS NULL OR "completed_at" >= "activated_at")))),
	CONSTRAINT "resource_migration_verification" CHECK ("verified_at" IS NULL OR ("target_reference" IS NOT NULL AND "started_at" IS NOT NULL AND "verified_at" >= "started_at")),
	CONSTRAINT "resource_migration_activation" CHECK ("activated_at" IS NULL OR ("verified_at" IS NOT NULL AND "activated_at" >= "verified_at")),
	CONSTRAINT "resource_migration_cleanup" CHECK ("source_removed_at" IS NULL OR ("activated_at" IS NOT NULL AND "source_removed_at" >= "activated_at"))
);
--> statement-breakpoint
CREATE TABLE "vault" (
	"resource_id" text PRIMARY KEY,
	"space_id" text NOT NULL,
	"kind" text DEFAULT 'vault' NOT NULL,
	CONSTRAINT "vault_space_id" UNIQUE("space_id","resource_id"),
	CONSTRAINT "vault_kind" CHECK ("kind" = 'vault')
);
--> statement-breakpoint
CREATE TABLE "secret" (
	"id" text PRIMARY KEY,
	"created_at" bigint NOT NULL,
	"updated_at" bigint NOT NULL,
	"revision" bigint DEFAULT 1 NOT NULL,
	"tags" jsonb DEFAULT '{}' NOT NULL,
	"source" jsonb,
	"source_detached_at" bigint,
	"space_id" text NOT NULL,
	"vault_id" text NOT NULL,
	"name" text NOT NULL,
	"current_version" bigint,
	"revoked_at" bigint,
	CONSTRAINT "secret_vault_name" UNIQUE("vault_id","name"),
	CONSTRAINT "secret_space_id" UNIQUE("space_id","id"),
	CONSTRAINT "secret_source_detached" CHECK ("source_detached_at" IS NULL OR ("source" IS NOT NULL AND "source_detached_at" >= 0)),
	CONSTRAINT "secret_name" CHECK (length("name") > 0)
);
--> statement-breakpoint
CREATE TABLE "secret_version" (
	"secret_id" text,
	"version" bigint,
	"created_at" bigint NOT NULL,
	"reference" text NOT NULL,
	"expires_at" bigint,
	"revoked_at" bigint,
	"destroyed_at" bigint,
	CONSTRAINT "secret_version_pkey" PRIMARY KEY("secret_id","version"),
	CONSTRAINT "secret_version_positive" CHECK ("version" > 0),
	CONSTRAINT "secret_version_reference" CHECK (length("reference") > 0),
	CONSTRAINT "secret_version_expiry" CHECK ("expires_at" IS NULL OR "expires_at" > "created_at")
);
--> statement-breakpoint
CREATE TABLE "secret_binding" (
	"space_id" text NOT NULL,
	"installation_id" text,
	"package_id" text,
	"name" text,
	"secret_id" text NOT NULL,
	"version" bigint,
	CONSTRAINT "secret_binding_pkey" PRIMARY KEY("installation_id","package_id","name"),
	CONSTRAINT "secret_binding_name" CHECK (length("name") > 0)
);
--> statement-breakpoint
CREATE TABLE "deployment_secret_binding" (
	"space_id" text NOT NULL,
	"deployment_id" text,
	"package_id" text,
	"name" text,
	"secret_id" text NOT NULL,
	"version" bigint,
	CONSTRAINT "deployment_secret_binding_pkey" PRIMARY KEY("deployment_id","package_id","name"),
	CONSTRAINT "deployment_secret_binding_name" CHECK (length("name") > 0)
);
--> statement-breakpoint
CREATE TABLE "resource" (
	"id" text PRIMARY KEY,
	"created_at" bigint NOT NULL,
	"updated_at" bigint NOT NULL,
	"revision" bigint DEFAULT 1 NOT NULL,
	"tags" jsonb DEFAULT '{}' NOT NULL,
	"source" jsonb,
	"source_detached_at" bigint,
	"account_id" text NOT NULL,
	"space_id" text NOT NULL,
	"name" text NOT NULL,
	"definition_package_id" text NOT NULL,
	"definition_version" text NOT NULL,
	"definition_name" text NOT NULL,
	"spec" jsonb NOT NULL,
	"status" jsonb DEFAULT '{}' NOT NULL,
	"kind" text NOT NULL,
	"requested_provider" text,
	"requested_region_id" text,
	"requested_host_id" text,
	"provider" text,
	"reference" text,
	"region_id" text,
	"host_id" text,
	"retention" text DEFAULT 'retain' NOT NULL,
	"generation" bigint DEFAULT 1 NOT NULL,
	"observed_generation" bigint DEFAULT 0 NOT NULL,
	"conditions" jsonb DEFAULT '{}' NOT NULL,
	"deletion_requested_at" bigint,
	"finalizers" jsonb DEFAULT '[]' NOT NULL,
	CONSTRAINT "resource_space_name" UNIQUE("space_id","name"),
	CONSTRAINT "resource_space_id" UNIQUE("space_id","id"),
	CONSTRAINT "resource_space_kind" UNIQUE("space_id","id","kind"),
	CONSTRAINT "resource_source_detached" CHECK ("source_detached_at" IS NULL OR ("source" IS NOT NULL AND "source_detached_at" >= 0)),
	CONSTRAINT "resource_requested_region" CHECK ("requested_region_id" IS NULL OR "requested_provider" IS NOT NULL),
	CONSTRAINT "resource_revision" CHECK ("revision" >= 1),
	CONSTRAINT "resource_generation" CHECK ("generation" >= 1),
	CONSTRAINT "resource_observed_generation" CHECK ("observed_generation" BETWEEN 0 AND "generation"),
	CONSTRAINT "resource_provider" CHECK (("provider" IS NULL) = ("reference" IS NULL)),
	CONSTRAINT "resource_region" CHECK ("region_id" IS NULL OR "provider" IS NOT NULL),
	CONSTRAINT "resource_host" CHECK ("host_id" IS NULL OR "provider" IS NOT NULL),
	CONSTRAINT "resource_retention" CHECK ("retention" IN ('retain', 'delete'))
);
--> statement-breakpoint
CREATE TABLE "deployment_binding" (
	"space_id" text NOT NULL,
	"deployment_id" text,
	"package_id" text,
	"name" text,
	"resource_id" text NOT NULL,
	"generation" bigint NOT NULL,
	CONSTRAINT "deployment_binding_pkey" PRIMARY KEY("deployment_id","package_id","name"),
	CONSTRAINT "deployment_binding_generation" CHECK ("generation" > 0)
);
--> statement-breakpoint
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
CREATE TABLE "installation" (
	"id" text PRIMARY KEY,
	"created_at" bigint NOT NULL,
	"updated_at" bigint NOT NULL,
	"revision" bigint DEFAULT 1 NOT NULL,
	"tags" jsonb DEFAULT '{}' NOT NULL,
	"source" jsonb,
	"source_detached_at" bigint,
	"space_id" text NOT NULL,
	"package_id" text NOT NULL,
	"version" text NOT NULL,
	"applied_version" text,
	"applied_generation" bigint DEFAULT 0 NOT NULL,
	"state" text DEFAULT 'enabled' NOT NULL,
	"alias" text NOT NULL,
	"generation" bigint DEFAULT 1 NOT NULL,
	"observed_generation" bigint DEFAULT 0 NOT NULL,
	"conditions" jsonb DEFAULT '{}' NOT NULL,
	"deletion_requested_at" bigint,
	"finalizers" jsonb DEFAULT '[]' NOT NULL,
	CONSTRAINT "installation_space_alias" UNIQUE("space_id","alias"),
	CONSTRAINT "installation_space_id" UNIQUE("space_id","id"),
	CONSTRAINT "installation_space_package" UNIQUE("space_id","id","package_id"),
	CONSTRAINT "installation_source_detached" CHECK ("source_detached_at" IS NULL OR ("source" IS NOT NULL AND "source_detached_at" >= 0)),
	CONSTRAINT "installation_revision" CHECK ("revision" >= 1),
	CONSTRAINT "installation_generation" CHECK ("generation" >= 1),
	CONSTRAINT "installation_observed_generation" CHECK ("observed_generation" BETWEEN 0 AND "generation"),
	CONSTRAINT "installation_state" CHECK ("state" IN ('enabled', 'suspended')),
	CONSTRAINT "installation_applied_generation" CHECK ("applied_generation" BETWEEN 0 AND "generation" AND (("applied_version" IS NULL AND "applied_generation" = 0) OR ("applied_version" IS NOT NULL AND "applied_generation" > 0)))
);
--> statement-breakpoint
CREATE TABLE "repository_reference" (
	"repository_id" text,
	"name" text,
	"object" text NOT NULL,
	"commit" text,
	"revision" bigint DEFAULT 1 NOT NULL,
	"observed_at" bigint NOT NULL,
	"deleted_at" bigint,
	CONSTRAINT "repository_reference_pkey" PRIMARY KEY("repository_id","name"),
	CONSTRAINT "repository_reference_name" CHECK ("name" LIKE 'refs/%'),
	CONSTRAINT "repository_reference_object" CHECK (length("object") IN (40, 64) AND ("object" COLLATE "C") !~ '[^0-9a-f]'),
	CONSTRAINT "repository_reference_commit" CHECK ("commit" IS NULL OR (length("commit") IN (40, 64) AND ("commit" COLLATE "C") !~ '[^0-9a-f]')),
	CONSTRAINT "repository_reference_revision" CHECK ("revision" >= 1)
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
CREATE TABLE "package_policy" (
	"id" text PRIMARY KEY,
	"created_at" bigint NOT NULL,
	"updated_at" bigint NOT NULL,
	"revision" bigint DEFAULT 1 NOT NULL,
	"tags" jsonb DEFAULT '{}' NOT NULL,
	"source" jsonb,
	"source_detached_at" bigint,
	"account_id" text NOT NULL,
	"scope" text NOT NULL,
	"space_id" text,
	"generation" bigint DEFAULT 1 NOT NULL,
	"current_revision_id" text,
	CONSTRAINT "package_policy_source_detached" CHECK ("source_detached_at" IS NULL OR ("source" IS NOT NULL AND "source_detached_at" >= 0)),
	CONSTRAINT "package_policy_scope" CHECK (
        ("scope" = 'account' AND "space_id" IS NULL) OR
        ("scope" = 'space' AND "space_id" IS NOT NULL)
    ),
	CONSTRAINT "package_policy_generation" CHECK ("generation" > 0)
);
--> statement-breakpoint
CREATE TABLE "package_policy_revision" (
	"id" text PRIMARY KEY,
	"created_at" bigint NOT NULL,
	"updated_at" bigint NOT NULL,
	"revision" bigint DEFAULT 1 NOT NULL,
	"tags" jsonb DEFAULT '{}' NOT NULL,
	"policy_id" text NOT NULL,
	"generation" bigint NOT NULL,
	"definition" jsonb NOT NULL,
	"source" jsonb,
	"digest" text NOT NULL,
	CONSTRAINT "package_policy_revision_policy_id" UNIQUE("policy_id","id"),
	CONSTRAINT "package_policy_revision_generation" UNIQUE("policy_id","generation"),
	CONSTRAINT "package_policy_revision_generation_positive" CHECK ("generation" > 0),
	CONSTRAINT "package_policy_revision_digest" CHECK (
        length("digest") = 64 AND ("digest" COLLATE "C") !~ '[^a-f0-9]'
    )
);
--> statement-breakpoint
CREATE TABLE "role" (
	"id" text PRIMARY KEY,
	"created_at" bigint NOT NULL,
	"updated_at" bigint NOT NULL,
	"revision" bigint DEFAULT 1 NOT NULL,
	"tags" jsonb DEFAULT '{}' NOT NULL,
	"source" jsonb,
	"source_detached_at" bigint,
	"account_id" text NOT NULL,
	"space_id" text NOT NULL,
	"name" text NOT NULL,
	"description" text NOT NULL,
	CONSTRAINT "role_space_name" UNIQUE("space_id","name"),
	CONSTRAINT "role_space_id" UNIQUE("space_id","id"),
	CONSTRAINT "role_account_id" UNIQUE("account_id","id"),
	CONSTRAINT "role_source_detached" CHECK ("source_detached_at" IS NULL OR ("source" IS NOT NULL AND "source_detached_at" >= 0))
);
--> statement-breakpoint
CREATE TABLE "role_binding" (
	"id" text PRIMARY KEY,
	"created_at" bigint NOT NULL,
	"updated_at" bigint NOT NULL,
	"revision" bigint DEFAULT 1 NOT NULL,
	"tags" jsonb DEFAULT '{}' NOT NULL,
	"source" jsonb,
	"source_detached_at" bigint,
	"account_id" text NOT NULL,
	"role_id" text NOT NULL,
	"space_id" text NOT NULL,
	"account_membership_id" text,
	"group_id" text,
	"service_account_id" text,
	"account_service_account_id" text,
	"expires_at" bigint,
	"revoked_at" bigint,
	CONSTRAINT "role_binding_source_detached" CHECK ("source_detached_at" IS NULL OR ("source" IS NOT NULL AND "source_detached_at" >= 0)),
	CONSTRAINT "role_binding_subject" CHECK (CAST("account_membership_id" IS NOT NULL AS integer) + CAST("group_id" IS NOT NULL AS integer) + CAST("service_account_id" IS NOT NULL AS integer) + CAST("account_service_account_id" IS NOT NULL AS integer) = 1),
	CONSTRAINT "role_binding_expiry" CHECK ("expires_at" IS NULL OR "expires_at" > "created_at")
);
--> statement-breakpoint
CREATE TABLE "role_permission" (
	"id" text PRIMARY KEY,
	"role_id" text NOT NULL,
	"resource" text NOT NULL,
	"action" text NOT NULL,
	"resource_id" text,
	CONSTRAINT "role_permission_resource" CHECK (length("resource") > 0 AND strpos("resource", '.') > 0 AND strpos("resource", '*') = 0),
	CONSTRAINT "role_permission_action" CHECK (length("action") > 0 AND strpos("action", '*') = 0),
	CONSTRAINT "role_permission_identifier" CHECK ("resource_id" IS NULL OR length("resource_id") > 0)
);
--> statement-breakpoint
CREATE TABLE "service_account" (
	"id" text PRIMARY KEY,
	"created_at" bigint NOT NULL,
	"updated_at" bigint NOT NULL,
	"revision" bigint DEFAULT 1 NOT NULL,
	"tags" jsonb DEFAULT '{}' NOT NULL,
	"account_id" text NOT NULL,
	"name" text NOT NULL,
	"space_id" text NOT NULL,
	"installation_id" text NOT NULL,
	"workload" text NOT NULL,
	"revoked_at" bigint,
	CONSTRAINT "service_account_space_id" UNIQUE("space_id","id"),
	CONSTRAINT "service_account_name" UNIQUE("space_id","name"),
	CONSTRAINT "service_account_account_id" UNIQUE("account_id","id"),
	CONSTRAINT "service_account_workload" UNIQUE("installation_id","workload"),
	CONSTRAINT "service_account_workload_id" UNIQUE("installation_id","workload","id"),
	CONSTRAINT "service_account_workload_name" CHECK (length("workload") > 0)
);
--> statement-breakpoint
CREATE TABLE "service_token" (
	"id" text PRIMARY KEY,
	"created_at" bigint NOT NULL,
	"updated_at" bigint NOT NULL,
	"revision" bigint DEFAULT 1 NOT NULL,
	"tags" jsonb DEFAULT '{}' NOT NULL,
	"service_account_id" text NOT NULL,
	"name" text NOT NULL,
	"token_hash" text NOT NULL UNIQUE,
	"expires_at" bigint NOT NULL,
	"revoked_at" bigint,
	CONSTRAINT "service_token_expiry" CHECK ("expires_at" > "created_at")
);
--> statement-breakpoint
CREATE TABLE "space_host" (
	"account_id" text NOT NULL,
	"space_id" text,
	"host_id" text,
	"state" text DEFAULT 'enabled' NOT NULL,
	"epoch" bigint DEFAULT 1 NOT NULL,
	CONSTRAINT "space_host_pkey" PRIMARY KEY("space_id","host_id"),
	CONSTRAINT "space_host_state" CHECK ("state" IN ('enabled', 'draining', 'disabled')),
	CONSTRAINT "space_host_epoch" CHECK ("epoch" >= 1)
);
--> statement-breakpoint
CREATE TABLE "schedule" (
	"id" text PRIMARY KEY,
	"created_at" bigint NOT NULL,
	"updated_at" bigint NOT NULL,
	"revision" bigint DEFAULT 1 NOT NULL,
	"tags" jsonb DEFAULT '{}' NOT NULL,
	"source" jsonb,
	"source_detached_at" bigint,
	"space_id" text NOT NULL,
	"name" text NOT NULL,
	"installation_id" text NOT NULL,
	"workload" text NOT NULL,
	"operation" text NOT NULL,
	"arguments" jsonb NOT NULL,
	"service_account_id" text NOT NULL,
	"timing" text NOT NULL,
	"cron" text,
	"timezone" text,
	"interval" bigint,
	"starts_at" bigint,
	"ends_at" bigint,
	"concurrency" text NOT NULL,
	"deadline" bigint NOT NULL,
	"paused_at" bigint,
	"generation" bigint DEFAULT 1 NOT NULL,
	CONSTRAINT "schedule_space_name" UNIQUE("space_id","name"),
	CONSTRAINT "schedule_source_detached" CHECK ("source_detached_at" IS NULL OR ("source" IS NOT NULL AND "source_detached_at" >= 0)),
	CONSTRAINT "schedule_timing" CHECK (("timing" = 'cron' AND "cron" IS NOT NULL AND "timezone" IS NOT NULL AND "interval" IS NULL) OR ("timing" = 'interval' AND "interval" > 0 AND "interval" IS NOT NULL AND "starts_at" IS NOT NULL AND "cron" IS NULL AND "timezone" IS NULL) OR ("timing" = 'once' AND "starts_at" IS NOT NULL AND "ends_at" IS NULL AND "cron" IS NULL AND "timezone" IS NULL AND "interval" IS NULL)),
	CONSTRAINT "schedule_concurrency" CHECK ("concurrency" IN ('allow', 'forbid', 'replace')),
	CONSTRAINT "schedule_deadline" CHECK ("deadline" >= 0),
	CONSTRAINT "schedule_generation" CHECK ("generation" > 0),
	CONSTRAINT "schedule_end" CHECK ("ends_at" IS NULL OR "starts_at" IS NULL OR "ends_at" > "starts_at"),
	CONSTRAINT "schedule_names" CHECK (length("name") > 0 AND length("workload") > 0 AND length("operation") > 0)
);
--> statement-breakpoint
CREATE TABLE "network_policy" (
	"id" text PRIMARY KEY,
	"created_at" bigint NOT NULL,
	"updated_at" bigint NOT NULL,
	"revision" bigint DEFAULT 1 NOT NULL,
	"tags" jsonb DEFAULT '{}' NOT NULL,
	"source" jsonb,
	"source_detached_at" bigint,
	"account_id" text NOT NULL,
	"scope" text NOT NULL,
	"space_id" text,
	"installation_id" text,
	"workload" text,
	"generation" bigint DEFAULT 1 NOT NULL,
	"current_revision_id" text,
	CONSTRAINT "network_policy_source_detached" CHECK ("source_detached_at" IS NULL OR ("source" IS NOT NULL AND "source_detached_at" >= 0)),
	CONSTRAINT "network_policy_scope" CHECK (
        ("scope" = 'account' AND "space_id" IS NULL AND "installation_id" IS NULL AND "workload" IS NULL) OR
        ("scope" = 'space' AND "space_id" IS NOT NULL AND "installation_id" IS NULL AND "workload" IS NULL) OR
        ("scope" = 'installation' AND "space_id" IS NOT NULL AND "installation_id" IS NOT NULL AND "workload" IS NULL) OR
        ("scope" = 'workload' AND "space_id" IS NOT NULL AND "installation_id" IS NOT NULL AND "workload" IS NOT NULL AND length("workload") > 0)
    ),
	CONSTRAINT "network_policy_generation" CHECK ("generation" > 0)
);
--> statement-breakpoint
CREATE TABLE "network_policy_revision" (
	"id" text PRIMARY KEY,
	"created_at" bigint NOT NULL,
	"updated_at" bigint NOT NULL,
	"revision" bigint DEFAULT 1 NOT NULL,
	"tags" jsonb DEFAULT '{}' NOT NULL,
	"policy_id" text NOT NULL,
	"generation" bigint NOT NULL,
	"definition" jsonb NOT NULL,
	"source" jsonb,
	"digest" text NOT NULL,
	CONSTRAINT "network_policy_revision_policy_id" UNIQUE("policy_id","id"),
	CONSTRAINT "network_policy_revision_generation" UNIQUE("policy_id","generation"),
	CONSTRAINT "network_policy_revision_generation_positive" CHECK ("generation" > 0),
	CONSTRAINT "network_policy_revision_digest" CHECK (
        length("digest") = 64 AND ("digest" COLLATE "C") !~ '[^a-f0-9]'
    )
);
--> statement-breakpoint
CREATE TABLE "deployment" (
	"id" text PRIMARY KEY,
	"created_at" bigint NOT NULL,
	"updated_at" bigint NOT NULL,
	"revision" bigint DEFAULT 1 NOT NULL,
	"tags" jsonb DEFAULT '{}' NOT NULL,
	"space_id" text NOT NULL,
	"installation_id" text NOT NULL,
	"generation" bigint NOT NULL,
	"package_id" text NOT NULL,
	"version" text NOT NULL,
	"manifest" text NOT NULL,
	"output" text NOT NULL,
	"workload" text NOT NULL,
	"runtime" text NOT NULL,
	"service_account_id" text NOT NULL,
	"description" jsonb NOT NULL,
	"policies" jsonb NOT NULL,
	"host_id" text,
	"state" text DEFAULT 'prepared' NOT NULL,
	"conditions" jsonb DEFAULT '{}' NOT NULL,
	"activated_at" bigint,
	"retired_at" bigint,
	CONSTRAINT "deployment_space_id" UNIQUE("space_id","id"),
	CONSTRAINT "deployment_service_account" UNIQUE("id","service_account_id"),
	CONSTRAINT "deployment_generation" UNIQUE("installation_id","generation","output","workload"),
	CONSTRAINT "deployment_runtime" CHECK ("runtime" IN ('deno', 'workerd')),
	CONSTRAINT "deployment_workload" CHECK (length("workload") > 0),
	CONSTRAINT "deployment_generation_positive" CHECK ("generation" > 0),
	CONSTRAINT "deployment_state" CHECK ("state" IN ('prepared', 'active', 'draining', 'retired')),
	CONSTRAINT "deployment_times" CHECK ("retired_at" IS NULL OR "activated_at" IS NULL OR "retired_at" >= "activated_at")
);
--> statement-breakpoint
CREATE TABLE "instance" (
	"id" text PRIMARY KEY,
	"created_at" bigint NOT NULL,
	"updated_at" bigint NOT NULL,
	"revision" bigint DEFAULT 1 NOT NULL,
	"tags" jsonb DEFAULT '{}' NOT NULL,
	"space_id" text NOT NULL,
	"deployment_id" text NOT NULL,
	"host_id" text NOT NULL,
	"host_epoch" bigint NOT NULL,
	"reference" text,
	"state" text NOT NULL,
	"conditions" jsonb DEFAULT '{}' NOT NULL,
	"observed_at" bigint NOT NULL,
	"lease_expires_at" bigint,
	"started_at" bigint,
	"stopped_at" bigint,
	CONSTRAINT "instance_epoch" CHECK ("host_epoch" > 0),
	CONSTRAINT "instance_state" CHECK ("state" IN ('starting', 'running', 'draining', 'stopped', 'failed')),
	CONSTRAINT "instance_times" CHECK ("stopped_at" IS NULL OR "started_at" IS NULL OR "stopped_at" >= "started_at")
);
--> statement-breakpoint
CREATE TABLE "space_configuration" (
	"space_id" text PRIMARY KEY,
	"repository_id" text NOT NULL,
	"reference" text NOT NULL,
	"directory" text NOT NULL,
	"entrypoint" text NOT NULL,
	"export" text NOT NULL,
	"parameter_schema" text,
	"parameters" jsonb NOT NULL,
	"generation" bigint DEFAULT 1 NOT NULL,
	"applied_revision_id" text,
	CONSTRAINT "space_configuration_generation" CHECK ("generation" > 0),
	CONSTRAINT "space_configuration_reference" CHECK ("reference" LIKE 'refs/heads/%' OR "reference" LIKE 'refs/tags/%'),
	CONSTRAINT "space_configuration_directory" CHECK (length("directory") > 0 AND substr("directory", 1, 1) <> '/' AND "directory" <> '..' AND "directory" NOT LIKE '../%' AND "directory" NOT LIKE '%/../%' AND "directory" NOT LIKE '%/..'),
	CONSTRAINT "space_configuration_export" CHECK (length("export") > 0 AND ("entrypoint" = '.' OR "entrypoint" LIKE './%'))
);
--> statement-breakpoint
CREATE TABLE "stack_revision" (
	"id" text PRIMARY KEY,
	"space_id" text NOT NULL,
	"source_generation" bigint NOT NULL,
	"source" jsonb NOT NULL,
	"parameters" jsonb NOT NULL,
	"definition" jsonb NOT NULL,
	"releases" jsonb NOT NULL,
	"digest" text NOT NULL,
	"created_at" bigint NOT NULL,
	CONSTRAINT "stack_revision_space_id" UNIQUE("space_id","id"),
	CONSTRAINT "stack_revision_source_generation" UNIQUE("space_id","id","source_generation"),
	CONSTRAINT "stack_revision_generation" CHECK ("source_generation" > 0),
	CONSTRAINT "stack_revision_digest" CHECK (length("digest") = 64 AND ("digest" COLLATE "C") !~ '[^a-f0-9]')
);
--> statement-breakpoint
CREATE TABLE "restoration" (
	"id" text PRIMARY KEY,
	"created_at" bigint NOT NULL,
	"updated_at" bigint NOT NULL,
	"revision" bigint DEFAULT 1 NOT NULL,
	"tags" jsonb DEFAULT '{}' NOT NULL,
	"space_id" text NOT NULL,
	"snapshot_id" text NOT NULL,
	"destination_resource_id" text NOT NULL,
	"started_at" bigint,
	"completed_at" bigint,
	"outcome" text,
	"error" text,
	CONSTRAINT "restoration_completion" CHECK (("completed_at" IS NULL) = ("outcome" IS NULL)),
	CONSTRAINT "restoration_outcome" CHECK ("outcome" IS NULL OR "outcome" IN ('succeeded', 'failed', 'cancelled')),
	CONSTRAINT "restoration_success" CHECK ("outcome" IS NULL OR "outcome" <> 'succeeded' OR ("started_at" IS NOT NULL AND "error" IS NULL)),
	CONSTRAINT "restoration_finish" CHECK ("completed_at" IS NULL OR "started_at" IS NULL OR "completed_at" >= "started_at"),
	CONSTRAINT "restoration_time" CHECK (("started_at" IS NULL OR "started_at" >= "created_at") AND ("completed_at" IS NULL OR "completed_at" >= "created_at"))
);
--> statement-breakpoint
CREATE TABLE "snapshot" (
	"id" text PRIMARY KEY,
	"created_at" bigint NOT NULL,
	"updated_at" bigint NOT NULL,
	"revision" bigint DEFAULT 1 NOT NULL,
	"tags" jsonb DEFAULT '{}' NOT NULL,
	"space_id" text NOT NULL,
	"resource_id" text NOT NULL,
	"provider" text NOT NULL,
	"region_id" text,
	"reference" text NOT NULL,
	"format" text NOT NULL,
	"position" text NOT NULL,
	"digest" text,
	"verified_at" bigint,
	"retain_until" bigint NOT NULL,
	"deleted_at" bigint,
	CONSTRAINT "snapshot_space_id" UNIQUE("space_id","id"),
	CONSTRAINT "snapshot_resource_id" UNIQUE("resource_id","id"),
	CONSTRAINT "snapshot_retention" CHECK ("retain_until" >= "created_at" AND ("deleted_at" IS NULL OR "deleted_at" >= "retain_until")),
	CONSTRAINT "snapshot_verification" CHECK ("verified_at" IS NULL OR "verified_at" >= "created_at")
);
--> statement-breakpoint
CREATE UNIQUE INDEX "space_migration_active" ON "space_migration" ("space_id") WHERE "completed_at" IS NULL;--> statement-breakpoint
CREATE INDEX "space_migration_history" ON "space_migration" ("space_id","created_at");--> statement-breakpoint
CREATE INDEX "resource_binding_resource" ON "resource_binding" ("space_id","resource_id");--> statement-breakpoint
CREATE UNIQUE INDEX "resource_migration_active" ON "resource_migration" ("resource_id") WHERE "completed_at" IS NULL;--> statement-breakpoint
CREATE INDEX "resource_migration_history" ON "resource_migration" ("resource_id","created_at");--> statement-breakpoint
CREATE INDEX "resource_migration_space_migration" ON "resource_migration" ("space_migration_id");--> statement-breakpoint
CREATE UNIQUE INDEX "secret_source" ON "secret" (("source"::jsonb ->> 'kind'),coalesce(("source"::jsonb ->> 'spaceId'), ("source"::jsonb ->> 'installationId')),("source"::jsonb ->> 'name')) WHERE "source" IS NOT NULL AND "source_detached_at" IS NULL;--> statement-breakpoint
CREATE UNIQUE INDEX "resource_source" ON "resource" (("source"::jsonb ->> 'kind'),coalesce(("source"::jsonb ->> 'spaceId'), ("source"::jsonb ->> 'installationId')),("source"::jsonb ->> 'name')) WHERE "source" IS NOT NULL AND "source_detached_at" IS NULL;--> statement-breakpoint
CREATE UNIQUE INDEX "installation_source" ON "installation" (("source"::jsonb ->> 'kind'),coalesce(("source"::jsonb ->> 'spaceId'), ("source"::jsonb ->> 'installationId')),("source"::jsonb ->> 'name')) WHERE "source" IS NOT NULL AND "source_detached_at" IS NULL;--> statement-breakpoint
CREATE INDEX "installation_release" ON "installation" ("package_id","version");--> statement-breakpoint
CREATE UNIQUE INDEX "package_policy_source" ON "package_policy" (("source"::jsonb ->> 'kind'),coalesce(("source"::jsonb ->> 'spaceId'), ("source"::jsonb ->> 'installationId')),("source"::jsonb ->> 'name')) WHERE "source" IS NOT NULL AND "source_detached_at" IS NULL;--> statement-breakpoint
CREATE UNIQUE INDEX "package_policy_account" ON "package_policy" ("account_id") WHERE "scope" = 'account';--> statement-breakpoint
CREATE UNIQUE INDEX "package_policy_space" ON "package_policy" ("space_id") WHERE "scope" = 'space';--> statement-breakpoint
CREATE UNIQUE INDEX "role_source" ON "role" (("source"::jsonb ->> 'kind'),coalesce(("source"::jsonb ->> 'spaceId'), ("source"::jsonb ->> 'installationId')),("source"::jsonb ->> 'name')) WHERE "source" IS NOT NULL AND "source_detached_at" IS NULL;--> statement-breakpoint
CREATE UNIQUE INDEX "role_binding_source" ON "role_binding" (("source"::jsonb ->> 'kind'),coalesce(("source"::jsonb ->> 'spaceId'), ("source"::jsonb ->> 'installationId')),("source"::jsonb ->> 'name')) WHERE "source" IS NOT NULL AND "source_detached_at" IS NULL;--> statement-breakpoint
CREATE UNIQUE INDEX "role_binding_active" ON "role_binding" ("role_id","space_id",coalesce("account_membership_id", "group_id", "service_account_id", "account_service_account_id")) WHERE "revoked_at" IS NULL;--> statement-breakpoint
CREATE UNIQUE INDEX "role_permission_object" ON "role_permission" ("role_id","resource","action","resource_id") WHERE "resource_id" IS NOT NULL;--> statement-breakpoint
CREATE UNIQUE INDEX "role_permission_all" ON "role_permission" ("role_id","resource","action") WHERE "resource_id" IS NULL;--> statement-breakpoint
CREATE UNIQUE INDEX "schedule_source" ON "schedule" (("source"::jsonb ->> 'kind'),coalesce(("source"::jsonb ->> 'spaceId'), ("source"::jsonb ->> 'installationId')),("source"::jsonb ->> 'name')) WHERE "source" IS NOT NULL AND "source_detached_at" IS NULL;--> statement-breakpoint
CREATE INDEX "schedule_installation" ON "schedule" ("installation_id");--> statement-breakpoint
CREATE UNIQUE INDEX "network_policy_source" ON "network_policy" (("source"::jsonb ->> 'kind'),coalesce(("source"::jsonb ->> 'spaceId'), ("source"::jsonb ->> 'installationId')),("source"::jsonb ->> 'name')) WHERE "source" IS NOT NULL AND "source_detached_at" IS NULL;--> statement-breakpoint
CREATE UNIQUE INDEX "network_policy_installation" ON "network_policy" ("installation_id") WHERE "scope" = 'installation';--> statement-breakpoint
CREATE UNIQUE INDEX "network_policy_workload" ON "network_policy" ("installation_id","workload") WHERE "scope" = 'workload';--> statement-breakpoint
CREATE UNIQUE INDEX "network_policy_account" ON "network_policy" ("account_id") WHERE "scope" = 'account';--> statement-breakpoint
CREATE UNIQUE INDEX "network_policy_space" ON "network_policy" ("space_id") WHERE "scope" = 'space';--> statement-breakpoint
CREATE INDEX "deployment_installation_state" ON "deployment" ("installation_id","state");--> statement-breakpoint
CREATE INDEX "instance_deployment_state" ON "instance" ("deployment_id","state");--> statement-breakpoint
CREATE INDEX "instance_host_state" ON "instance" ("host_id","state");--> statement-breakpoint
ALTER TABLE "space_migration" ADD CONSTRAINT "space_migration_account_id_space_id_space_account_id_id_fkey" FOREIGN KEY ("account_id","space_id") REFERENCES "space"("account_id","id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "resource_binding" ADD CONSTRAINT "resource_binding_JsKTzo9Bzlk4_fkey" FOREIGN KEY ("space_id","installation_id") REFERENCES "installation"("space_id","id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "resource_binding" ADD CONSTRAINT "resource_binding_space_id_resource_id_resource_space_id_id_fkey" FOREIGN KEY ("space_id","resource_id") REFERENCES "resource"("space_id","id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "resource_migration" ADD CONSTRAINT "resource_migration_4UbLvK2m6A4D_fkey" FOREIGN KEY ("resource_id","snapshot_id") REFERENCES "snapshot"("resource_id","id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "resource_migration" ADD CONSTRAINT "resource_migration_account_id_space_id_space_account_id_id_fkey" FOREIGN KEY ("account_id","space_id") REFERENCES "space"("account_id","id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "resource_migration" ADD CONSTRAINT "resource_migration_RabvczZuksIR_fkey" FOREIGN KEY ("space_id","space_migration_id") REFERENCES "space_migration"("space_id","id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "resource_migration" ADD CONSTRAINT "resource_migration_hdlCv7W6iBJp_fkey" FOREIGN KEY ("space_id","resource_id") REFERENCES "resource"("space_id","id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "vault" ADD CONSTRAINT "vault_space_id_resource_id_kind_resource_space_id_id_kind_fkey" FOREIGN KEY ("space_id","resource_id","kind") REFERENCES "resource"("space_id","id","kind") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "secret" ADD CONSTRAINT "secret_space_id_vault_id_vault_space_id_resource_id_fkey" FOREIGN KEY ("space_id","vault_id") REFERENCES "vault"("space_id","resource_id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "secret" ADD CONSTRAINT "secret_id_current_version_secret_version_secret_id_version_fkey" FOREIGN KEY ("id","current_version") REFERENCES "secret_version"("secret_id","version") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "secret_version" ADD CONSTRAINT "secret_version_secret_id_secret_id_fkey" FOREIGN KEY ("secret_id") REFERENCES "secret"("id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "secret_binding" ADD CONSTRAINT "secret_binding_x2dtNhi3BR70_fkey" FOREIGN KEY ("space_id","installation_id") REFERENCES "installation"("space_id","id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "secret_binding" ADD CONSTRAINT "secret_binding_space_id_secret_id_secret_space_id_id_fkey" FOREIGN KEY ("space_id","secret_id") REFERENCES "secret"("space_id","id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "secret_binding" ADD CONSTRAINT "secret_binding_gNNr9zduKkzC_fkey" FOREIGN KEY ("secret_id","version") REFERENCES "secret_version"("secret_id","version") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "deployment_secret_binding" ADD CONSTRAINT "deployment_secret_binding_aVDIrn84mM2c_fkey" FOREIGN KEY ("space_id","deployment_id") REFERENCES "deployment"("space_id","id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "deployment_secret_binding" ADD CONSTRAINT "deployment_secret_binding_95TAuMbAwAdI_fkey" FOREIGN KEY ("space_id","secret_id") REFERENCES "secret"("space_id","id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "deployment_secret_binding" ADD CONSTRAINT "deployment_secret_binding_H4HaC3rS1dqY_fkey" FOREIGN KEY ("secret_id","version") REFERENCES "secret_version"("secret_id","version") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "resource" ADD CONSTRAINT "resource_account_id_space_id_space_account_id_id_fkey" FOREIGN KEY ("account_id","space_id") REFERENCES "space"("account_id","id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "resource" ADD CONSTRAINT "resource_space_id_space_id_fkey" FOREIGN KEY ("space_id") REFERENCES "space"("id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "deployment_binding" ADD CONSTRAINT "deployment_binding_sKHriQNrM0qr_fkey" FOREIGN KEY ("space_id","deployment_id") REFERENCES "deployment"("space_id","id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "deployment_binding" ADD CONSTRAINT "deployment_binding_aHlHGW0fQkol_fkey" FOREIGN KEY ("space_id","resource_id") REFERENCES "resource"("space_id","id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "installation" ADD CONSTRAINT "installation_space_id_space_id_fkey" FOREIGN KEY ("space_id") REFERENCES "space"("id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "repository_reference" ADD CONSTRAINT "repository_reference_repository_id_repository_id_fkey" FOREIGN KEY ("repository_id") REFERENCES "repository"("id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "release" ADD CONSTRAINT "release_repository_id_package_id_package_repository_id_id_fkey" FOREIGN KEY ("repository_id","package_id") REFERENCES "package"("repository_id","id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "release" ADD CONSTRAINT "release_package_id_package_id_fkey" FOREIGN KEY ("package_id") REFERENCES "package"("id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "release" ADD CONSTRAINT "release_repository_id_repository_id_fkey" FOREIGN KEY ("repository_id") REFERENCES "repository"("id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "package" ADD CONSTRAINT "package_account_id_repository_id_repository_account_id_id_fkey" FOREIGN KEY ("account_id","repository_id") REFERENCES "repository"("account_id","id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "package_policy" ADD CONSTRAINT "package_policy_account_id_space_id_space_account_id_id_fkey" FOREIGN KEY ("account_id","space_id") REFERENCES "space"("account_id","id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "package_policy" ADD CONSTRAINT "package_policy_xvCs0PPbi1QQ_fkey" FOREIGN KEY ("id","current_revision_id") REFERENCES "package_policy_revision"("policy_id","id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "package_policy_revision" ADD CONSTRAINT "package_policy_revision_policy_id_package_policy_id_fkey" FOREIGN KEY ("policy_id") REFERENCES "package_policy"("id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "role" ADD CONSTRAINT "role_account_id_space_id_space_account_id_id_fkey" FOREIGN KEY ("account_id","space_id") REFERENCES "space"("account_id","id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "role_binding" ADD CONSTRAINT "role_binding_space_id_role_id_role_space_id_id_fkey" FOREIGN KEY ("space_id","role_id") REFERENCES "role"("space_id","id");--> statement-breakpoint
ALTER TABLE "role_binding" ADD CONSTRAINT "role_binding_account_id_space_id_space_account_id_id_fkey" FOREIGN KEY ("account_id","space_id") REFERENCES "space"("account_id","id");--> statement-breakpoint
ALTER TABLE "role_binding" ADD CONSTRAINT "role_binding_9Gt5N7hzvGhC_fkey" FOREIGN KEY ("space_id","service_account_id") REFERENCES "service_account"("space_id","id");--> statement-breakpoint
ALTER TABLE "role_permission" ADD CONSTRAINT "role_permission_role_id_role_id_fkey" FOREIGN KEY ("role_id") REFERENCES "role"("id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "service_account" ADD CONSTRAINT "service_account_account_id_space_id_space_account_id_id_fkey" FOREIGN KEY ("account_id","space_id") REFERENCES "space"("account_id","id");--> statement-breakpoint
ALTER TABLE "service_account" ADD CONSTRAINT "service_account_uhEfsIBjdTbR_fkey" FOREIGN KEY ("space_id","installation_id") REFERENCES "installation"("space_id","id");--> statement-breakpoint
ALTER TABLE "service_token" ADD CONSTRAINT "service_token_service_account_id_service_account_id_fkey" FOREIGN KEY ("service_account_id") REFERENCES "service_account"("id");--> statement-breakpoint
ALTER TABLE "space_host" ADD CONSTRAINT "space_host_account_id_space_id_space_account_id_id_fkey" FOREIGN KEY ("account_id","space_id") REFERENCES "space"("account_id","id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "schedule" ADD CONSTRAINT "schedule_space_id_installation_id_installation_space_id_id_fkey" FOREIGN KEY ("space_id","installation_id") REFERENCES "installation"("space_id","id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "schedule" ADD CONSTRAINT "schedule_gzBE4fhR2j7T_fkey" FOREIGN KEY ("space_id","service_account_id") REFERENCES "service_account"("space_id","id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "network_policy" ADD CONSTRAINT "network_policy_account_id_space_id_space_account_id_id_fkey" FOREIGN KEY ("account_id","space_id") REFERENCES "space"("account_id","id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "network_policy" ADD CONSTRAINT "network_policy_4Y2hWTY0wng8_fkey" FOREIGN KEY ("id","current_revision_id") REFERENCES "network_policy_revision"("policy_id","id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "network_policy" ADD CONSTRAINT "network_policy_7AS7x8uXLs0z_fkey" FOREIGN KEY ("space_id","installation_id") REFERENCES "installation"("space_id","id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "network_policy_revision" ADD CONSTRAINT "network_policy_revision_policy_id_network_policy_id_fkey" FOREIGN KEY ("policy_id") REFERENCES "network_policy"("id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "deployment" ADD CONSTRAINT "deployment_ISYznpIySk33_fkey" FOREIGN KEY ("space_id","installation_id","package_id") REFERENCES "installation"("space_id","id","package_id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "deployment" ADD CONSTRAINT "deployment_space_id_host_id_space_host_space_id_host_id_fkey" FOREIGN KEY ("space_id","host_id") REFERENCES "space_host"("space_id","host_id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "deployment" ADD CONSTRAINT "deployment_FSmeh0YY8Wj3_fkey" FOREIGN KEY ("installation_id","workload","service_account_id") REFERENCES "service_account"("installation_id","workload","id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "instance" ADD CONSTRAINT "instance_space_id_deployment_id_deployment_space_id_id_fkey" FOREIGN KEY ("space_id","deployment_id") REFERENCES "deployment"("space_id","id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "instance" ADD CONSTRAINT "instance_space_id_host_id_space_host_space_id_host_id_fkey" FOREIGN KEY ("space_id","host_id") REFERENCES "space_host"("space_id","host_id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "space_configuration" ADD CONSTRAINT "space_configuration_wo0ktYyXElYS_fkey" FOREIGN KEY ("space_id","applied_revision_id") REFERENCES "stack_revision"("space_id","id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "space_configuration" ADD CONSTRAINT "space_configuration_space_id_space_id_fkey" FOREIGN KEY ("space_id") REFERENCES "space"("id");--> statement-breakpoint
ALTER TABLE "stack_revision" ADD CONSTRAINT "stack_revision_space_id_space_id_fkey" FOREIGN KEY ("space_id") REFERENCES "space"("id");--> statement-breakpoint
ALTER TABLE "restoration" ADD CONSTRAINT "restoration_space_id_snapshot_id_snapshot_space_id_id_fkey" FOREIGN KEY ("space_id","snapshot_id") REFERENCES "snapshot"("space_id","id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "restoration" ADD CONSTRAINT "restoration_hQnAjCrvAs2d_fkey" FOREIGN KEY ("space_id","destination_resource_id") REFERENCES "resource"("space_id","id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "restoration" ADD CONSTRAINT "restoration_snapshot_id_snapshot_id_fkey" FOREIGN KEY ("snapshot_id") REFERENCES "snapshot"("id");--> statement-breakpoint
ALTER TABLE "restoration" ADD CONSTRAINT "restoration_destination_resource_id_resource_id_fkey" FOREIGN KEY ("destination_resource_id") REFERENCES "resource"("id");--> statement-breakpoint
ALTER TABLE "snapshot" ADD CONSTRAINT "snapshot_space_id_resource_id_resource_space_id_id_fkey" FOREIGN KEY ("space_id","resource_id") REFERENCES "resource"("space_id","id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "snapshot" ADD CONSTRAINT "snapshot_resource_id_resource_id_fkey" FOREIGN KEY ("resource_id") REFERENCES "resource"("id");