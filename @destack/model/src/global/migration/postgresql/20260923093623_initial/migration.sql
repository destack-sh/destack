CREATE TABLE "user" (
	"id" text PRIMARY KEY,
	"created_at" bigint NOT NULL,
	"updated_at" bigint NOT NULL,
	"revision" bigint DEFAULT 1 NOT NULL,
	"tags" jsonb DEFAULT '{}' NOT NULL,
	"name" text NOT NULL,
	"email" text NOT NULL UNIQUE,
	"email_verified" boolean DEFAULT false NOT NULL,
	"image" text,
	"two_factor_enabled" boolean DEFAULT false NOT NULL,
	"suspended_at" bigint,
	"deletion_requested_at" bigint
);
--> statement-breakpoint
CREATE TABLE "organisation" (
	"id" text PRIMARY KEY,
	"created_at" bigint NOT NULL,
	"updated_at" bigint NOT NULL,
	"revision" bigint DEFAULT 1 NOT NULL,
	"tags" jsonb DEFAULT '{}' NOT NULL,
	"name" text NOT NULL,
	"image" text,
	"deletion_requested_at" bigint
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
CREATE TABLE "role" (
	"id" text PRIMARY KEY,
	"created_at" bigint NOT NULL,
	"updated_at" bigint NOT NULL,
	"revision" bigint DEFAULT 1 NOT NULL,
	"tags" jsonb DEFAULT '{}' NOT NULL,
	"provenance" jsonb,
	"detached_at" bigint,
	"account_id" text NOT NULL,
	"name" text NOT NULL,
	"description" text NOT NULL,
	CONSTRAINT "role_account_id" UNIQUE("account_id","id"),
	CONSTRAINT "role_provenance_account" CHECK ("provenance" IS NULL OR ("provenance"::jsonb ->> 'kind') <> 'account' OR ("provenance"::jsonb ->> 'accountId') = "account_id"),
	CONSTRAINT "role_provenance_detached" CHECK ("detached_at" IS NULL OR ("provenance" IS NOT NULL AND "detached_at" >= 0))
);
--> statement-breakpoint
CREATE TABLE "role_binding" (
	"id" text PRIMARY KEY,
	"created_at" bigint NOT NULL,
	"updated_at" bigint NOT NULL,
	"revision" bigint DEFAULT 1 NOT NULL,
	"tags" jsonb DEFAULT '{}' NOT NULL,
	"provenance" jsonb,
	"detached_at" bigint,
	"account_id" text NOT NULL,
	"role_id" text NOT NULL,
	"space_id" text,
	"account_membership_id" text,
	"group_id" text,
	"service_account_id" text,
	"expires_at" bigint,
	"revoked_at" bigint,
	CONSTRAINT "role_binding_provenance_account" CHECK ("provenance" IS NULL OR ("provenance"::jsonb ->> 'kind') <> 'account' OR ("provenance"::jsonb ->> 'accountId') = "account_id"),
	CONSTRAINT "role_binding_provenance_space" CHECK ("provenance" IS NULL OR ("provenance"::jsonb ->> 'kind') <> 'stack' OR ("provenance"::jsonb ->> 'spaceId') = "space_id"),
	CONSTRAINT "role_binding_provenance_detached" CHECK ("detached_at" IS NULL OR ("provenance" IS NOT NULL AND "detached_at" >= 0)),
	CONSTRAINT "role_binding_subject" CHECK (CAST("account_membership_id" IS NOT NULL AS integer) + CAST("group_id" IS NOT NULL AS integer) + CAST("service_account_id" IS NOT NULL AS integer) = 1),
	CONSTRAINT "role_binding_expiry" CHECK ("expires_at" IS NULL OR "expires_at" > "created_at")
);
--> statement-breakpoint
CREATE TABLE "role_permission" (
	"id" text PRIMARY KEY,
	"role_id" text NOT NULL,
	"package_id" text NOT NULL,
	"type" text NOT NULL,
	"name" text NOT NULL,
	"object_id" text,
	CONSTRAINT "role_permission_name_0" CHECK (("type" COLLATE "C") ~ '^[a-z][a-z0-9]*(-[a-z0-9]+)*$'),
	CONSTRAINT "role_permission_name_1" CHECK (("name" COLLATE "C") ~ '^[a-z][a-z0-9]*(-[a-z0-9]+)*$'),
	CONSTRAINT "role_permission_object" CHECK ("object_id" IS NULL OR length("object_id") > 0)
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
	"account_id" text NOT NULL,
	"service_account_id" text NOT NULL,
	"name" text NOT NULL,
	"token_hash" text NOT NULL UNIQUE,
	"expires_at" bigint NOT NULL,
	"revoked_at" bigint,
	CONSTRAINT "service_token_account_id" UNIQUE("account_id","id"),
	CONSTRAINT "service_token_expiry" CHECK ("expires_at" > "created_at")
);
--> statement-breakpoint
CREATE TABLE "service_token_permission" (
	"id" text PRIMARY KEY,
	"account_id" text NOT NULL,
	"token_id" text NOT NULL,
	"space_id" text,
	"package_id" text NOT NULL,
	"type" text NOT NULL,
	"name" text NOT NULL,
	"object_id" text,
	CONSTRAINT "service_token_permission_name_0" CHECK (("type" COLLATE "C") ~ '^[a-z][a-z0-9]*(-[a-z0-9]+)*$'),
	CONSTRAINT "service_token_permission_name_1" CHECK (("name" COLLATE "C") ~ '^[a-z][a-z0-9]*(-[a-z0-9]+)*$'),
	CONSTRAINT "service_token_permission_object" CHECK ("object_id" IS NULL OR length("object_id") > 0)
);
--> statement-breakpoint
CREATE TABLE "personal_access_token" (
	"id" text PRIMARY KEY,
	"created_at" bigint NOT NULL,
	"updated_at" bigint NOT NULL,
	"revision" bigint DEFAULT 1 NOT NULL,
	"tags" jsonb DEFAULT '{}' NOT NULL,
	"user_id" text NOT NULL,
	"account_id" text NOT NULL,
	"name" text NOT NULL,
	"token_hash" text NOT NULL UNIQUE,
	"expires_at" bigint NOT NULL,
	"revoked_at" bigint,
	CONSTRAINT "personal_access_token_account_id" UNIQUE("account_id","id"),
	CONSTRAINT "personal_access_token_user_id" UNIQUE("user_id","id"),
	CONSTRAINT "personal_access_token_expiry" CHECK ("expires_at" > "created_at")
);
--> statement-breakpoint
CREATE TABLE "organisation_membership" (
	"id" text PRIMARY KEY,
	"created_at" bigint NOT NULL,
	"updated_at" bigint NOT NULL,
	"revision" bigint DEFAULT 1 NOT NULL,
	"tags" jsonb DEFAULT '{}' NOT NULL,
	"organisation_id" text NOT NULL,
	"user_id" text NOT NULL,
	"role" text NOT NULL,
	CONSTRAINT "organisation_membership_user" UNIQUE("organisation_id","user_id"),
	CONSTRAINT "organisation_membership_role" CHECK ("role" IN ('owner', 'admin', 'member'))
);
--> statement-breakpoint
CREATE TABLE "personal_access_token_permission" (
	"id" text PRIMARY KEY,
	"account_id" text NOT NULL,
	"token_id" text NOT NULL,
	"space_id" text,
	"package_id" text NOT NULL,
	"type" text NOT NULL,
	"name" text NOT NULL,
	"object_id" text,
	CONSTRAINT "personal_access_token_permission_name_0" CHECK (("type" COLLATE "C") ~ '^[a-z][a-z0-9]*(-[a-z0-9]+)*$'),
	CONSTRAINT "personal_access_token_permission_name_1" CHECK (("name" COLLATE "C") ~ '^[a-z][a-z0-9]*(-[a-z0-9]+)*$'),
	CONSTRAINT "personal_access_token_permission_object" CHECK ("object_id" IS NULL OR length("object_id") > 0)
);
--> statement-breakpoint
CREATE TABLE "identity" (
	"id" text PRIMARY KEY,
	"created_at" bigint NOT NULL,
	"updated_at" bigint NOT NULL,
	"revision" bigint DEFAULT 1 NOT NULL,
	"tags" jsonb DEFAULT '{}' NOT NULL,
	"user_id" text NOT NULL,
	"provider_id" text NOT NULL,
	"provider_user_id" text NOT NULL,
	"access_token" text,
	"refresh_token" text,
	"id_token" text,
	"access_token_expires_at" bigint,
	"refresh_token_expires_at" bigint,
	"scope" text,
	"password" text,
	CONSTRAINT "identity_provider_user" UNIQUE("provider_id","provider_user_id")
);
--> statement-breakpoint
CREATE TABLE "session" (
	"id" text PRIMARY KEY,
	"user_id" text NOT NULL,
	"device_id" text,
	"token" text NOT NULL UNIQUE,
	"created_at" bigint NOT NULL,
	"updated_at" bigint NOT NULL,
	"ip_address" text,
	"user_agent" text,
	"country" text,
	"city" text,
	"authenticated_at" bigint,
	"authentication_method" text,
	"expires_at" bigint NOT NULL,
	"revoked_at" bigint,
	CONSTRAINT "session_expiry_order" CHECK ("expires_at" > "created_at"),
	CONSTRAINT "session_authentication_method" CHECK ("authentication_method" IS NULL OR "authentication_method" IN ('magic-link', 'email-otp', 'oauth', 'device', 'totp', 'webauthn', 'recovery')),
	CONSTRAINT "session_authentication_time" CHECK ("authentication_method" IS NULL OR "authenticated_at" IS NOT NULL)
);
--> statement-breakpoint
CREATE TABLE "passkey" (
	"id" text PRIMARY KEY,
	"name" text,
	"public_key" text NOT NULL,
	"user_id" text NOT NULL,
	"credential_id" text NOT NULL UNIQUE,
	"counter" bigint NOT NULL,
	"device_type" text NOT NULL,
	"backed_up" boolean NOT NULL,
	"transports" text,
	"created_at" bigint,
	"aaguid" text
);
--> statement-breakpoint
CREATE TABLE "two_factor" (
	"id" text PRIMARY KEY,
	"secret" text NOT NULL,
	"backup_codes" text NOT NULL,
	"user_id" text NOT NULL,
	"verified" boolean DEFAULT true,
	"failed_verification_count" bigint DEFAULT 0,
	"locked_until" bigint
);
--> statement-breakpoint
CREATE TABLE "device_authorization" (
	"id" text PRIMARY KEY,
	"device_code" text NOT NULL UNIQUE,
	"user_code" text NOT NULL UNIQUE,
	"user_id" text,
	"expires_at" bigint NOT NULL,
	"status" text NOT NULL,
	"last_polled_at" bigint,
	"polling_interval" bigint,
	"client_id" text,
	"oauth_client_id" text,
	"resources" jsonb,
	"scope" text,
	CONSTRAINT "device_authorization_status" CHECK ("status" IN ('pending', 'approved', 'denied'))
);
--> statement-breakpoint
CREATE TABLE "signing_key" (
	"id" text PRIMARY KEY,
	"public_key" text NOT NULL,
	"private_key" text NOT NULL,
	"created_at" bigint NOT NULL,
	"expires_at" bigint,
	"alg" text,
	"crv" text
);
--> statement-breakpoint
CREATE TABLE "authentication_replay" (
	"id" text PRIMARY KEY,
	"expires_at" bigint NOT NULL
);
--> statement-breakpoint
CREATE TABLE "oauth_client" (
	"id" text PRIMARY KEY,
	"client_id" text NOT NULL UNIQUE,
	"account_id" text,
	"service_account_id" text,
	"client_secret" text,
	"client_discovery_id" text,
	"subject_type" text,
	"name" text,
	"uri" text,
	"icon" text,
	"tos" text,
	"policy" text,
	"software_id" text,
	"software_version" text,
	"software_statement" text,
	"backchannel_logout_uri" text,
	"token_endpoint_auth_method" text,
	"application_type" text,
	"jwks" text,
	"jwks_uri" text,
	"reference_id" text,
	"disabled" boolean,
	"skip_consent" boolean,
	"enable_end_session" boolean,
	"backchannel_logout_session_required" boolean,
	"require_pkce" boolean,
	"dpop_bound_access_tokens" boolean,
	"scopes" jsonb,
	"client_credentials_scopes" jsonb,
	"contacts" jsonb,
	"redirect_uris" jsonb NOT NULL,
	"post_logout_redirect_uris" jsonb,
	"grant_types" jsonb,
	"response_types" jsonb,
	"user_id" text,
	"created_at" bigint,
	"updated_at" bigint,
	"metadata" jsonb,
	CONSTRAINT "oauth_client_service_account" CHECK ("service_account_id" IS NULL OR "account_id" IS NOT NULL)
);
--> statement-breakpoint
CREATE TABLE "oauth_resource" (
	"id" text PRIMARY KEY,
	"identifier" text NOT NULL UNIQUE,
	"name" text NOT NULL,
	"access_token_ttl" bigint,
	"refresh_token_ttl" bigint,
	"signing_algorithm" text,
	"signing_key_id" text,
	"allowed_scopes" jsonb,
	"custom_claims" jsonb,
	"dpop_bound_access_tokens_required" boolean DEFAULT false,
	"disabled" boolean DEFAULT false,
	"created_at" bigint,
	"updated_at" bigint,
	"policy_version" bigint DEFAULT 1,
	"metadata" jsonb
);
--> statement-breakpoint
CREATE TABLE "organisation_invitation" (
	"id" text PRIMARY KEY,
	"created_at" bigint NOT NULL,
	"updated_at" bigint NOT NULL,
	"revision" bigint DEFAULT 1 NOT NULL,
	"tags" jsonb DEFAULT '{}' NOT NULL,
	"organisation_id" text NOT NULL,
	"invited_by" text NOT NULL,
	"email" text NOT NULL,
	"role" text NOT NULL,
	"token_hash" text NOT NULL UNIQUE,
	"expires_at" bigint NOT NULL,
	"accepted_by" text,
	"accepted_at" bigint,
	"revoked_at" bigint,
	CONSTRAINT "organisation_invitation_role" CHECK ("role" IN ('owner', 'admin', 'member')),
	CONSTRAINT "organisation_invitation_expiry" CHECK ("expires_at" > "created_at"),
	CONSTRAINT "organisation_invitation_acceptance" CHECK (("accepted_by" IS NULL) = ("accepted_at" IS NULL) AND ("accepted_at" IS NULL OR ("revoked_at" IS NULL AND "accepted_at" >= "created_at" AND "accepted_at" < "expires_at")))
);
--> statement-breakpoint
CREATE TABLE "oauth_client_resource" (
	"id" text PRIMARY KEY,
	"client_id" text NOT NULL,
	"resource_id" text NOT NULL,
	"metadata" jsonb,
	"created_at" bigint,
	CONSTRAINT "oauth_client_resource_grant" UNIQUE("client_id","resource_id")
);
--> statement-breakpoint
CREATE TABLE "oauth_consent" (
	"id" text PRIMARY KEY,
	"client_id" text NOT NULL,
	"user_id" text,
	"reference_id" text,
	"resources" jsonb,
	"requested_user_info_claims" jsonb,
	"scopes" jsonb NOT NULL,
	"created_at" bigint NOT NULL,
	"updated_at" bigint NOT NULL
);
--> statement-breakpoint
CREATE TABLE "oauth_access_token" (
	"id" text PRIMARY KEY,
	"token" text NOT NULL UNIQUE,
	"client_id" text NOT NULL,
	"session_id" text,
	"user_id" text,
	"reference_id" text,
	"authorization_code_id" text,
	"resources" jsonb,
	"requested_user_info_claims" jsonb,
	"expires_at" bigint NOT NULL,
	"created_at" bigint NOT NULL,
	"revoked" bigint,
	"confirmation" jsonb,
	"scopes" jsonb NOT NULL,
	"refresh_id" text
);
--> statement-breakpoint
CREATE TABLE "oauth_refresh_token" (
	"id" text PRIMARY KEY,
	"token" text NOT NULL UNIQUE,
	"client_id" text NOT NULL,
	"session_id" text,
	"user_id" text NOT NULL,
	"reference_id" text,
	"authorization_code_id" text,
	"resources" jsonb,
	"requested_user_info_claims" jsonb,
	"expires_at" bigint NOT NULL,
	"created_at" bigint NOT NULL,
	"revoked" bigint,
	"confirmation" jsonb,
	"scopes" jsonb NOT NULL,
	"rotated_at" bigint,
	"rotation_replay_response" text,
	"rotation_replay_expires_at" bigint,
	"auth_time" bigint
);
--> statement-breakpoint
CREATE TABLE "oauth_client_assertion" (
	"id" text PRIMARY KEY,
	"expires_at" bigint NOT NULL
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
	"enrolled_by" text,
	"category" text DEFAULT 'unknown' NOT NULL,
	"operating_system" text,
	"operating_system_version" text,
	"architecture" text,
	"model" text,
	"client_version" text,
	"revoked_at" bigint,
	"last_seen_at" bigint,
	CONSTRAINT "device_account_id" UNIQUE("account_id","id"),
	CONSTRAINT "device_category" CHECK ("category" IN ('desktop', 'laptop', 'phone', 'tablet', 'server', 'unknown'))
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
	"thumbprint" text NOT NULL UNIQUE,
	"verified_at" bigint NOT NULL,
	"expires_at" bigint NOT NULL,
	"revoked_at" bigint,
	CONSTRAINT "device_key_device_id" UNIQUE("device_id","id"),
	CONSTRAINT "device_key_expiry" CHECK ("expires_at" > "created_at"),
	CONSTRAINT "device_key_verification" CHECK ("verified_at" >= "created_at" AND "verified_at" < "expires_at")
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
	"provider" text,
	"location" text,
	"status" text DEFAULT 'enabled' NOT NULL,
	"version" text,
	"runtimes" jsonb DEFAULT '[]' NOT NULL,
	"last_seen_at" bigint,
	CONSTRAINT "host_device_id" UNIQUE("device_id","id"),
	CONSTRAINT "host_account_id" UNIQUE("account_id","id"),
	CONSTRAINT "host_status" CHECK ("status" IN ('enabled', 'draining', 'disabled')),
	CONSTRAINT "host_location" CHECK ("location" IS NULL OR ("provider" IS NOT NULL AND length("location") > 0)),
	CONSTRAINT "host_kind" CHECK (("kind" = 'device' AND "device_id" IS NOT NULL) OR ("kind" = 'cloud' AND "device_id" IS NULL))
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
	"code" text NOT NULL CONSTRAINT "region_code" UNIQUE,
	"name" text NOT NULL,
	"residency" text NOT NULL,
	CONSTRAINT "region_residency_id" UNIQUE("id","residency"),
	CONSTRAINT "region_residency" CHECK ("residency" IN ('eu', 'us'))
);
--> statement-breakpoint
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
	"user_id" text,
	"organisation_id" text,
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
	CONSTRAINT "account_user" CHECK (("kind" = 'personal' AND "user_id" IS NOT NULL AND "organisation_id" IS NULL) OR ("kind" = 'organisation' AND "user_id" IS NULL AND "organisation_id" IS NOT NULL)),
	CONSTRAINT "account_handle" CHECK (length("handle") BETWEEN 1 AND 63 AND ("handle" COLLATE "C") !~ '[^a-z0-9-]' AND "handle" NOT LIKE '-%' AND "handle" NOT LIKE '%-')
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
CREATE TABLE "space" (
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
	"authority_host_id" text,
	"authority_epoch" bigint NOT NULL,
	"account_id" text NOT NULL,
	"name" text NOT NULL,
	"environment_id" text,
	CONSTRAINT "space_name" UNIQUE("account_id","name"),
	CONSTRAINT "space_account" UNIQUE("account_id","id"),
	CONSTRAINT "space_revision" CHECK ("revision" >= 1),
	CONSTRAINT "space_generation" CHECK ("generation" >= 1),
	CONSTRAINT "space_observed_generation" CHECK ("observed_generation" BETWEEN 0 AND "generation"),
	CONSTRAINT "space_authority_epoch" CHECK ("authority_epoch" > 0),
	CONSTRAINT "space_name_value" CHECK (length("name") BETWEEN 1 AND 63 AND ("name" COLLATE "C") !~ '[^a-z0-9-]' AND "name" NOT LIKE '-%' AND "name" NOT LIKE '%-')
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
	"residency" text NOT NULL,
	"region_id" text NOT NULL,
	"account_id" text NOT NULL,
	"name" text NOT NULL,
	CONSTRAINT "repository_name" UNIQUE("account_id","name"),
	CONSTRAINT "repository_account" UNIQUE("account_id","id"),
	CONSTRAINT "repository_revision" CHECK ("revision" >= 1),
	CONSTRAINT "repository_generation" CHECK ("generation" >= 1),
	CONSTRAINT "repository_observed_generation" CHECK ("observed_generation" BETWEEN 0 AND "generation"),
	CONSTRAINT "repository_name_value" CHECK (length("name") BETWEEN 1 AND 63 AND ("name" COLLATE "C") !~ '[^a-z0-9-]' AND "name" NOT LIKE '-%' AND "name" NOT LIKE '%-')
);
--> statement-breakpoint
CREATE TABLE "package" (
	"id" text PRIMARY KEY,
	"created_at" bigint NOT NULL,
	"updated_at" bigint NOT NULL,
	"revision" bigint DEFAULT 1 NOT NULL,
	"tags" jsonb DEFAULT '{}' NOT NULL,
	"account_id" text NOT NULL,
	"name" text NOT NULL,
	"repository_id" text NOT NULL,
	CONSTRAINT "package_name" UNIQUE("account_id","name")
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
CREATE TABLE "route" (
	"id" text PRIMARY KEY,
	"created_at" bigint NOT NULL,
	"updated_at" bigint NOT NULL,
	"revision" bigint DEFAULT 1 NOT NULL,
	"tags" jsonb DEFAULT '{}' NOT NULL,
	"provenance" jsonb,
	"detached_at" bigint,
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
	CONSTRAINT "route_provenance_account" CHECK ("provenance" IS NULL OR ("provenance"::jsonb ->> 'kind') <> 'account' OR ("provenance"::jsonb ->> 'accountId') = "account_id"),
	CONSTRAINT "route_provenance_space" CHECK ("provenance" IS NULL OR ("provenance"::jsonb ->> 'kind') <> 'stack' OR ("provenance"::jsonb ->> 'spaceId') = "space_id"),
	CONSTRAINT "route_provenance_detached" CHECK ("detached_at" IS NULL OR ("provenance" IS NOT NULL AND "detached_at" >= 0)),
	CONSTRAINT "route_revision" CHECK ("revision" >= 1),
	CONSTRAINT "route_generation" CHECK ("generation" >= 1),
	CONSTRAINT "route_observed_generation" CHECK ("observed_generation" BETWEEN 0 AND "generation"),
	CONSTRAINT "route_match" CHECK ("match" IN ('exact', 'prefix')),
	CONSTRAINT "route_path" CHECK (substr("path", 1, 1) = '/'),
	CONSTRAINT "route_applied_generation" CHECK ("applied_generation" BETWEEN 0 AND "generation"),
	CONSTRAINT "route_destination" CHECK (("kind" = 'application' AND "space_id" IS NOT NULL AND "installation_id" IS NOT NULL AND "entrypoint" IS NOT NULL AND "redirect" IS NULL AND "redirect_status" IS NULL) OR ("kind" = 'redirect' AND "space_id" IS NULL AND "installation_id" IS NULL AND "entrypoint" IS NULL AND "redirect" IS NOT NULL AND "redirect_status" IN (301, 302, 303, 307, 308) AND "redirect_status" IS NOT NULL))
);
--> statement-breakpoint
CREATE TABLE "environment" (
	"id" text PRIMARY KEY,
	"created_at" bigint NOT NULL,
	"updated_at" bigint NOT NULL,
	"revision" bigint DEFAULT 1 NOT NULL,
	"tags" jsonb DEFAULT '{}' NOT NULL,
	"provenance" jsonb,
	"detached_at" bigint,
	"account_id" text NOT NULL,
	"name" text NOT NULL,
	CONSTRAINT "environment_account_name" UNIQUE("account_id","name"),
	CONSTRAINT "environment_account_id" UNIQUE("account_id","id"),
	CONSTRAINT "environment_name" CHECK (("name" COLLATE "C") ~ '^[a-z][a-z0-9-]*$' AND ("name" COLLATE "C") !~ '[^a-z0-9-]'),
	CONSTRAINT "environment_provenance_account" CHECK ("provenance" IS NULL OR ("provenance"::jsonb ->> 'kind') <> 'account' OR ("provenance"::jsonb ->> 'accountId') = "account_id"),
	CONSTRAINT "environment_provenance_detached" CHECK ("detached_at" IS NULL OR ("provenance" IS NOT NULL AND "detached_at" >= 0))
);
--> statement-breakpoint
CREATE TABLE "account_source" (
	"account_id" text PRIMARY KEY,
	"repository_id" text NOT NULL,
	"reference" text NOT NULL,
	"directory" text NOT NULL,
	"entrypoint" text NOT NULL,
	"export" text NOT NULL,
	"parameters" jsonb NOT NULL,
	"generation" bigint DEFAULT 1 NOT NULL,
	"applied_revision_id" text,
	CONSTRAINT "account_source_generation" CHECK ("generation" > 0),
	CONSTRAINT "account_source_directory" CHECK (length("directory") > 0 AND substr("directory", 1, 1) <> '/' AND "directory" <> '..' AND "directory" NOT LIKE '../%' AND "directory" NOT LIKE '%/../%' AND "directory" NOT LIKE '%/..'),
	CONSTRAINT "account_source_export" CHECK (length("export") > 0 AND ("entrypoint" = '.' OR "entrypoint" LIKE './_%')),
	CONSTRAINT "account_source_reference" CHECK ("reference" LIKE 'refs/heads/_%' OR "reference" LIKE 'refs/tags/_%')
);
--> statement-breakpoint
CREATE TABLE "account_revision" (
	"id" text PRIMARY KEY,
	"account_id" text NOT NULL,
	"source_generation" bigint NOT NULL,
	"source" jsonb NOT NULL,
	"parameters" jsonb NOT NULL,
	"definition" jsonb NOT NULL,
	"digest" text NOT NULL,
	"created_at" bigint NOT NULL,
	CONSTRAINT "account_revision_account_id" UNIQUE("account_id","id"),
	CONSTRAINT "account_revision_generation" CHECK ("source_generation" > 0),
	CONSTRAINT "account_revision_digest" CHECK (("digest" COLLATE "C") ~ '^[a-f0-9]{64}$')
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
CREATE UNIQUE INDEX "role_provenance" ON "role" (("provenance"::jsonb ->> 'kind'),coalesce(("provenance"::jsonb ->> 'accountId'), ("provenance"::jsonb ->> 'spaceId'), ("provenance"::jsonb ->> 'installationId')),("provenance"::jsonb ->> 'name')) WHERE "provenance" IS NOT NULL AND "detached_at" IS NULL;--> statement-breakpoint
CREATE UNIQUE INDEX "role_scope_name" ON "role" ("account_id","name");--> statement-breakpoint
CREATE UNIQUE INDEX "role_binding_provenance" ON "role_binding" (("provenance"::jsonb ->> 'kind'),coalesce(("provenance"::jsonb ->> 'accountId'), ("provenance"::jsonb ->> 'spaceId'), ("provenance"::jsonb ->> 'installationId')),("provenance"::jsonb ->> 'name')) WHERE "provenance" IS NOT NULL AND "detached_at" IS NULL;--> statement-breakpoint
CREATE UNIQUE INDEX "role_binding_active" ON "role_binding" ("role_id",coalesce("space_id", ''),coalesce("account_membership_id", "group_id", "service_account_id")) WHERE "revoked_at" IS NULL;--> statement-breakpoint
CREATE UNIQUE INDEX "role_permission_scope" ON "role_permission" ("role_id","package_id","type","name",coalesce("object_id", ''));--> statement-breakpoint
CREATE UNIQUE INDEX "connection_oauth" ON "connected_account" ("account_id","provider","issuer","application_id","subject") WHERE "kind" = 'oauth';--> statement-breakpoint
CREATE UNIQUE INDEX "connection_installation" ON "connected_account" ("account_id","provider","issuer","application_id","installation_id") WHERE "kind" = 'installation';--> statement-breakpoint
CREATE INDEX "connection_user" ON "connected_account" ("user_id");--> statement-breakpoint
CREATE UNIQUE INDEX "service_token_permission_scope" ON "service_token_permission" ("token_id",coalesce("space_id", ''),"package_id","type","name",coalesce("object_id", ''));--> statement-breakpoint
CREATE INDEX "personal_access_token_user" ON "personal_access_token" ("user_id");--> statement-breakpoint
CREATE INDEX "organisation_membership_user_id" ON "organisation_membership" ("user_id");--> statement-breakpoint
CREATE UNIQUE INDEX "personal_access_token_permission_scope" ON "personal_access_token_permission" ("token_id",coalesce("space_id", ''),"package_id","type","name",coalesce("object_id", ''));--> statement-breakpoint
CREATE INDEX "identity_user" ON "identity" ("user_id");--> statement-breakpoint
CREATE INDEX "session_user" ON "session" ("user_id");--> statement-breakpoint
CREATE INDEX "session_device" ON "session" ("device_id");--> statement-breakpoint
CREATE INDEX "session_expiry" ON "session" ("expires_at");--> statement-breakpoint
CREATE INDEX "passkey_user" ON "passkey" ("user_id");--> statement-breakpoint
CREATE INDEX "two_factor_user" ON "two_factor" ("user_id");--> statement-breakpoint
CREATE INDEX "two_factor_secret" ON "two_factor" ("secret");--> statement-breakpoint
CREATE INDEX "device_authorization_expiry" ON "device_authorization" ("expires_at");--> statement-breakpoint
CREATE INDEX "device_authorization_user" ON "device_authorization" ("user_id");--> statement-breakpoint
CREATE INDEX "signing_key_created" ON "signing_key" ("created_at");--> statement-breakpoint
CREATE INDEX "authentication_replay_expiry" ON "authentication_replay" ("expires_at");--> statement-breakpoint
CREATE INDEX "oauth_client_user" ON "oauth_client" ("user_id");--> statement-breakpoint
CREATE INDEX "oauth_client_account" ON "oauth_client" ("account_id");--> statement-breakpoint
CREATE INDEX "organisation_invitation_organisation" ON "organisation_invitation" ("organisation_id");--> statement-breakpoint
CREATE INDEX "oauth_client_resource_resource" ON "oauth_client_resource" ("resource_id");--> statement-breakpoint
CREATE INDEX "oauth_consent_client" ON "oauth_consent" ("client_id");--> statement-breakpoint
CREATE INDEX "oauth_consent_user" ON "oauth_consent" ("user_id");--> statement-breakpoint
CREATE INDEX "oauth_access_client" ON "oauth_access_token" ("client_id");--> statement-breakpoint
CREATE INDEX "oauth_access_user" ON "oauth_access_token" ("user_id");--> statement-breakpoint
CREATE INDEX "oauth_access_session" ON "oauth_access_token" ("session_id");--> statement-breakpoint
CREATE INDEX "oauth_access_refresh" ON "oauth_access_token" ("refresh_id");--> statement-breakpoint
CREATE INDEX "oauth_access_code" ON "oauth_access_token" ("authorization_code_id");--> statement-breakpoint
CREATE INDEX "oauth_access_expiry" ON "oauth_access_token" ("expires_at");--> statement-breakpoint
CREATE INDEX "oauth_refresh_client" ON "oauth_refresh_token" ("client_id");--> statement-breakpoint
CREATE INDEX "oauth_refresh_user" ON "oauth_refresh_token" ("user_id");--> statement-breakpoint
CREATE INDEX "oauth_refresh_session" ON "oauth_refresh_token" ("session_id");--> statement-breakpoint
CREATE INDEX "oauth_refresh_code" ON "oauth_refresh_token" ("authorization_code_id");--> statement-breakpoint
CREATE INDEX "oauth_refresh_expiry" ON "oauth_refresh_token" ("expires_at");--> statement-breakpoint
CREATE INDEX "oauth_client_assertion_expiry" ON "oauth_client_assertion" ("expires_at");--> statement-breakpoint
CREATE INDEX "device_enrolled_by" ON "device" ("enrolled_by");--> statement-breakpoint
CREATE INDEX "account_user" ON "account" ("user_id");--> statement-breakpoint
CREATE INDEX "account_organisation" ON "account" ("organisation_id");--> statement-breakpoint
CREATE INDEX "tunnel_host_expiry" ON "tunnel" ("host_id","expires_at");--> statement-breakpoint
CREATE UNIQUE INDEX "route_provenance" ON "route" (("provenance"::jsonb ->> 'kind'),coalesce(("provenance"::jsonb ->> 'accountId'), ("provenance"::jsonb ->> 'spaceId'), ("provenance"::jsonb ->> 'installationId')),("provenance"::jsonb ->> 'name')) WHERE "provenance" IS NOT NULL AND "detached_at" IS NULL;--> statement-breakpoint
CREATE UNIQUE INDEX "environment_provenance" ON "environment" (("provenance"::jsonb ->> 'kind'),coalesce(("provenance"::jsonb ->> 'accountId'), ("provenance"::jsonb ->> 'spaceId'), ("provenance"::jsonb ->> 'installationId')),("provenance"::jsonb ->> 'name')) WHERE "provenance" IS NOT NULL AND "detached_at" IS NULL;--> statement-breakpoint
CREATE INDEX "account_membership_user" ON "account_membership" ("user_id");--> statement-breakpoint
ALTER TABLE "group" ADD CONSTRAINT "group_account_id_account_id_fkey" FOREIGN KEY ("account_id") REFERENCES "account"("id");--> statement-breakpoint
ALTER TABLE "group_membership" ADD CONSTRAINT "group_membership_account_id_group_id_group_account_id_id_fkey" FOREIGN KEY ("account_id","group_id") REFERENCES "group"("account_id","id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "group_membership" ADD CONSTRAINT "group_membership_duH96h7dnPNa_fkey" FOREIGN KEY ("account_id","account_membership_id") REFERENCES "account_membership"("account_id","id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "role" ADD CONSTRAINT "role_account_id_account_id_fkey" FOREIGN KEY ("account_id") REFERENCES "account"("id");--> statement-breakpoint
ALTER TABLE "role_binding" ADD CONSTRAINT "role_binding_account_id_space_id_space_account_id_id_fkey" FOREIGN KEY ("account_id","space_id") REFERENCES "space"("account_id","id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "role_binding" ADD CONSTRAINT "role_binding_account_id_role_id_role_account_id_id_fkey" FOREIGN KEY ("account_id","role_id") REFERENCES "role"("account_id","id");--> statement-breakpoint
ALTER TABLE "role_binding" ADD CONSTRAINT "role_binding_fXOt6GFY2BtO_fkey" FOREIGN KEY ("account_id","account_membership_id") REFERENCES "account_membership"("account_id","id");--> statement-breakpoint
ALTER TABLE "role_binding" ADD CONSTRAINT "role_binding_account_id_group_id_group_account_id_id_fkey" FOREIGN KEY ("account_id","group_id") REFERENCES "group"("account_id","id");--> statement-breakpoint
ALTER TABLE "role_binding" ADD CONSTRAINT "role_binding_AMsSsGJGSTWe_fkey" FOREIGN KEY ("account_id","service_account_id") REFERENCES "service_account"("account_id","id");--> statement-breakpoint
ALTER TABLE "role_permission" ADD CONSTRAINT "role_permission_role_id_role_id_fkey" FOREIGN KEY ("role_id") REFERENCES "role"("id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "connected_account" ADD CONSTRAINT "connected_account_aWTilmxzzrwo_fkey" FOREIGN KEY ("account_id","secret_space_id") REFERENCES "space"("account_id","id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "connected_account" ADD CONSTRAINT "connected_account_account_id_account_id_fkey" FOREIGN KEY ("account_id") REFERENCES "account"("id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "connected_account" ADD CONSTRAINT "connected_account_user_id_user_id_fkey" FOREIGN KEY ("user_id") REFERENCES "user"("id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "service_account" ADD CONSTRAINT "service_account_account_id_account_id_fkey" FOREIGN KEY ("account_id") REFERENCES "account"("id");--> statement-breakpoint
ALTER TABLE "service_token" ADD CONSTRAINT "service_token_tUZENUY6Hqd9_fkey" FOREIGN KEY ("account_id","service_account_id") REFERENCES "service_account"("account_id","id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "service_token_permission" ADD CONSTRAINT "service_token_permission_fVsSu6h7iizZ_fkey" FOREIGN KEY ("account_id","token_id") REFERENCES "service_token"("account_id","id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "service_token_permission" ADD CONSTRAINT "service_token_permission_SGJOQiXZASWx_fkey" FOREIGN KEY ("account_id","space_id") REFERENCES "space"("account_id","id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "personal_access_token" ADD CONSTRAINT "personal_access_token_user_id_user_id_fkey" FOREIGN KEY ("user_id") REFERENCES "user"("id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "personal_access_token" ADD CONSTRAINT "personal_access_token_account_id_account_id_fkey" FOREIGN KEY ("account_id") REFERENCES "account"("id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "organisation_membership" ADD CONSTRAINT "organisation_membership_organisation_id_organisation_id_fkey" FOREIGN KEY ("organisation_id") REFERENCES "organisation"("id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "organisation_membership" ADD CONSTRAINT "organisation_membership_user_id_user_id_fkey" FOREIGN KEY ("user_id") REFERENCES "user"("id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "personal_access_token_permission" ADD CONSTRAINT "personal_access_token_permission_p8NC8du9W2Yf_fkey" FOREIGN KEY ("account_id","token_id") REFERENCES "personal_access_token"("account_id","id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "personal_access_token_permission" ADD CONSTRAINT "personal_access_token_permission_g8ZDPNlHxYEt_fkey" FOREIGN KEY ("account_id","space_id") REFERENCES "space"("account_id","id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "identity" ADD CONSTRAINT "identity_user_id_user_id_fkey" FOREIGN KEY ("user_id") REFERENCES "user"("id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "session" ADD CONSTRAINT "session_user_id_user_id_fkey" FOREIGN KEY ("user_id") REFERENCES "user"("id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "session" ADD CONSTRAINT "session_device_id_device_id_fkey" FOREIGN KEY ("device_id") REFERENCES "device"("id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "passkey" ADD CONSTRAINT "passkey_user_id_user_id_fkey" FOREIGN KEY ("user_id") REFERENCES "user"("id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "two_factor" ADD CONSTRAINT "two_factor_user_id_user_id_fkey" FOREIGN KEY ("user_id") REFERENCES "user"("id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "device_authorization" ADD CONSTRAINT "device_authorization_user_id_user_id_fkey" FOREIGN KEY ("user_id") REFERENCES "user"("id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "device_authorization" ADD CONSTRAINT "device_authorization_7zeD1sTp8yDW_fkey" FOREIGN KEY ("oauth_client_id") REFERENCES "oauth_client"("client_id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "oauth_client" ADD CONSTRAINT "oauth_client_D61d6nC9GuYl_fkey" FOREIGN KEY ("account_id","service_account_id") REFERENCES "service_account"("account_id","id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "oauth_client" ADD CONSTRAINT "oauth_client_account_id_account_id_fkey" FOREIGN KEY ("account_id") REFERENCES "account"("id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "oauth_client" ADD CONSTRAINT "oauth_client_user_id_user_id_fkey" FOREIGN KEY ("user_id") REFERENCES "user"("id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "organisation_invitation" ADD CONSTRAINT "organisation_invitation_organisation_id_organisation_id_fkey" FOREIGN KEY ("organisation_id") REFERENCES "organisation"("id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "organisation_invitation" ADD CONSTRAINT "organisation_invitation_invited_by_user_id_fkey" FOREIGN KEY ("invited_by") REFERENCES "user"("id");--> statement-breakpoint
ALTER TABLE "organisation_invitation" ADD CONSTRAINT "organisation_invitation_accepted_by_user_id_fkey" FOREIGN KEY ("accepted_by") REFERENCES "user"("id");--> statement-breakpoint
ALTER TABLE "oauth_client_resource" ADD CONSTRAINT "oauth_client_resource_client_id_oauth_client_client_id_fkey" FOREIGN KEY ("client_id") REFERENCES "oauth_client"("client_id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "oauth_client_resource" ADD CONSTRAINT "oauth_client_resource_dn2L1gs9Dolm_fkey" FOREIGN KEY ("resource_id") REFERENCES "oauth_resource"("identifier") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "oauth_consent" ADD CONSTRAINT "oauth_consent_client_id_oauth_client_client_id_fkey" FOREIGN KEY ("client_id") REFERENCES "oauth_client"("client_id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "oauth_consent" ADD CONSTRAINT "oauth_consent_user_id_user_id_fkey" FOREIGN KEY ("user_id") REFERENCES "user"("id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "oauth_access_token" ADD CONSTRAINT "oauth_access_token_client_id_oauth_client_client_id_fkey" FOREIGN KEY ("client_id") REFERENCES "oauth_client"("client_id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "oauth_access_token" ADD CONSTRAINT "oauth_access_token_session_id_session_id_fkey" FOREIGN KEY ("session_id") REFERENCES "session"("id") ON DELETE SET NULL;--> statement-breakpoint
ALTER TABLE "oauth_access_token" ADD CONSTRAINT "oauth_access_token_user_id_user_id_fkey" FOREIGN KEY ("user_id") REFERENCES "user"("id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "oauth_access_token" ADD CONSTRAINT "oauth_access_token_refresh_id_oauth_refresh_token_id_fkey" FOREIGN KEY ("refresh_id") REFERENCES "oauth_refresh_token"("id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "oauth_refresh_token" ADD CONSTRAINT "oauth_refresh_token_client_id_oauth_client_client_id_fkey" FOREIGN KEY ("client_id") REFERENCES "oauth_client"("client_id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "oauth_refresh_token" ADD CONSTRAINT "oauth_refresh_token_session_id_session_id_fkey" FOREIGN KEY ("session_id") REFERENCES "session"("id") ON DELETE SET NULL;--> statement-breakpoint
ALTER TABLE "oauth_refresh_token" ADD CONSTRAINT "oauth_refresh_token_user_id_user_id_fkey" FOREIGN KEY ("user_id") REFERENCES "user"("id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "device" ADD CONSTRAINT "device_account_id_account_id_fkey" FOREIGN KEY ("account_id") REFERENCES "account"("id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "device" ADD CONSTRAINT "device_enrolled_by_user_id_fkey" FOREIGN KEY ("enrolled_by") REFERENCES "user"("id") ON DELETE SET NULL;--> statement-breakpoint
ALTER TABLE "device_key" ADD CONSTRAINT "device_key_device_id_device_id_fkey" FOREIGN KEY ("device_id") REFERENCES "device"("id");--> statement-breakpoint
ALTER TABLE "host" ADD CONSTRAINT "host_account_id_device_id_device_account_id_id_fkey" FOREIGN KEY ("account_id","device_id") REFERENCES "device"("account_id","id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "host" ADD CONSTRAINT "host_account_id_account_id_fkey" FOREIGN KEY ("account_id") REFERENCES "account"("id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "host_access" ADD CONSTRAINT "host_access_host_id_host_id_fkey" FOREIGN KEY ("host_id") REFERENCES "host"("id");--> statement-breakpoint
ALTER TABLE "host_access" ADD CONSTRAINT "host_access_account_id_account_id_fkey" FOREIGN KEY ("account_id") REFERENCES "account"("id");--> statement-breakpoint
ALTER TABLE "account" ADD CONSTRAINT "account_package_policy_region_id_region_id_fkey" FOREIGN KEY ("package_policy_region_id") REFERENCES "region"("id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "account" ADD CONSTRAINT "account_network_policy_region_id_region_id_fkey" FOREIGN KEY ("network_policy_region_id") REFERENCES "region"("id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "account" ADD CONSTRAINT "account_user_id_user_id_fkey" FOREIGN KEY ("user_id") REFERENCES "user"("id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "account" ADD CONSTRAINT "account_organisation_id_organisation_id_fkey" FOREIGN KEY ("organisation_id") REFERENCES "organisation"("id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "tunnel" ADD CONSTRAINT "tunnel_device_id_host_id_host_device_id_id_fkey" FOREIGN KEY ("device_id","host_id") REFERENCES "host"("device_id","id");--> statement-breakpoint
ALTER TABLE "tunnel" ADD CONSTRAINT "tunnel_device_id_device_key_id_device_key_device_id_id_fkey" FOREIGN KEY ("device_id","device_key_id") REFERENCES "device_key"("device_id","id");--> statement-breakpoint
ALTER TABLE "tunnel" ADD CONSTRAINT "tunnel_host_id_host_id_fkey" FOREIGN KEY ("host_id") REFERENCES "host"("id");--> statement-breakpoint
ALTER TABLE "tunnel" ADD CONSTRAINT "tunnel_device_id_device_id_fkey" FOREIGN KEY ("device_id") REFERENCES "device"("id");--> statement-breakpoint
ALTER TABLE "space" ADD CONSTRAINT "space_account_id_environment_id_environment_account_id_id_fkey" FOREIGN KEY ("account_id","environment_id") REFERENCES "environment"("account_id","id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "space" ADD CONSTRAINT "space_account_id_authority_host_id_host_account_id_id_fkey" FOREIGN KEY ("account_id","authority_host_id") REFERENCES "host"("account_id","id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "space" ADD CONSTRAINT "space_region_id_residency_region_id_residency_fkey" FOREIGN KEY ("region_id","residency") REFERENCES "region"("id","residency") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "space" ADD CONSTRAINT "space_account_id_account_id_fkey" FOREIGN KEY ("account_id") REFERENCES "account"("id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "repository" ADD CONSTRAINT "repository_region_id_residency_region_id_residency_fkey" FOREIGN KEY ("region_id","residency") REFERENCES "region"("id","residency") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "repository" ADD CONSTRAINT "repository_account_id_account_id_fkey" FOREIGN KEY ("account_id") REFERENCES "account"("id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "package" ADD CONSTRAINT "package_account_id_repository_id_repository_account_id_id_fkey" FOREIGN KEY ("account_id","repository_id") REFERENCES "repository"("account_id","id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "domain" ADD CONSTRAINT "domain_account_id_account_id_fkey" FOREIGN KEY ("account_id") REFERENCES "account"("id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "route" ADD CONSTRAINT "route_account_id_domain_id_domain_account_id_id_fkey" FOREIGN KEY ("account_id","domain_id") REFERENCES "domain"("account_id","id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "route" ADD CONSTRAINT "route_account_id_account_id_fkey" FOREIGN KEY ("account_id") REFERENCES "account"("id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "route" ADD CONSTRAINT "route_space_id_space_id_fkey" FOREIGN KEY ("space_id") REFERENCES "space"("id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "environment" ADD CONSTRAINT "environment_account_id_account_id_fkey" FOREIGN KEY ("account_id") REFERENCES "account"("id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "account_source" ADD CONSTRAINT "account_source_9GRh1XzITk3v_fkey" FOREIGN KEY ("account_id","repository_id") REFERENCES "repository"("account_id","id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "account_source" ADD CONSTRAINT "account_source_LvwBrf7PesfH_fkey" FOREIGN KEY ("account_id","applied_revision_id") REFERENCES "account_revision"("account_id","id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "account_source" ADD CONSTRAINT "account_source_account_id_account_id_fkey" FOREIGN KEY ("account_id") REFERENCES "account"("id");--> statement-breakpoint
ALTER TABLE "account_revision" ADD CONSTRAINT "account_revision_account_id_account_id_fkey" FOREIGN KEY ("account_id") REFERENCES "account"("id");--> statement-breakpoint
ALTER TABLE "account_membership" ADD CONSTRAINT "account_membership_account_id_account_id_fkey" FOREIGN KEY ("account_id") REFERENCES "account"("id") ON DELETE CASCADE;--> statement-breakpoint
ALTER TABLE "account_membership" ADD CONSTRAINT "account_membership_user_id_user_id_fkey" FOREIGN KEY ("user_id") REFERENCES "user"("id") ON DELETE RESTRICT;--> statement-breakpoint
ALTER TABLE "account_invitation" ADD CONSTRAINT "account_invitation_account_id_account_id_fkey" FOREIGN KEY ("account_id") REFERENCES "account"("id");--> statement-breakpoint
ALTER TABLE "account_invitation" ADD CONSTRAINT "account_invitation_invited_by_user_id_fkey" FOREIGN KEY ("invited_by") REFERENCES "user"("id");--> statement-breakpoint
ALTER TABLE "account_invitation" ADD CONSTRAINT "account_invitation_accepted_by_user_id_fkey" FOREIGN KEY ("accepted_by") REFERENCES "user"("id");