CREATE TABLE "account" (
	"id" text PRIMARY KEY,
	"created_at" bigint NOT NULL,
	"updated_at" bigint NOT NULL,
	"revision" bigint DEFAULT 1 NOT NULL,
	"tags" jsonb DEFAULT '{}' NOT NULL,
	"handle" text NOT NULL UNIQUE,
	"name" text NOT NULL,
	"default_residency" text NOT NULL,
	"package_policy_id" text,
	"package_policy_region_id" text,
	"network_policy_id" text,
	"network_policy_region_id" text,
	"suspended_at" bigint,
	"deletion_requested_at" bigint,
	"kind" text NOT NULL,
	"user_id" text UNIQUE,
	CONSTRAINT "account_residency" CHECK ("default_residency" IN ('eu', 'us')),
	CONSTRAINT "account_kind" CHECK ("kind" IN ('personal', 'organisation')),
	CONSTRAINT "account_network_policy" CHECK (
        ("network_policy_id" IS NULL AND "network_policy_region_id" IS NULL) OR
        ("network_policy_id" IS NOT NULL AND "network_policy_region_id" IS NOT NULL)
    ),
	CONSTRAINT "account_package_policy" CHECK (
        ("package_policy_id" IS NULL AND "package_policy_region_id" IS NULL) OR
        ("package_policy_id" IS NOT NULL AND "package_policy_region_id" IS NOT NULL)
    ),
	CONSTRAINT "account_user" CHECK (("kind" = 'personal' AND "user_id" IS NOT NULL) OR ("kind" = 'organisation' AND "user_id" IS NULL)),
	CONSTRAINT "account_handle" CHECK (length("handle") BETWEEN 1 AND 63 AND ("handle" COLLATE "C") !~ '[^a-z0-9-]' AND "handle" NOT LIKE '-%' AND "handle" NOT LIKE '%-')
);
--> statement-breakpoint
CREATE TABLE "identity" (
	"id" text PRIMARY KEY,
	"created_at" bigint NOT NULL,
	"updated_at" bigint NOT NULL,
	"revision" bigint DEFAULT 1 NOT NULL,
	"tags" jsonb DEFAULT '{}' NOT NULL,
	"user_id" text NOT NULL,
	"issuer" text NOT NULL,
	"subject" text NOT NULL,
	CONSTRAINT "identity_issuer_subject" UNIQUE("issuer","subject")
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
CREATE TABLE "sign_in_request" (
	"id" text PRIMARY KEY,
	"created_at" bigint NOT NULL,
	"updated_at" bigint NOT NULL,
	"revision" bigint DEFAULT 1 NOT NULL,
	"tags" jsonb DEFAULT '{}' NOT NULL,
	"client" text NOT NULL,
	"token_hash" text NOT NULL UNIQUE,
	"code_hash" text NOT NULL UNIQUE,
	"challenge" text NOT NULL,
	"user_id" text,
	"expires_at" bigint NOT NULL,
	"approved_at" bigint,
	"denied_at" bigint,
	"consumed_at" bigint,
	CONSTRAINT "sign_in_request_client" CHECK ("client" IN ('desktop', 'cli')),
	CONSTRAINT "sign_in_request_expiry" CHECK ("expires_at" > "created_at"),
	CONSTRAINT "sign_in_request_approval" CHECK (("approved_at" IS NULL) = ("user_id" IS NULL) AND ("approved_at" IS NULL OR ("denied_at" IS NULL AND "approved_at" >= "created_at" AND "approved_at" < "expires_at"))),
	CONSTRAINT "sign_in_request_exchange" CHECK ("consumed_at" IS NULL OR ("approved_at" IS NOT NULL AND "consumed_at" >= "approved_at" AND "consumed_at" < "expires_at"))
);
--> statement-breakpoint
CREATE TABLE "user" (
	"id" text PRIMARY KEY,
	"created_at" bigint NOT NULL,
	"updated_at" bigint NOT NULL,
	"revision" bigint DEFAULT 1 NOT NULL,
	"tags" jsonb DEFAULT '{}' NOT NULL,
	"name" text NOT NULL
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
	"revoked_at" bigint,
	CONSTRAINT "service_account_name" UNIQUE("account_id","name"),
	CONSTRAINT "service_account_account_id" UNIQUE("account_id","id")
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
CREATE TABLE "preference" (
	"id" text PRIMARY KEY,
	"created_at" bigint NOT NULL,
	"updated_at" bigint NOT NULL,
	"revision" bigint DEFAULT 1 NOT NULL,
	"tags" jsonb DEFAULT '{}' NOT NULL,
	"account_id" text NOT NULL,
	"user_id" text,
	"device_id" text,
	"name" text NOT NULL,
	"value" jsonb NOT NULL,
	CONSTRAINT "preference_device_user" CHECK ("device_id" IS NULL OR "user_id" IS NOT NULL)
);
--> statement-breakpoint
CREATE TABLE "tunnel" (
	"id" text PRIMARY KEY,
	"created_at" bigint NOT NULL,
	"updated_at" bigint NOT NULL,
	"revision" bigint DEFAULT 1 NOT NULL,
	"tags" jsonb DEFAULT '{}' NOT NULL,
	"host_id" text NOT NULL,
	"device_id" text NOT NULL,
	"device_key_id" text NOT NULL,
	"relay" text NOT NULL,
	"connection" text NOT NULL,
	"heartbeat_at" bigint NOT NULL,
	"expires_at" bigint NOT NULL,
	"closed_at" bigint,
	CONSTRAINT "tunnel_connection" UNIQUE("relay","connection"),
	CONSTRAINT "tunnel_lease" CHECK ("heartbeat_at" >= "created_at" AND "expires_at" > "heartbeat_at"),
	CONSTRAINT "tunnel_close" CHECK ("closed_at" IS NULL OR "closed_at" >= "created_at")
);
--> statement-breakpoint
CREATE TABLE "host_access" (
	"host_id" text,
	"account_id" text,
	"created_at" bigint NOT NULL,
	"revoked_at" bigint,
	CONSTRAINT "host_access_pkey" PRIMARY KEY("account_id","host_id")
);
--> statement-breakpoint
CREATE TABLE "region" (
	"id" text PRIMARY KEY,
	"created_at" bigint NOT NULL,
	"updated_at" bigint NOT NULL,
	"revision" bigint DEFAULT 1 NOT NULL,
	"tags" jsonb DEFAULT '{}' NOT NULL,
	"provider" text NOT NULL,
	"code" text NOT NULL,
	"name" text NOT NULL,
	"residency" text NOT NULL,
	CONSTRAINT "region_provider_code" UNIQUE("provider","code"),
	CONSTRAINT "region_provider_id" UNIQUE("provider","id"),
	CONSTRAINT "region_residency_id" UNIQUE("id","residency"),
	CONSTRAINT "region_residency" CHECK ("residency" IN ('eu', 'us'))
);
--> statement-breakpoint
CREATE TABLE "device_key" (
	"id" text PRIMARY KEY,
	"created_at" bigint NOT NULL,
	"updated_at" bigint NOT NULL,
	"revision" bigint DEFAULT 1 NOT NULL,
	"tags" jsonb DEFAULT '{}' NOT NULL,
	"device_id" text NOT NULL,
	"public_key" text NOT NULL UNIQUE,
	"expires_at" bigint NOT NULL,
	"revoked_at" bigint,
	CONSTRAINT "device_key_device_id" UNIQUE("device_id","id"),
	CONSTRAINT "device_key_expiry" CHECK ("expires_at" > "created_at")
);
--> statement-breakpoint
CREATE TABLE "account_membership" (
	"id" text PRIMARY KEY,
	"created_at" bigint NOT NULL,
	"updated_at" bigint NOT NULL,
	"revision" bigint DEFAULT 1 NOT NULL,
	"tags" jsonb DEFAULT '{}' NOT NULL,
	"account_id" text NOT NULL,
	"user_id" text NOT NULL,
	CONSTRAINT "account_membership_account_id" UNIQUE("account_id","id"),
	CONSTRAINT "account_membership_account_user" UNIQUE("account_id","user_id")
);
--> statement-breakpoint
CREATE TABLE "device" (
	"id" text PRIMARY KEY,
	"created_at" bigint NOT NULL,
	"updated_at" bigint NOT NULL,
	"revision" bigint DEFAULT 1 NOT NULL,
	"tags" jsonb DEFAULT '{}' NOT NULL,
	"account_id" text NOT NULL,
	"name" text NOT NULL,
	"revoked_at" bigint,
	"last_seen_at" bigint,
	CONSTRAINT "device_account_id" UNIQUE("account_id","id")
);
--> statement-breakpoint
CREATE TABLE "host" (
	"id" text PRIMARY KEY,
	"created_at" bigint NOT NULL,
	"updated_at" bigint NOT NULL,
	"revision" bigint DEFAULT 1 NOT NULL,
	"tags" jsonb DEFAULT '{}' NOT NULL,
	"account_id" text NOT NULL,
	"kind" text NOT NULL,
	"device_id" text,
	"region_id" text,
	"state" text DEFAULT 'enabled' NOT NULL,
	"version" text,
	"runtimes" jsonb DEFAULT '[]' NOT NULL,
	"last_seen_at" bigint,
	CONSTRAINT "host_device_id" UNIQUE("device_id","id"),
	CONSTRAINT "host_account_id" UNIQUE("account_id","id"),
	CONSTRAINT "host_state" CHECK ("state" IN ('enabled', 'draining', 'disabled')),
	CONSTRAINT "host_kind" CHECK (("kind" = 'device' AND "device_id" IS NOT NULL) OR ("kind" = 'cloud' AND "device_id" IS NULL))
);
--> statement-breakpoint
CREATE TABLE "route" (
	"id" text PRIMARY KEY,
	"created_at" bigint NOT NULL,
	"updated_at" bigint NOT NULL,
	"revision" bigint DEFAULT 1 NOT NULL,
	"tags" jsonb DEFAULT '{}' NOT NULL,
	"source" jsonb,
	"source_detached_at" bigint,
	"account_id" text NOT NULL,
	"domain_id" text NOT NULL,
	"path" text NOT NULL,
	"match" text NOT NULL,
	"kind" text NOT NULL,
	"space_id" text,
	"installation_id" text,
	"entrypoint" text,
	"redirect" text,
	"redirect_status" bigint,
	"applied_generation" bigint DEFAULT 0 NOT NULL,
	"generation" bigint DEFAULT 1 NOT NULL,
	"observed_generation" bigint DEFAULT 0 NOT NULL,
	"conditions" jsonb DEFAULT '{}' NOT NULL,
	"deletion_requested_at" bigint,
	"finalizers" jsonb DEFAULT '[]' NOT NULL,
	CONSTRAINT "route_domain_path_match" UNIQUE("domain_id","path","match"),
	CONSTRAINT "route_source_detached" CHECK ("source_detached_at" IS NULL OR ("source" IS NOT NULL AND "source_detached_at" >= 0)),
	CONSTRAINT "route_revision" CHECK ("revision" >= 1),
	CONSTRAINT "route_generation" CHECK ("generation" >= 1),
	CONSTRAINT "route_observed_generation" CHECK ("observed_generation" BETWEEN 0 AND "generation"),
	CONSTRAINT "route_match" CHECK ("match" IN ('exact', 'prefix')),
	CONSTRAINT "route_path" CHECK (substr("path", 1, 1) = '/'),
	CONSTRAINT "route_applied_generation" CHECK ("applied_generation" BETWEEN 0 AND "generation"),
	CONSTRAINT "route_destination" CHECK (("kind" = 'application' AND "space_id" IS NOT NULL AND "installation_id" IS NOT NULL AND "entrypoint" IS NOT NULL AND "redirect" IS NULL AND "redirect_status" IS NULL) OR ("kind" = 'redirect' AND "space_id" IS NULL AND "installation_id" IS NULL AND "entrypoint" IS NULL AND "redirect" IS NOT NULL AND "redirect_status" IN (301, 302, 303, 307, 308) AND "redirect_status" IS NOT NULL))
);
--> statement-breakpoint
CREATE TABLE "domain" (
	"id" text PRIMARY KEY,
	"created_at" bigint NOT NULL,
	"updated_at" bigint NOT NULL,
	"revision" bigint DEFAULT 1 NOT NULL,
	"tags" jsonb DEFAULT '{}' NOT NULL,
	"account_id" text NOT NULL,
	"hostname" text NOT NULL UNIQUE,
	"verified_at" bigint,
	CONSTRAINT "domain_account_id" UNIQUE("account_id","id"),
	CONSTRAINT "domain_hostname" CHECK (length("hostname") BETWEEN 1 AND 253 AND "hostname" = lower("hostname"))
);
--> statement-breakpoint
CREATE TABLE "space_directory" (
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
	"residency" text NOT NULL,
	"region_id" text NOT NULL,
	"account_id" text NOT NULL,
	"name" text NOT NULL,
	CONSTRAINT "space_directory_name" UNIQUE("account_id","name"),
	CONSTRAINT "space_directory_account" UNIQUE("account_id","id"),
	CONSTRAINT "space_directory_revision" CHECK ("revision" >= 1),
	CONSTRAINT "space_directory_generation" CHECK ("generation" >= 1),
	CONSTRAINT "space_directory_observed_generation" CHECK ("observed_generation" BETWEEN 0 AND "generation"),
	CONSTRAINT "space_directory_name_value" CHECK (length("name") BETWEEN 1 AND 63 AND ("name" COLLATE "C") !~ '[^a-z0-9-]' AND "name" NOT LIKE '-%' AND "name" NOT LIKE '%-')
);
--> statement-breakpoint
CREATE TABLE "repository_directory" (
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
	"residency" text NOT NULL,
	"region_id" text NOT NULL,
	"account_id" text NOT NULL,
	"name" text NOT NULL,
	CONSTRAINT "repository_directory_name" UNIQUE("account_id","name"),
	CONSTRAINT "repository_directory_account" UNIQUE("account_id","id"),
	CONSTRAINT "repository_directory_revision" CHECK ("revision" >= 1),
	CONSTRAINT "repository_directory_generation" CHECK ("generation" >= 1),
	CONSTRAINT "repository_directory_observed_generation" CHECK ("observed_generation" BETWEEN 0 AND "generation"),
	CONSTRAINT "repository_directory_name_value" CHECK (length("name") BETWEEN 1 AND 63 AND ("name" COLLATE "C") !~ '[^a-z0-9-]' AND "name" NOT LIKE '-%' AND "name" NOT LIKE '%-')
);
--> statement-breakpoint
CREATE TABLE "package_directory" (
	"id" text PRIMARY KEY,
	"created_at" bigint NOT NULL,
	"updated_at" bigint NOT NULL,
	"revision" bigint DEFAULT 1 NOT NULL,
	"tags" jsonb DEFAULT '{}' NOT NULL,
	"account_id" text NOT NULL,
	"name" text NOT NULL,
	"repository_id" text NOT NULL,
	CONSTRAINT "package_directory_name" UNIQUE("account_id","name")
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
	"space_id" text,
	"account_membership_id" text,
	"group_id" text,
	"service_account_id" text,
	"expires_at" bigint,
	"revoked_at" bigint,
	CONSTRAINT "role_binding_source_detached" CHECK ("source_detached_at" IS NULL OR ("source" IS NOT NULL AND "source_detached_at" >= 0)),
	CONSTRAINT "role_binding_subject" CHECK (CAST("account_membership_id" IS NOT NULL AS integer) + CAST("group_id" IS NOT NULL AS integer) + CAST("service_account_id" IS NOT NULL AS integer) = 1),
	CONSTRAINT "role_binding_expiry" CHECK ("expires_at" IS NULL OR "expires_at" > "created_at")
);
--> statement-breakpoint
CREATE TABLE "account_invitation" (
	"id" text PRIMARY KEY,
	"created_at" bigint NOT NULL,
	"updated_at" bigint NOT NULL,
	"revision" bigint DEFAULT 1 NOT NULL,
	"tags" jsonb DEFAULT '{}' NOT NULL,
	"account_id" text NOT NULL,
	"invited_by" text NOT NULL,
	"email" text NOT NULL,
	"token_hash" text NOT NULL UNIQUE,
	"expires_at" bigint NOT NULL,
	"accepted_by" text,
	"accepted_at" bigint,
	"revoked_at" bigint,
	CONSTRAINT "account_invitation_expiry" CHECK ("expires_at" > "created_at"),
	CONSTRAINT "account_invitation_acceptance" CHECK (("accepted_at" IS NULL) = ("accepted_by" IS NULL) AND ("accepted_at" IS NULL OR ("accepted_at" >= "created_at" AND "accepted_at" < "expires_at" AND "revoked_at" IS NULL)))
);
--> statement-breakpoint
CREATE TABLE "group" (
	"id" text PRIMARY KEY,
	"created_at" bigint NOT NULL,
	"updated_at" bigint NOT NULL,
	"revision" bigint DEFAULT 1 NOT NULL,
	"tags" jsonb DEFAULT '{}' NOT NULL,
	"account_id" text NOT NULL,
	"name" text NOT NULL,
	CONSTRAINT "group_account_name" UNIQUE("account_id","name"),
	CONSTRAINT "group_account_id" UNIQUE("account_id","id")
);
--> statement-breakpoint
CREATE TABLE "group_membership" (
	"account_id" text NOT NULL,
	"group_id" text,
	"account_membership_id" text,
	CONSTRAINT "group_membership_pkey" PRIMARY KEY("group_id","account_membership_id")
);
--> statement-breakpoint
CREATE TABLE "connected_account" (
	"id" text PRIMARY KEY,
	"created_at" bigint NOT NULL,
	"updated_at" bigint NOT NULL,
	"revision" bigint DEFAULT 1 NOT NULL,
	"tags" jsonb DEFAULT '{}' NOT NULL,
	"account_id" text NOT NULL,
	"user_id" text NOT NULL,
	"provider" text NOT NULL,
	"issuer" text NOT NULL,
	"subject" text NOT NULL,
	"application_id" text NOT NULL,
	"kind" text NOT NULL,
	"installation_id" text,
	"scopes" jsonb NOT NULL,
	"permissions" jsonb NOT NULL,
	"secret_space_id" text,
	"secret_id" text,
	"expires_at" bigint,
	"revoked_at" bigint,
	CONSTRAINT "connection_account_id" UNIQUE("account_id","id"),
	CONSTRAINT "connection_authorisation" CHECK (("kind" = 'oauth' AND "installation_id" IS NULL AND "secret_space_id" IS NOT NULL AND "secret_id" IS NOT NULL)
            OR ("kind" = 'installation' AND "installation_id" IS NOT NULL AND "secret_space_id" IS NULL AND "secret_id" IS NULL)),
	CONSTRAINT "connection_identifiers" CHECK (length("provider") > 0 AND length("issuer") > 0
            AND length("subject") > 0 AND length("application_id") > 0
            AND ("installation_id" IS NULL OR length("installation_id") > 0))
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
	"name" text NOT NULL,
	"description" text NOT NULL,
	CONSTRAINT "role_account_id" UNIQUE("account_id","id"),
	CONSTRAINT "role_source_detached" CHECK ("source_detached_at" IS NULL OR ("source" IS NOT NULL AND "source_detached_at" >= 0))
);
--> statement-breakpoint
CREATE TABLE "session" (
	"id" text PRIMARY KEY,
	"user_id" text NOT NULL,
	"device_id" text,
	"token_hash" text NOT NULL UNIQUE,
	"created_at" bigint NOT NULL,
	"expires_at" bigint NOT NULL,
	"revoked_at" bigint,
	CONSTRAINT "session_expiry_order" CHECK ("expires_at" > "created_at")
);
--> statement-breakpoint
CREATE INDEX "identity_user" ON "identity" ("user_id");--> statement-breakpoint
CREATE UNIQUE INDEX "role_permission_object" ON "role_permission" ("role_id","resource","action","resource_id") WHERE "resource_id" IS NOT NULL;--> statement-breakpoint
CREATE UNIQUE INDEX "role_permission_all" ON "role_permission" ("role_id","resource","action") WHERE "resource_id" IS NULL;--> statement-breakpoint
CREATE UNIQUE INDEX "preference_account" ON "preference" ("account_id","name") WHERE "user_id" IS NULL AND "device_id" IS NULL;--> statement-breakpoint
CREATE UNIQUE INDEX "preference_user" ON "preference" ("account_id","user_id","name") WHERE "user_id" IS NOT NULL AND "device_id" IS NULL;--> statement-breakpoint
CREATE UNIQUE INDEX "preference_device" ON "preference" ("account_id","user_id","device_id","name") WHERE "device_id" IS NOT NULL;--> statement-breakpoint
CREATE INDEX "tunnel_host_expiry" ON "tunnel" ("host_id","expires_at");--> statement-breakpoint
CREATE INDEX "account_membership_user" ON "account_membership" ("user_id");--> statement-breakpoint
CREATE UNIQUE INDEX "route_source" ON "route" (("source"::jsonb ->> 'kind'),coalesce(("source"::jsonb ->> 'spaceId'), ("source"::jsonb ->> 'installationId')),("source"::jsonb ->> 'name')) WHERE "source" IS NOT NULL AND "source_detached_at" IS NULL;--> statement-breakpoint
CREATE UNIQUE INDEX "role_binding_source" ON "role_binding" (("source"::jsonb ->> 'kind'),coalesce(("source"::jsonb ->> 'spaceId'), ("source"::jsonb ->> 'installationId')),("source"::jsonb ->> 'name')) WHERE "source" IS NOT NULL AND "source_detached_at" IS NULL;--> statement-breakpoint
CREATE UNIQUE INDEX "role_binding_active" ON "role_binding" ("role_id",coalesce("space_id", ''),coalesce("account_membership_id", "group_id", "service_account_id")) WHERE "revoked_at" IS NULL;--> statement-breakpoint
CREATE UNIQUE INDEX "connection_oauth" ON "connected_account" ("account_id","provider","issuer","application_id","subject") WHERE "kind" = 'oauth';--> statement-breakpoint
CREATE UNIQUE INDEX "connection_installation" ON "connected_account" ("account_id","provider","issuer","application_id","installation_id") WHERE "kind" = 'installation';--> statement-breakpoint
CREATE INDEX "connection_user" ON "connected_account" ("user_id");--> statement-breakpoint
CREATE UNIQUE INDEX "role_source" ON "role" (("source"::jsonb ->> 'kind'),coalesce(("source"::jsonb ->> 'spaceId'), ("source"::jsonb ->> 'installationId')),("source"::jsonb ->> 'name')) WHERE "source" IS NOT NULL AND "source_detached_at" IS NULL;--> statement-breakpoint
CREATE UNIQUE INDEX "role_scope_name" ON "role" ("account_id","name");--> statement-breakpoint
CREATE INDEX "session_user" ON "session" ("user_id");--> statement-breakpoint
CREATE INDEX "session_expiry" ON "session" ("expires_at");--> statement-breakpoint
ALTER TABLE "account" ADD CONSTRAINT "account_package_policy_region_id_region_id_fkey" FOREIGN KEY ("package_policy_region_id") REFERENCES "region"("id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "account" ADD CONSTRAINT "account_network_policy_region_id_region_id_fkey" FOREIGN KEY ("network_policy_region_id") REFERENCES "region"("id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "account" ADD CONSTRAINT "account_user_id_user_id_fkey" FOREIGN KEY ("user_id") REFERENCES "user"("id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "identity" ADD CONSTRAINT "identity_user_id_user_id_fkey" FOREIGN KEY ("user_id") REFERENCES "user"("id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "role_permission" ADD CONSTRAINT "role_permission_role_id_role_id_fkey" FOREIGN KEY ("role_id") REFERENCES "role"("id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "sign_in_request" ADD CONSTRAINT "sign_in_request_user_id_user_id_fkey" FOREIGN KEY ("user_id") REFERENCES "user"("id");--> statement-breakpoint
ALTER TABLE "service_account" ADD CONSTRAINT "service_account_account_id_account_id_fkey" FOREIGN KEY ("account_id") REFERENCES "account"("id");--> statement-breakpoint
ALTER TABLE "service_token" ADD CONSTRAINT "service_token_service_account_id_service_account_id_fkey" FOREIGN KEY ("service_account_id") REFERENCES "service_account"("id");--> statement-breakpoint
ALTER TABLE "preference" ADD CONSTRAINT "preference_account_id_device_id_device_account_id_id_fkey" FOREIGN KEY ("account_id","device_id") REFERENCES "device"("account_id","id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "preference" ADD CONSTRAINT "preference_account_id_account_id_fkey" FOREIGN KEY ("account_id") REFERENCES "account"("id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "preference" ADD CONSTRAINT "preference_user_id_user_id_fkey" FOREIGN KEY ("user_id") REFERENCES "user"("id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "preference" ADD CONSTRAINT "preference_device_id_device_id_fkey" FOREIGN KEY ("device_id") REFERENCES "device"("id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "tunnel" ADD CONSTRAINT "tunnel_device_id_host_id_host_device_id_id_fkey" FOREIGN KEY ("device_id","host_id") REFERENCES "host"("device_id","id");--> statement-breakpoint
ALTER TABLE "tunnel" ADD CONSTRAINT "tunnel_device_id_device_key_id_device_key_device_id_id_fkey" FOREIGN KEY ("device_id","device_key_id") REFERENCES "device_key"("device_id","id");--> statement-breakpoint
ALTER TABLE "tunnel" ADD CONSTRAINT "tunnel_host_id_host_id_fkey" FOREIGN KEY ("host_id") REFERENCES "host"("id");--> statement-breakpoint
ALTER TABLE "tunnel" ADD CONSTRAINT "tunnel_device_id_device_id_fkey" FOREIGN KEY ("device_id") REFERENCES "device"("id");--> statement-breakpoint
ALTER TABLE "host_access" ADD CONSTRAINT "host_access_host_id_host_id_fkey" FOREIGN KEY ("host_id") REFERENCES "host"("id");--> statement-breakpoint
ALTER TABLE "host_access" ADD CONSTRAINT "host_access_account_id_account_id_fkey" FOREIGN KEY ("account_id") REFERENCES "account"("id");--> statement-breakpoint
ALTER TABLE "device_key" ADD CONSTRAINT "device_key_device_id_device_id_fkey" FOREIGN KEY ("device_id") REFERENCES "device"("id");--> statement-breakpoint
ALTER TABLE "account_membership" ADD CONSTRAINT "account_membership_account_id_account_id_fkey" FOREIGN KEY ("account_id") REFERENCES "account"("id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "account_membership" ADD CONSTRAINT "account_membership_user_id_user_id_fkey" FOREIGN KEY ("user_id") REFERENCES "user"("id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "device" ADD CONSTRAINT "device_account_id_account_id_fkey" FOREIGN KEY ("account_id") REFERENCES "account"("id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "host" ADD CONSTRAINT "host_account_id_device_id_device_account_id_id_fkey" FOREIGN KEY ("account_id","device_id") REFERENCES "device"("account_id","id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "host" ADD CONSTRAINT "host_account_id_account_id_fkey" FOREIGN KEY ("account_id") REFERENCES "account"("id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "host" ADD CONSTRAINT "host_region_id_region_id_fkey" FOREIGN KEY ("region_id") REFERENCES "region"("id");--> statement-breakpoint
ALTER TABLE "route" ADD CONSTRAINT "route_account_id_domain_id_domain_account_id_id_fkey" FOREIGN KEY ("account_id","domain_id") REFERENCES "domain"("account_id","id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "route" ADD CONSTRAINT "route_account_id_account_id_fkey" FOREIGN KEY ("account_id") REFERENCES "account"("id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "route" ADD CONSTRAINT "route_space_id_space_directory_id_fkey" FOREIGN KEY ("space_id") REFERENCES "space_directory"("id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "domain" ADD CONSTRAINT "domain_account_id_account_id_fkey" FOREIGN KEY ("account_id") REFERENCES "account"("id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "space_directory" ADD CONSTRAINT "space_directory_region_id_residency_region_id_residency_fkey" FOREIGN KEY ("region_id","residency") REFERENCES "region"("id","residency") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "space_directory" ADD CONSTRAINT "space_directory_account_id_account_id_fkey" FOREIGN KEY ("account_id") REFERENCES "account"("id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "repository_directory" ADD CONSTRAINT "repository_directory_WuFBEyWrlU6q_fkey" FOREIGN KEY ("region_id","residency") REFERENCES "region"("id","residency") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "repository_directory" ADD CONSTRAINT "repository_directory_account_id_account_id_fkey" FOREIGN KEY ("account_id") REFERENCES "account"("id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "package_directory" ADD CONSTRAINT "package_directory_m2XXeeZRYxUc_fkey" FOREIGN KEY ("account_id","repository_id") REFERENCES "repository_directory"("account_id","id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "role_binding" ADD CONSTRAINT "role_binding_KXZJoiZdkaOA_fkey" FOREIGN KEY ("account_id","space_id") REFERENCES "space_directory"("account_id","id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "role_binding" ADD CONSTRAINT "role_binding_account_id_role_id_role_account_id_id_fkey" FOREIGN KEY ("account_id","role_id") REFERENCES "role"("account_id","id");--> statement-breakpoint
ALTER TABLE "role_binding" ADD CONSTRAINT "role_binding_fXOt6GFY2BtO_fkey" FOREIGN KEY ("account_id","account_membership_id") REFERENCES "account_membership"("account_id","id");--> statement-breakpoint
ALTER TABLE "role_binding" ADD CONSTRAINT "role_binding_account_id_group_id_group_account_id_id_fkey" FOREIGN KEY ("account_id","group_id") REFERENCES "group"("account_id","id");--> statement-breakpoint
ALTER TABLE "role_binding" ADD CONSTRAINT "role_binding_AMsSsGJGSTWe_fkey" FOREIGN KEY ("account_id","service_account_id") REFERENCES "service_account"("account_id","id");--> statement-breakpoint
ALTER TABLE "account_invitation" ADD CONSTRAINT "account_invitation_account_id_account_id_fkey" FOREIGN KEY ("account_id") REFERENCES "account"("id");--> statement-breakpoint
ALTER TABLE "account_invitation" ADD CONSTRAINT "account_invitation_invited_by_user_id_fkey" FOREIGN KEY ("invited_by") REFERENCES "user"("id");--> statement-breakpoint
ALTER TABLE "account_invitation" ADD CONSTRAINT "account_invitation_accepted_by_user_id_fkey" FOREIGN KEY ("accepted_by") REFERENCES "user"("id");--> statement-breakpoint
ALTER TABLE "group" ADD CONSTRAINT "group_account_id_account_id_fkey" FOREIGN KEY ("account_id") REFERENCES "account"("id");--> statement-breakpoint
ALTER TABLE "group_membership" ADD CONSTRAINT "group_membership_account_id_group_id_group_account_id_id_fkey" FOREIGN KEY ("account_id","group_id") REFERENCES "group"("account_id","id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "group_membership" ADD CONSTRAINT "group_membership_duH96h7dnPNa_fkey" FOREIGN KEY ("account_id","account_membership_id") REFERENCES "account_membership"("account_id","id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "connected_account" ADD CONSTRAINT "connected_account_5ax8AnSpUIcc_fkey" FOREIGN KEY ("account_id","secret_space_id") REFERENCES "space_directory"("account_id","id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "connected_account" ADD CONSTRAINT "connected_account_account_id_account_id_fkey" FOREIGN KEY ("account_id") REFERENCES "account"("id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "connected_account" ADD CONSTRAINT "connected_account_user_id_user_id_fkey" FOREIGN KEY ("user_id") REFERENCES "user"("id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "role" ADD CONSTRAINT "role_account_id_account_id_fkey" FOREIGN KEY ("account_id") REFERENCES "account"("id");--> statement-breakpoint
ALTER TABLE "session" ADD CONSTRAINT "session_user_id_user_id_fkey" FOREIGN KEY ("user_id") REFERENCES "user"("id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "session" ADD CONSTRAINT "session_device_id_device_id_fkey" FOREIGN KEY ("device_id") REFERENCES "device"("id") ON DELETE RESTRICT;