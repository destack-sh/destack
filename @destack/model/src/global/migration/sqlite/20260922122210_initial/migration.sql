CREATE TABLE `user` (
	`id` text PRIMARY KEY,
	`created_at` integer NOT NULL,
	`updated_at` integer NOT NULL,
	`revision` integer DEFAULT 1 NOT NULL,
	`tags` text DEFAULT '{}' NOT NULL,
	`name` text NOT NULL,
	`email` text NOT NULL UNIQUE,
	`email_verified` integer DEFAULT false NOT NULL,
	`image` text,
	`two_factor_enabled` integer DEFAULT false NOT NULL,
	`suspended_at` integer,
	`deletion_requested_at` integer,
	CONSTRAINT "user_id_not_null" CHECK("id" IS NOT NULL)
);
--> statement-breakpoint
CREATE TABLE `organisation` (
	`id` text PRIMARY KEY,
	`created_at` integer NOT NULL,
	`updated_at` integer NOT NULL,
	`revision` integer DEFAULT 1 NOT NULL,
	`tags` text DEFAULT '{}' NOT NULL,
	`name` text NOT NULL,
	`image` text,
	`deletion_requested_at` integer,
	CONSTRAINT "organisation_id_not_null" CHECK("id" IS NOT NULL)
);
--> statement-breakpoint
CREATE TABLE `group` (
	`id` text PRIMARY KEY,
	`created_at` integer NOT NULL,
	`updated_at` integer NOT NULL,
	`revision` integer DEFAULT 1 NOT NULL,
	`tags` text DEFAULT '{}' NOT NULL,
	`account_id` text NOT NULL,
	`name` text NOT NULL,
	CONSTRAINT `fk_group_account_id_account_id_fk` FOREIGN KEY (`account_id`) REFERENCES `account`(`id`),
	CONSTRAINT `group_account_name` UNIQUE(`account_id`,`name`),
	CONSTRAINT `group_account_id` UNIQUE(`account_id`,`id`),
	CONSTRAINT "group_id_not_null" CHECK("id" IS NOT NULL)
);
--> statement-breakpoint
CREATE TABLE `group_membership` (
	`account_id` text NOT NULL,
	`group_id` text NOT NULL,
	`account_membership_id` text NOT NULL,
	CONSTRAINT `group_membership_pk` PRIMARY KEY(`group_id`, `account_membership_id`),
	CONSTRAINT `fk_group_membership_account_id_group_id_group_account_id_id_fk` FOREIGN KEY (`account_id`,`group_id`) REFERENCES `group`(`account_id`,`id`) ON DELETE CASCADE,
	CONSTRAINT `fk_group_membership_account_id_account_membership_id_account_membership_account_id_id_fk` FOREIGN KEY (`account_id`,`account_membership_id`) REFERENCES `account_membership`(`account_id`,`id`) ON DELETE CASCADE
);
--> statement-breakpoint
CREATE TABLE `role` (
	`id` text PRIMARY KEY,
	`created_at` integer NOT NULL,
	`updated_at` integer NOT NULL,
	`revision` integer DEFAULT 1 NOT NULL,
	`tags` text DEFAULT '{}' NOT NULL,
	`provenance` text,
	`detached_at` integer,
	`account_id` text NOT NULL,
	`name` text NOT NULL,
	`description` text NOT NULL,
	CONSTRAINT `fk_role_account_id_account_id_fk` FOREIGN KEY (`account_id`) REFERENCES `account`(`id`),
	CONSTRAINT `role_account_id` UNIQUE(`account_id`,`id`),
	CONSTRAINT "role_provenance_account" CHECK("provenance" IS NULL OR json_extract("provenance", '$.kind') <> 'account' OR json_extract("provenance", '$.accountId') = "account_id"),
	CONSTRAINT "role_provenance_detached" CHECK("detached_at" IS NULL OR ("provenance" IS NOT NULL AND "detached_at" >= 0)),
	CONSTRAINT "role_id_not_null" CHECK("id" IS NOT NULL)
);
--> statement-breakpoint
CREATE TABLE `role_binding` (
	`id` text PRIMARY KEY,
	`created_at` integer NOT NULL,
	`updated_at` integer NOT NULL,
	`revision` integer DEFAULT 1 NOT NULL,
	`tags` text DEFAULT '{}' NOT NULL,
	`provenance` text,
	`detached_at` integer,
	`account_id` text NOT NULL,
	`role_id` text NOT NULL,
	`space_id` text,
	`account_membership_id` text,
	`group_id` text,
	`service_account_id` text,
	`expires_at` integer,
	`revoked_at` integer,
	CONSTRAINT `fk_role_binding_account_id_space_id_space_account_id_id_fk` FOREIGN KEY (`account_id`,`space_id`) REFERENCES `space`(`account_id`,`id`) ON DELETE RESTRICT,
	CONSTRAINT `fk_role_binding_account_id_role_id_role_account_id_id_fk` FOREIGN KEY (`account_id`,`role_id`) REFERENCES `role`(`account_id`,`id`),
	CONSTRAINT `fk_role_binding_account_id_account_membership_id_account_membership_account_id_id_fk` FOREIGN KEY (`account_id`,`account_membership_id`) REFERENCES `account_membership`(`account_id`,`id`),
	CONSTRAINT `fk_role_binding_account_id_group_id_group_account_id_id_fk` FOREIGN KEY (`account_id`,`group_id`) REFERENCES `group`(`account_id`,`id`),
	CONSTRAINT `fk_role_binding_account_id_service_account_id_service_account_account_id_id_fk` FOREIGN KEY (`account_id`,`service_account_id`) REFERENCES `service_account`(`account_id`,`id`),
	CONSTRAINT "role_binding_provenance_account" CHECK("provenance" IS NULL OR json_extract("provenance", '$.kind') <> 'account' OR json_extract("provenance", '$.accountId') = "account_id"),
	CONSTRAINT "role_binding_provenance_space" CHECK("provenance" IS NULL OR json_extract("provenance", '$.kind') <> 'stack' OR json_extract("provenance", '$.spaceId') = "space_id"),
	CONSTRAINT "role_binding_provenance_detached" CHECK("detached_at" IS NULL OR ("provenance" IS NOT NULL AND "detached_at" >= 0)),
	CONSTRAINT "role_binding_subject" CHECK(CAST("account_membership_id" IS NOT NULL AS integer) + CAST("group_id" IS NOT NULL AS integer) + CAST("service_account_id" IS NOT NULL AS integer) = 1),
	CONSTRAINT "role_binding_expiry" CHECK("expires_at" IS NULL OR "expires_at" > "created_at"),
	CONSTRAINT "role_binding_id_not_null" CHECK("id" IS NOT NULL)
);
--> statement-breakpoint
CREATE TABLE `role_permission` (
	`id` text PRIMARY KEY,
	`role_id` text NOT NULL,
	`package_id` text NOT NULL,
	`type` text NOT NULL,
	`name` text NOT NULL,
	`object_id` text,
	CONSTRAINT `fk_role_permission_role_id_role_id_fk` FOREIGN KEY (`role_id`) REFERENCES `role`(`id`) ON DELETE CASCADE,
	CONSTRAINT "role_permission_name_0" CHECK(length("type") > 0 AND substr("type", 1, 1) GLOB '[a-z]' AND "type" NOT GLOB '*[^a-z0-9-]*' AND "type" NOT LIKE '%--%' AND "type" NOT LIKE '%-'),
	CONSTRAINT "role_permission_name_1" CHECK(length("name") > 0 AND substr("name", 1, 1) GLOB '[a-z]' AND "name" NOT GLOB '*[^a-z0-9-]*' AND "name" NOT LIKE '%--%' AND "name" NOT LIKE '%-'),
	CONSTRAINT "role_permission_object" CHECK("object_id" IS NULL OR length("object_id") > 0),
	CONSTRAINT "role_permission_id_not_null" CHECK("id" IS NOT NULL)
);
--> statement-breakpoint
CREATE TABLE `preference` (
	`id` text PRIMARY KEY,
	`created_at` integer NOT NULL,
	`updated_at` integer NOT NULL,
	`revision` integer DEFAULT 1 NOT NULL,
	`tags` text DEFAULT '{}' NOT NULL,
	`account_id` text NOT NULL,
	`user_id` text,
	`device_id` text,
	`package_id` text NOT NULL,
	`name` text NOT NULL,
	`value` text NOT NULL,
	CONSTRAINT `fk_preference_account_id_device_id_device_account_id_id_fk` FOREIGN KEY (`account_id`,`device_id`) REFERENCES `device`(`account_id`,`id`) ON DELETE CASCADE,
	CONSTRAINT `fk_preference_account_id_account_id_fk` FOREIGN KEY (`account_id`) REFERENCES `account`(`id`) ON DELETE CASCADE,
	CONSTRAINT `fk_preference_user_id_user_id_fk` FOREIGN KEY (`user_id`) REFERENCES `user`(`id`) ON DELETE CASCADE,
	CONSTRAINT `fk_preference_device_id_device_id_fk` FOREIGN KEY (`device_id`) REFERENCES `device`(`id`) ON DELETE CASCADE,
	CONSTRAINT "preference_device_user" CHECK("device_id" IS NULL OR "user_id" IS NOT NULL),
	CONSTRAINT "preference_id_not_null" CHECK("id" IS NOT NULL)
);
--> statement-breakpoint
CREATE TABLE `connected_account` (
	`id` text PRIMARY KEY,
	`created_at` integer NOT NULL,
	`updated_at` integer NOT NULL,
	`revision` integer DEFAULT 1 NOT NULL,
	`tags` text DEFAULT '{}' NOT NULL,
	`account_id` text NOT NULL,
	`user_id` text NOT NULL,
	`provider` text NOT NULL,
	`issuer` text NOT NULL,
	`subject` text NOT NULL,
	`application_id` text NOT NULL,
	`kind` text NOT NULL,
	`installation_id` text,
	`scopes` text NOT NULL,
	`permissions` text NOT NULL,
	`secret_space_id` text,
	`secret_id` text,
	`expires_at` integer,
	`revoked_at` integer,
	CONSTRAINT `fk_connected_account_account_id_secret_space_id_space_account_id_id_fk` FOREIGN KEY (`account_id`,`secret_space_id`) REFERENCES `space`(`account_id`,`id`) ON DELETE RESTRICT,
	CONSTRAINT `fk_connected_account_account_id_account_id_fk` FOREIGN KEY (`account_id`) REFERENCES `account`(`id`) ON DELETE RESTRICT,
	CONSTRAINT `fk_connected_account_user_id_user_id_fk` FOREIGN KEY (`user_id`) REFERENCES `user`(`id`) ON DELETE RESTRICT,
	CONSTRAINT `connection_account_id` UNIQUE(`account_id`,`id`),
	CONSTRAINT "connection_authorisation" CHECK(("kind" = 'oauth' AND "installation_id" IS NULL AND "secret_space_id" IS NOT NULL AND "secret_id" IS NOT NULL)
            OR ("kind" = 'installation' AND "installation_id" IS NOT NULL AND "secret_space_id" IS NULL AND "secret_id" IS NULL)),
	CONSTRAINT "connection_identifiers" CHECK(length("provider") > 0 AND length("issuer") > 0
            AND length("subject") > 0 AND length("application_id") > 0
            AND ("installation_id" IS NULL OR length("installation_id") > 0)),
	CONSTRAINT "connected_account_id_not_null" CHECK("id" IS NOT NULL)
);
--> statement-breakpoint
CREATE TABLE `service_account` (
	`id` text PRIMARY KEY,
	`created_at` integer NOT NULL,
	`updated_at` integer NOT NULL,
	`revision` integer DEFAULT 1 NOT NULL,
	`tags` text DEFAULT '{}' NOT NULL,
	`account_id` text NOT NULL,
	`name` text NOT NULL,
	`revoked_at` integer,
	CONSTRAINT `fk_service_account_account_id_account_id_fk` FOREIGN KEY (`account_id`) REFERENCES `account`(`id`),
	CONSTRAINT `service_account_name` UNIQUE(`account_id`,`name`),
	CONSTRAINT `service_account_account_id` UNIQUE(`account_id`,`id`),
	CONSTRAINT "service_account_id_not_null" CHECK("id" IS NOT NULL)
);
--> statement-breakpoint
CREATE TABLE `service_token` (
	`id` text PRIMARY KEY,
	`created_at` integer NOT NULL,
	`updated_at` integer NOT NULL,
	`revision` integer DEFAULT 1 NOT NULL,
	`tags` text DEFAULT '{}' NOT NULL,
	`account_id` text NOT NULL,
	`service_account_id` text NOT NULL,
	`name` text NOT NULL,
	`token_hash` text NOT NULL UNIQUE,
	`expires_at` integer NOT NULL,
	`revoked_at` integer,
	CONSTRAINT `fk_service_token_account_id_service_account_id_service_account_account_id_id_fk` FOREIGN KEY (`account_id`,`service_account_id`) REFERENCES `service_account`(`account_id`,`id`) ON DELETE CASCADE,
	CONSTRAINT `service_token_account_id` UNIQUE(`account_id`,`id`),
	CONSTRAINT "service_token_expiry" CHECK("expires_at" > "created_at"),
	CONSTRAINT "service_token_id_not_null" CHECK("id" IS NOT NULL)
);
--> statement-breakpoint
CREATE TABLE `service_token_permission` (
	`id` text PRIMARY KEY,
	`account_id` text NOT NULL,
	`token_id` text NOT NULL,
	`space_id` text,
	`package_id` text NOT NULL,
	`type` text NOT NULL,
	`name` text NOT NULL,
	`object_id` text,
	CONSTRAINT `fk_service_token_permission_account_id_token_id_service_token_account_id_id_fk` FOREIGN KEY (`account_id`,`token_id`) REFERENCES `service_token`(`account_id`,`id`) ON DELETE CASCADE,
	CONSTRAINT `fk_service_token_permission_account_id_space_id_space_account_id_id_fk` FOREIGN KEY (`account_id`,`space_id`) REFERENCES `space`(`account_id`,`id`) ON DELETE CASCADE,
	CONSTRAINT "service_token_permission_name_0" CHECK(length("type") > 0 AND substr("type", 1, 1) GLOB '[a-z]' AND "type" NOT GLOB '*[^a-z0-9-]*' AND "type" NOT LIKE '%--%' AND "type" NOT LIKE '%-'),
	CONSTRAINT "service_token_permission_name_1" CHECK(length("name") > 0 AND substr("name", 1, 1) GLOB '[a-z]' AND "name" NOT GLOB '*[^a-z0-9-]*' AND "name" NOT LIKE '%--%' AND "name" NOT LIKE '%-'),
	CONSTRAINT "service_token_permission_object" CHECK("object_id" IS NULL OR length("object_id") > 0),
	CONSTRAINT "service_token_permission_id_not_null" CHECK("id" IS NOT NULL)
);
--> statement-breakpoint
CREATE TABLE `organisation_membership` (
	`id` text PRIMARY KEY,
	`created_at` integer NOT NULL,
	`updated_at` integer NOT NULL,
	`revision` integer DEFAULT 1 NOT NULL,
	`tags` text DEFAULT '{}' NOT NULL,
	`organisation_id` text NOT NULL,
	`user_id` text NOT NULL,
	`role` text NOT NULL,
	CONSTRAINT `fk_organisation_membership_organisation_id_organisation_id_fk` FOREIGN KEY (`organisation_id`) REFERENCES `organisation`(`id`) ON DELETE CASCADE,
	CONSTRAINT `fk_organisation_membership_user_id_user_id_fk` FOREIGN KEY (`user_id`) REFERENCES `user`(`id`) ON DELETE RESTRICT,
	CONSTRAINT `organisation_membership_user` UNIQUE(`organisation_id`,`user_id`),
	CONSTRAINT "organisation_membership_role" CHECK("role" IN ('owner', 'admin', 'member')),
	CONSTRAINT "organisation_membership_id_not_null" CHECK("id" IS NOT NULL)
);
--> statement-breakpoint
CREATE TABLE `personal_access_token` (
	`id` text PRIMARY KEY,
	`created_at` integer NOT NULL,
	`updated_at` integer NOT NULL,
	`revision` integer DEFAULT 1 NOT NULL,
	`tags` text DEFAULT '{}' NOT NULL,
	`user_id` text NOT NULL,
	`account_id` text NOT NULL,
	`name` text NOT NULL,
	`token_hash` text NOT NULL UNIQUE,
	`expires_at` integer NOT NULL,
	`revoked_at` integer,
	CONSTRAINT `fk_personal_access_token_user_id_user_id_fk` FOREIGN KEY (`user_id`) REFERENCES `user`(`id`) ON DELETE CASCADE,
	CONSTRAINT `fk_personal_access_token_account_id_account_id_fk` FOREIGN KEY (`account_id`) REFERENCES `account`(`id`) ON DELETE CASCADE,
	CONSTRAINT `personal_access_token_account_id` UNIQUE(`account_id`,`id`),
	CONSTRAINT `personal_access_token_user_id` UNIQUE(`user_id`,`id`),
	CONSTRAINT "personal_access_token_expiry" CHECK("expires_at" > "created_at"),
	CONSTRAINT "personal_access_token_id_not_null" CHECK("id" IS NOT NULL)
);
--> statement-breakpoint
CREATE TABLE `personal_access_token_permission` (
	`id` text PRIMARY KEY,
	`account_id` text NOT NULL,
	`token_id` text NOT NULL,
	`space_id` text,
	`package_id` text NOT NULL,
	`type` text NOT NULL,
	`name` text NOT NULL,
	`object_id` text,
	CONSTRAINT `fk_personal_access_token_permission_account_id_token_id_personal_access_token_account_id_id_fk` FOREIGN KEY (`account_id`,`token_id`) REFERENCES `personal_access_token`(`account_id`,`id`) ON DELETE CASCADE,
	CONSTRAINT `fk_personal_access_token_permission_account_id_space_id_space_account_id_id_fk` FOREIGN KEY (`account_id`,`space_id`) REFERENCES `space`(`account_id`,`id`) ON DELETE CASCADE,
	CONSTRAINT "personal_access_token_permission_name_0" CHECK(length("type") > 0 AND substr("type", 1, 1) GLOB '[a-z]' AND "type" NOT GLOB '*[^a-z0-9-]*' AND "type" NOT LIKE '%--%' AND "type" NOT LIKE '%-'),
	CONSTRAINT "personal_access_token_permission_name_1" CHECK(length("name") > 0 AND substr("name", 1, 1) GLOB '[a-z]' AND "name" NOT GLOB '*[^a-z0-9-]*' AND "name" NOT LIKE '%--%' AND "name" NOT LIKE '%-'),
	CONSTRAINT "personal_access_token_permission_object" CHECK("object_id" IS NULL OR length("object_id") > 0),
	CONSTRAINT "personal_access_token_permission_id_not_null" CHECK("id" IS NOT NULL)
);
--> statement-breakpoint
CREATE TABLE `identity` (
	`id` text PRIMARY KEY,
	`created_at` integer NOT NULL,
	`updated_at` integer NOT NULL,
	`revision` integer DEFAULT 1 NOT NULL,
	`tags` text DEFAULT '{}' NOT NULL,
	`user_id` text NOT NULL,
	`provider_id` text NOT NULL,
	`provider_user_id` text NOT NULL,
	`access_token` text,
	`refresh_token` text,
	`id_token` text,
	`access_token_expires_at` integer,
	`refresh_token_expires_at` integer,
	`scope` text,
	`password` text,
	CONSTRAINT `fk_identity_user_id_user_id_fk` FOREIGN KEY (`user_id`) REFERENCES `user`(`id`) ON DELETE CASCADE,
	CONSTRAINT `identity_provider_user` UNIQUE(`provider_id`,`provider_user_id`),
	CONSTRAINT "identity_id_not_null" CHECK("id" IS NOT NULL)
);
--> statement-breakpoint
CREATE TABLE `session` (
	`id` text PRIMARY KEY,
	`user_id` text NOT NULL,
	`device_id` text,
	`token` text NOT NULL UNIQUE,
	`created_at` integer NOT NULL,
	`updated_at` integer NOT NULL,
	`ip_address` text,
	`user_agent` text,
	`country` text,
	`city` text,
	`authenticated_at` integer,
	`authentication_method` text,
	`expires_at` integer NOT NULL,
	`revoked_at` integer,
	CONSTRAINT `fk_session_user_id_user_id_fk` FOREIGN KEY (`user_id`) REFERENCES `user`(`id`) ON DELETE CASCADE,
	CONSTRAINT `fk_session_device_id_device_id_fk` FOREIGN KEY (`device_id`) REFERENCES `device`(`id`) ON DELETE RESTRICT,
	CONSTRAINT "session_expiry_order" CHECK("expires_at" > "created_at"),
	CONSTRAINT "session_authentication_method" CHECK("authentication_method" IS NULL OR "authentication_method" IN ('magic-link', 'email-otp', 'oauth', 'device', 'totp', 'webauthn', 'recovery')),
	CONSTRAINT "session_authentication_time" CHECK("authentication_method" IS NULL OR "authenticated_at" IS NOT NULL),
	CONSTRAINT "session_id_not_null" CHECK("id" IS NOT NULL)
);
--> statement-breakpoint
CREATE TABLE `passkey` (
	`id` text PRIMARY KEY,
	`name` text,
	`public_key` text NOT NULL,
	`user_id` text NOT NULL,
	`credential_id` text NOT NULL UNIQUE,
	`counter` integer NOT NULL,
	`device_type` text NOT NULL,
	`backed_up` integer NOT NULL,
	`transports` text,
	`created_at` integer,
	`aaguid` text,
	CONSTRAINT `fk_passkey_user_id_user_id_fk` FOREIGN KEY (`user_id`) REFERENCES `user`(`id`) ON DELETE CASCADE,
	CONSTRAINT "passkey_id_not_null" CHECK("id" IS NOT NULL)
);
--> statement-breakpoint
CREATE TABLE `two_factor` (
	`id` text PRIMARY KEY,
	`secret` text NOT NULL,
	`backup_codes` text NOT NULL,
	`user_id` text NOT NULL,
	`verified` integer DEFAULT true,
	`failed_verification_count` integer DEFAULT 0,
	`locked_until` integer,
	CONSTRAINT `fk_two_factor_user_id_user_id_fk` FOREIGN KEY (`user_id`) REFERENCES `user`(`id`) ON DELETE CASCADE,
	CONSTRAINT "two_factor_id_not_null" CHECK("id" IS NOT NULL)
);
--> statement-breakpoint
CREATE TABLE `device_authorization` (
	`id` text PRIMARY KEY,
	`device_code` text NOT NULL UNIQUE,
	`user_code` text NOT NULL UNIQUE,
	`user_id` text,
	`expires_at` integer NOT NULL,
	`status` text NOT NULL,
	`last_polled_at` integer,
	`polling_interval` integer,
	`client_id` text,
	`oauth_client_id` text,
	`resources` text,
	`scope` text,
	CONSTRAINT `fk_device_authorization_user_id_user_id_fk` FOREIGN KEY (`user_id`) REFERENCES `user`(`id`) ON DELETE CASCADE,
	CONSTRAINT `fk_device_authorization_oauth_client_id_oauth_client_client_id_fk` FOREIGN KEY (`oauth_client_id`) REFERENCES `oauth_client`(`client_id`) ON DELETE CASCADE,
	CONSTRAINT "device_authorization_status" CHECK("status" IN ('pending', 'approved', 'denied')),
	CONSTRAINT "device_authorization_id_not_null" CHECK("id" IS NOT NULL)
);
--> statement-breakpoint
CREATE TABLE `signing_key` (
	`id` text PRIMARY KEY,
	`public_key` text NOT NULL,
	`private_key` text NOT NULL,
	`created_at` integer NOT NULL,
	`expires_at` integer,
	`alg` text,
	`crv` text,
	CONSTRAINT "signing_key_id_not_null" CHECK("id" IS NOT NULL)
);
--> statement-breakpoint
CREATE TABLE `authentication_replay` (
	`id` text PRIMARY KEY,
	`expires_at` integer NOT NULL,
	CONSTRAINT "authentication_replay_id_not_null" CHECK("id" IS NOT NULL)
);
--> statement-breakpoint
CREATE TABLE `oauth_client` (
	`id` text PRIMARY KEY,
	`client_id` text NOT NULL UNIQUE,
	`account_id` text,
	`service_account_id` text,
	`client_secret` text,
	`client_discovery_id` text,
	`subject_type` text,
	`name` text,
	`uri` text,
	`icon` text,
	`tos` text,
	`policy` text,
	`software_id` text,
	`software_version` text,
	`software_statement` text,
	`backchannel_logout_uri` text,
	`token_endpoint_auth_method` text,
	`application_type` text,
	`jwks` text,
	`jwks_uri` text,
	`reference_id` text,
	`disabled` integer,
	`skip_consent` integer,
	`enable_end_session` integer,
	`backchannel_logout_session_required` integer,
	`require_pkce` integer,
	`dpop_bound_access_tokens` integer,
	`scopes` text,
	`client_credentials_scopes` text,
	`contacts` text,
	`redirect_uris` text NOT NULL,
	`post_logout_redirect_uris` text,
	`grant_types` text,
	`response_types` text,
	`user_id` text,
	`created_at` integer,
	`updated_at` integer,
	`metadata` text,
	CONSTRAINT `fk_oauth_client_account_id_service_account_id_service_account_account_id_id_fk` FOREIGN KEY (`account_id`,`service_account_id`) REFERENCES `service_account`(`account_id`,`id`) ON DELETE CASCADE,
	CONSTRAINT `fk_oauth_client_account_id_account_id_fk` FOREIGN KEY (`account_id`) REFERENCES `account`(`id`) ON DELETE CASCADE,
	CONSTRAINT `fk_oauth_client_user_id_user_id_fk` FOREIGN KEY (`user_id`) REFERENCES `user`(`id`) ON DELETE RESTRICT,
	CONSTRAINT "oauth_client_service_account" CHECK("service_account_id" IS NULL OR "account_id" IS NOT NULL),
	CONSTRAINT "oauth_client_id_not_null" CHECK("id" IS NOT NULL)
);
--> statement-breakpoint
CREATE TABLE `organisation_invitation` (
	`id` text PRIMARY KEY,
	`created_at` integer NOT NULL,
	`updated_at` integer NOT NULL,
	`revision` integer DEFAULT 1 NOT NULL,
	`tags` text DEFAULT '{}' NOT NULL,
	`organisation_id` text NOT NULL,
	`invited_by` text NOT NULL,
	`email` text NOT NULL,
	`role` text NOT NULL,
	`token_hash` text NOT NULL UNIQUE,
	`expires_at` integer NOT NULL,
	`accepted_by` text,
	`accepted_at` integer,
	`revoked_at` integer,
	CONSTRAINT `fk_organisation_invitation_organisation_id_organisation_id_fk` FOREIGN KEY (`organisation_id`) REFERENCES `organisation`(`id`) ON DELETE CASCADE,
	CONSTRAINT `fk_organisation_invitation_invited_by_user_id_fk` FOREIGN KEY (`invited_by`) REFERENCES `user`(`id`),
	CONSTRAINT `fk_organisation_invitation_accepted_by_user_id_fk` FOREIGN KEY (`accepted_by`) REFERENCES `user`(`id`),
	CONSTRAINT "organisation_invitation_role" CHECK("role" IN ('owner', 'admin', 'member')),
	CONSTRAINT "organisation_invitation_expiry" CHECK("expires_at" > "created_at"),
	CONSTRAINT "organisation_invitation_acceptance" CHECK(("accepted_by" IS NULL) = ("accepted_at" IS NULL) AND ("accepted_at" IS NULL OR ("revoked_at" IS NULL AND "accepted_at" >= "created_at" AND "accepted_at" < "expires_at"))),
	CONSTRAINT "organisation_invitation_id_not_null" CHECK("id" IS NOT NULL)
);
--> statement-breakpoint
CREATE TABLE `oauth_resource` (
	`id` text PRIMARY KEY,
	`identifier` text NOT NULL UNIQUE,
	`name` text NOT NULL,
	`access_token_ttl` integer,
	`refresh_token_ttl` integer,
	`signing_algorithm` text,
	`signing_key_id` text,
	`allowed_scopes` text,
	`custom_claims` text,
	`dpop_bound_access_tokens_required` integer DEFAULT false,
	`disabled` integer DEFAULT false,
	`created_at` integer,
	`updated_at` integer,
	`policy_version` integer DEFAULT 1,
	`metadata` text,
	CONSTRAINT "oauth_resource_id_not_null" CHECK("id" IS NOT NULL)
);
--> statement-breakpoint
CREATE TABLE `oauth_client_resource` (
	`id` text PRIMARY KEY,
	`client_id` text NOT NULL,
	`resource_id` text NOT NULL,
	`metadata` text,
	`created_at` integer,
	CONSTRAINT `fk_oauth_client_resource_client_id_oauth_client_client_id_fk` FOREIGN KEY (`client_id`) REFERENCES `oauth_client`(`client_id`) ON DELETE CASCADE,
	CONSTRAINT `fk_oauth_client_resource_resource_id_oauth_resource_identifier_fk` FOREIGN KEY (`resource_id`) REFERENCES `oauth_resource`(`identifier`) ON DELETE CASCADE,
	CONSTRAINT `oauth_client_resource_grant` UNIQUE(`client_id`,`resource_id`),
	CONSTRAINT "oauth_client_resource_id_not_null" CHECK("id" IS NOT NULL)
);
--> statement-breakpoint
CREATE TABLE `oauth_consent` (
	`id` text PRIMARY KEY,
	`client_id` text NOT NULL,
	`user_id` text,
	`reference_id` text,
	`resources` text,
	`requested_user_info_claims` text,
	`scopes` text NOT NULL,
	`created_at` integer NOT NULL,
	`updated_at` integer NOT NULL,
	CONSTRAINT `fk_oauth_consent_client_id_oauth_client_client_id_fk` FOREIGN KEY (`client_id`) REFERENCES `oauth_client`(`client_id`) ON DELETE CASCADE,
	CONSTRAINT `fk_oauth_consent_user_id_user_id_fk` FOREIGN KEY (`user_id`) REFERENCES `user`(`id`) ON DELETE CASCADE,
	CONSTRAINT "oauth_consent_id_not_null" CHECK("id" IS NOT NULL)
);
--> statement-breakpoint
CREATE TABLE `oauth_access_token` (
	`id` text PRIMARY KEY,
	`token` text NOT NULL UNIQUE,
	`client_id` text NOT NULL,
	`session_id` text,
	`user_id` text,
	`reference_id` text,
	`authorization_code_id` text,
	`resources` text,
	`requested_user_info_claims` text,
	`expires_at` integer NOT NULL,
	`created_at` integer NOT NULL,
	`revoked` integer,
	`confirmation` text,
	`scopes` text NOT NULL,
	`refresh_id` text,
	CONSTRAINT `fk_oauth_access_token_client_id_oauth_client_client_id_fk` FOREIGN KEY (`client_id`) REFERENCES `oauth_client`(`client_id`) ON DELETE CASCADE,
	CONSTRAINT `fk_oauth_access_token_session_id_session_id_fk` FOREIGN KEY (`session_id`) REFERENCES `session`(`id`) ON DELETE SET NULL,
	CONSTRAINT `fk_oauth_access_token_user_id_user_id_fk` FOREIGN KEY (`user_id`) REFERENCES `user`(`id`) ON DELETE CASCADE,
	CONSTRAINT `fk_oauth_access_token_refresh_id_oauth_refresh_token_id_fk` FOREIGN KEY (`refresh_id`) REFERENCES `oauth_refresh_token`(`id`) ON DELETE CASCADE,
	CONSTRAINT "oauth_access_token_id_not_null" CHECK("id" IS NOT NULL)
);
--> statement-breakpoint
CREATE TABLE `oauth_refresh_token` (
	`id` text PRIMARY KEY,
	`token` text NOT NULL UNIQUE,
	`client_id` text NOT NULL,
	`session_id` text,
	`user_id` text NOT NULL,
	`reference_id` text,
	`authorization_code_id` text,
	`resources` text,
	`requested_user_info_claims` text,
	`expires_at` integer NOT NULL,
	`created_at` integer NOT NULL,
	`revoked` integer,
	`confirmation` text,
	`scopes` text NOT NULL,
	`rotated_at` integer,
	`rotation_replay_response` text,
	`rotation_replay_expires_at` integer,
	`auth_time` integer,
	CONSTRAINT `fk_oauth_refresh_token_client_id_oauth_client_client_id_fk` FOREIGN KEY (`client_id`) REFERENCES `oauth_client`(`client_id`) ON DELETE CASCADE,
	CONSTRAINT `fk_oauth_refresh_token_session_id_session_id_fk` FOREIGN KEY (`session_id`) REFERENCES `session`(`id`) ON DELETE SET NULL,
	CONSTRAINT `fk_oauth_refresh_token_user_id_user_id_fk` FOREIGN KEY (`user_id`) REFERENCES `user`(`id`) ON DELETE CASCADE,
	CONSTRAINT "oauth_refresh_token_id_not_null" CHECK("id" IS NOT NULL)
);
--> statement-breakpoint
CREATE TABLE `oauth_client_assertion` (
	`id` text PRIMARY KEY,
	`expires_at` integer NOT NULL,
	CONSTRAINT "oauth_client_assertion_id_not_null" CHECK("id" IS NOT NULL)
);
--> statement-breakpoint
CREATE TABLE `device` (
	`id` text PRIMARY KEY,
	`created_at` integer NOT NULL,
	`updated_at` integer NOT NULL,
	`revision` integer DEFAULT 1 NOT NULL,
	`tags` text DEFAULT '{}' NOT NULL,
	`account_id` text NOT NULL,
	`name` text NOT NULL,
	`enrolled_by` text,
	`category` text DEFAULT 'unknown' NOT NULL,
	`operating_system` text,
	`operating_system_version` text,
	`architecture` text,
	`model` text,
	`client_version` text,
	`revoked_at` integer,
	`last_seen_at` integer,
	CONSTRAINT `fk_device_account_id_account_id_fk` FOREIGN KEY (`account_id`) REFERENCES `account`(`id`) ON DELETE RESTRICT,
	CONSTRAINT `fk_device_enrolled_by_user_id_fk` FOREIGN KEY (`enrolled_by`) REFERENCES `user`(`id`) ON DELETE SET NULL,
	CONSTRAINT `device_account_id` UNIQUE(`account_id`,`id`),
	CONSTRAINT "device_category" CHECK("category" IN ('desktop', 'laptop', 'phone', 'tablet', 'server', 'unknown')),
	CONSTRAINT "device_id_not_null" CHECK("id" IS NOT NULL)
);
--> statement-breakpoint
CREATE TABLE `device_key` (
	`id` text PRIMARY KEY,
	`created_at` integer NOT NULL,
	`updated_at` integer NOT NULL,
	`revision` integer DEFAULT 1 NOT NULL,
	`tags` text DEFAULT '{}' NOT NULL,
	`device_id` text NOT NULL,
	`public_key` text NOT NULL UNIQUE,
	`thumbprint` text NOT NULL UNIQUE,
	`verified_at` integer NOT NULL,
	`expires_at` integer NOT NULL,
	`revoked_at` integer,
	CONSTRAINT `fk_device_key_device_id_device_id_fk` FOREIGN KEY (`device_id`) REFERENCES `device`(`id`),
	CONSTRAINT `device_key_device_id` UNIQUE(`device_id`,`id`),
	CONSTRAINT "device_key_expiry" CHECK("expires_at" > "created_at"),
	CONSTRAINT "device_key_verification" CHECK("verified_at" >= "created_at" AND "verified_at" < "expires_at"),
	CONSTRAINT "device_key_id_not_null" CHECK("id" IS NOT NULL)
);
--> statement-breakpoint
CREATE TABLE `host` (
	`id` text PRIMARY KEY,
	`created_at` integer NOT NULL,
	`updated_at` integer NOT NULL,
	`revision` integer DEFAULT 1 NOT NULL,
	`tags` text DEFAULT '{}' NOT NULL,
	`account_id` text NOT NULL,
	`kind` text NOT NULL,
	`device_id` text,
	`region_id` text,
	`state` text DEFAULT 'enabled' NOT NULL,
	`version` text,
	`runtimes` text DEFAULT '[]' NOT NULL,
	`last_seen_at` integer,
	CONSTRAINT `fk_host_account_id_device_id_device_account_id_id_fk` FOREIGN KEY (`account_id`,`device_id`) REFERENCES `device`(`account_id`,`id`) ON DELETE RESTRICT,
	CONSTRAINT `fk_host_account_id_account_id_fk` FOREIGN KEY (`account_id`) REFERENCES `account`(`id`) ON DELETE RESTRICT,
	CONSTRAINT `fk_host_region_id_region_id_fk` FOREIGN KEY (`region_id`) REFERENCES `region`(`id`),
	CONSTRAINT `host_device_id` UNIQUE(`device_id`,`id`),
	CONSTRAINT `host_account_id` UNIQUE(`account_id`,`id`),
	CONSTRAINT "host_state" CHECK("state" IN ('enabled', 'draining', 'disabled')),
	CONSTRAINT "host_kind" CHECK(("kind" = 'device' AND "device_id" IS NOT NULL) OR ("kind" = 'cloud' AND "device_id" IS NULL)),
	CONSTRAINT "host_id_not_null" CHECK("id" IS NOT NULL)
);
--> statement-breakpoint
CREATE TABLE `host_access` (
	`host_id` text NOT NULL,
	`account_id` text NOT NULL,
	`created_at` integer NOT NULL,
	`revoked_at` integer,
	CONSTRAINT `host_access_pk` PRIMARY KEY(`account_id`, `host_id`),
	CONSTRAINT `fk_host_access_host_id_host_id_fk` FOREIGN KEY (`host_id`) REFERENCES `host`(`id`),
	CONSTRAINT `fk_host_access_account_id_account_id_fk` FOREIGN KEY (`account_id`) REFERENCES `account`(`id`)
);
--> statement-breakpoint
CREATE TABLE `account` (
	`id` text PRIMARY KEY,
	`created_at` integer NOT NULL,
	`updated_at` integer NOT NULL,
	`revision` integer DEFAULT 1 NOT NULL,
	`tags` text DEFAULT '{}' NOT NULL,
	`handle` text NOT NULL UNIQUE,
	`name` text NOT NULL,
	`default_residency` text NOT NULL,
	`package_policy_id` text,
	`package_policy_region_id` text,
	`network_policy_id` text,
	`network_policy_region_id` text,
	`suspended_at` integer,
	`deletion_requested_at` integer,
	`kind` text NOT NULL,
	`user_id` text,
	`organisation_id` text,
	CONSTRAINT `fk_account_package_policy_region_id_region_id_fk` FOREIGN KEY (`package_policy_region_id`) REFERENCES `region`(`id`) ON DELETE RESTRICT,
	CONSTRAINT `fk_account_network_policy_region_id_region_id_fk` FOREIGN KEY (`network_policy_region_id`) REFERENCES `region`(`id`) ON DELETE RESTRICT,
	CONSTRAINT `fk_account_user_id_user_id_fk` FOREIGN KEY (`user_id`) REFERENCES `user`(`id`) ON DELETE RESTRICT,
	CONSTRAINT `fk_account_organisation_id_organisation_id_fk` FOREIGN KEY (`organisation_id`) REFERENCES `organisation`(`id`) ON DELETE RESTRICT,
	CONSTRAINT "account_residency" CHECK("default_residency" IN ('eu', 'us')),
	CONSTRAINT "account_kind" CHECK("kind" IN ('personal', 'organisation')),
	CONSTRAINT "account_network_policy" CHECK(
        ("network_policy_id" IS NULL AND "network_policy_region_id" IS NULL) OR
        ("network_policy_id" IS NOT NULL AND "network_policy_region_id" IS NOT NULL)
    ),
	CONSTRAINT "account_package_policy" CHECK(
        ("package_policy_id" IS NULL AND "package_policy_region_id" IS NULL) OR
        ("package_policy_id" IS NOT NULL AND "package_policy_region_id" IS NOT NULL)
    ),
	CONSTRAINT "account_user" CHECK(("kind" = 'personal' AND "user_id" IS NOT NULL AND "organisation_id" IS NULL) OR ("kind" = 'organisation' AND "user_id" IS NULL AND "organisation_id" IS NOT NULL)),
	CONSTRAINT "account_handle" CHECK(length("handle") BETWEEN 1 AND 63 AND "handle" NOT GLOB '*[^a-z0-9-]*' AND "handle" NOT LIKE '-%' AND "handle" NOT LIKE '%-'),
	CONSTRAINT "account_id_not_null" CHECK("id" IS NOT NULL)
);
--> statement-breakpoint
CREATE TABLE `region` (
	`id` text PRIMARY KEY,
	`created_at` integer NOT NULL,
	`updated_at` integer NOT NULL,
	`revision` integer DEFAULT 1 NOT NULL,
	`tags` text DEFAULT '{}' NOT NULL,
	`provider` text NOT NULL,
	`code` text NOT NULL,
	`name` text NOT NULL,
	`residency` text NOT NULL,
	CONSTRAINT `region_provider_code` UNIQUE(`provider`,`code`),
	CONSTRAINT `region_provider_id` UNIQUE(`provider`,`id`),
	CONSTRAINT `region_residency_id` UNIQUE(`id`,`residency`),
	CONSTRAINT "region_residency" CHECK("residency" IN ('eu', 'us')),
	CONSTRAINT "region_id_not_null" CHECK("id" IS NOT NULL)
);
--> statement-breakpoint
CREATE TABLE `tunnel` (
	`id` text PRIMARY KEY,
	`created_at` integer NOT NULL,
	`updated_at` integer NOT NULL,
	`revision` integer DEFAULT 1 NOT NULL,
	`tags` text DEFAULT '{}' NOT NULL,
	`host_id` text NOT NULL,
	`device_id` text NOT NULL,
	`device_key_id` text NOT NULL,
	`relay` text NOT NULL,
	`connection` text NOT NULL,
	`heartbeat_at` integer NOT NULL,
	`expires_at` integer NOT NULL,
	`closed_at` integer,
	CONSTRAINT `fk_tunnel_device_id_host_id_host_device_id_id_fk` FOREIGN KEY (`device_id`,`host_id`) REFERENCES `host`(`device_id`,`id`),
	CONSTRAINT `fk_tunnel_device_id_device_key_id_device_key_device_id_id_fk` FOREIGN KEY (`device_id`,`device_key_id`) REFERENCES `device_key`(`device_id`,`id`),
	CONSTRAINT `fk_tunnel_host_id_host_id_fk` FOREIGN KEY (`host_id`) REFERENCES `host`(`id`),
	CONSTRAINT `fk_tunnel_device_id_device_id_fk` FOREIGN KEY (`device_id`) REFERENCES `device`(`id`),
	CONSTRAINT `tunnel_connection` UNIQUE(`relay`,`connection`),
	CONSTRAINT "tunnel_lease" CHECK("heartbeat_at" >= "created_at" AND "expires_at" > "heartbeat_at"),
	CONSTRAINT "tunnel_close" CHECK("closed_at" IS NULL OR "closed_at" >= "created_at"),
	CONSTRAINT "tunnel_id_not_null" CHECK("id" IS NOT NULL)
);
--> statement-breakpoint
CREATE TABLE `space` (
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
	`residency` text NOT NULL,
	`region_id` text NOT NULL,
	`account_id` text NOT NULL,
	`name` text NOT NULL,
	`environment_id` text,
	CONSTRAINT `fk_space_account_id_environment_id_environment_account_id_id_fk` FOREIGN KEY (`account_id`,`environment_id`) REFERENCES `environment`(`account_id`,`id`) ON DELETE RESTRICT,
	CONSTRAINT `fk_space_region_id_residency_region_id_residency_fk` FOREIGN KEY (`region_id`,`residency`) REFERENCES `region`(`id`,`residency`) ON DELETE RESTRICT,
	CONSTRAINT `fk_space_account_id_account_id_fk` FOREIGN KEY (`account_id`) REFERENCES `account`(`id`) ON DELETE RESTRICT,
	CONSTRAINT `space_name` UNIQUE(`account_id`,`name`),
	CONSTRAINT `space_account` UNIQUE(`account_id`,`id`),
	CONSTRAINT "space_revision" CHECK("revision" >= 1),
	CONSTRAINT "space_generation" CHECK("generation" >= 1),
	CONSTRAINT "space_observed_generation" CHECK("observed_generation" BETWEEN 0 AND "generation"),
	CONSTRAINT "space_name_value" CHECK(length("name") BETWEEN 1 AND 63 AND "name" NOT GLOB '*[^a-z0-9-]*' AND "name" NOT LIKE '-%' AND "name" NOT LIKE '%-'),
	CONSTRAINT "space_id_not_null" CHECK("id" IS NOT NULL)
);
--> statement-breakpoint
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
	`residency` text NOT NULL,
	`region_id` text NOT NULL,
	`account_id` text NOT NULL,
	`name` text NOT NULL,
	CONSTRAINT `fk_repository_region_id_residency_region_id_residency_fk` FOREIGN KEY (`region_id`,`residency`) REFERENCES `region`(`id`,`residency`) ON DELETE RESTRICT,
	CONSTRAINT `fk_repository_account_id_account_id_fk` FOREIGN KEY (`account_id`) REFERENCES `account`(`id`) ON DELETE RESTRICT,
	CONSTRAINT `repository_name` UNIQUE(`account_id`,`name`),
	CONSTRAINT `repository_account` UNIQUE(`account_id`,`id`),
	CONSTRAINT "repository_revision" CHECK("revision" >= 1),
	CONSTRAINT "repository_generation" CHECK("generation" >= 1),
	CONSTRAINT "repository_observed_generation" CHECK("observed_generation" BETWEEN 0 AND "generation"),
	CONSTRAINT "repository_name_value" CHECK(length("name") BETWEEN 1 AND 63 AND "name" NOT GLOB '*[^a-z0-9-]*' AND "name" NOT LIKE '-%' AND "name" NOT LIKE '%-'),
	CONSTRAINT "repository_id_not_null" CHECK("id" IS NOT NULL)
);
--> statement-breakpoint
CREATE TABLE `package` (
	`id` text PRIMARY KEY,
	`created_at` integer NOT NULL,
	`updated_at` integer NOT NULL,
	`revision` integer DEFAULT 1 NOT NULL,
	`tags` text DEFAULT '{}' NOT NULL,
	`account_id` text NOT NULL,
	`name` text NOT NULL,
	`repository_id` text NOT NULL,
	CONSTRAINT `fk_package_account_id_repository_id_repository_account_id_id_fk` FOREIGN KEY (`account_id`,`repository_id`) REFERENCES `repository`(`account_id`,`id`) ON DELETE RESTRICT,
	CONSTRAINT `package_name` UNIQUE(`account_id`,`name`),
	CONSTRAINT "package_id_not_null" CHECK("id" IS NOT NULL)
);
--> statement-breakpoint
CREATE TABLE `domain` (
	`id` text PRIMARY KEY,
	`created_at` integer NOT NULL,
	`updated_at` integer NOT NULL,
	`revision` integer DEFAULT 1 NOT NULL,
	`tags` text DEFAULT '{}' NOT NULL,
	`account_id` text NOT NULL,
	`hostname` text NOT NULL UNIQUE,
	`verified_at` integer,
	CONSTRAINT `fk_domain_account_id_account_id_fk` FOREIGN KEY (`account_id`) REFERENCES `account`(`id`) ON DELETE RESTRICT,
	CONSTRAINT `domain_account_id` UNIQUE(`account_id`,`id`),
	CONSTRAINT "domain_hostname" CHECK(length("hostname") BETWEEN 1 AND 253 AND "hostname" = lower("hostname")),
	CONSTRAINT "domain_id_not_null" CHECK("id" IS NOT NULL)
);
--> statement-breakpoint
CREATE TABLE `route` (
	`id` text PRIMARY KEY,
	`created_at` integer NOT NULL,
	`updated_at` integer NOT NULL,
	`revision` integer DEFAULT 1 NOT NULL,
	`tags` text DEFAULT '{}' NOT NULL,
	`provenance` text,
	`detached_at` integer,
	`account_id` text NOT NULL,
	`domain_id` text NOT NULL,
	`path` text NOT NULL,
	`match` text NOT NULL,
	`kind` text NOT NULL,
	`space_id` text,
	`installation_id` text,
	`entrypoint` text,
	`redirect` text,
	`redirect_status` integer,
	`applied_generation` integer DEFAULT 0 NOT NULL,
	`generation` integer DEFAULT 1 NOT NULL,
	`observed_generation` integer DEFAULT 0 NOT NULL,
	`conditions` text DEFAULT '{}' NOT NULL,
	`deletion_requested_at` integer,
	`finalizers` text DEFAULT '[]' NOT NULL,
	CONSTRAINT `fk_route_account_id_domain_id_domain_account_id_id_fk` FOREIGN KEY (`account_id`,`domain_id`) REFERENCES `domain`(`account_id`,`id`) ON DELETE RESTRICT,
	CONSTRAINT `fk_route_account_id_account_id_fk` FOREIGN KEY (`account_id`) REFERENCES `account`(`id`) ON DELETE RESTRICT,
	CONSTRAINT `fk_route_space_id_space_id_fk` FOREIGN KEY (`space_id`) REFERENCES `space`(`id`) ON DELETE RESTRICT,
	CONSTRAINT `route_domain_path_match` UNIQUE(`domain_id`,`path`,`match`),
	CONSTRAINT "route_provenance_account" CHECK("provenance" IS NULL OR json_extract("provenance", '$.kind') <> 'account' OR json_extract("provenance", '$.accountId') = "account_id"),
	CONSTRAINT "route_provenance_space" CHECK("provenance" IS NULL OR json_extract("provenance", '$.kind') <> 'stack' OR json_extract("provenance", '$.spaceId') = "space_id"),
	CONSTRAINT "route_provenance_detached" CHECK("detached_at" IS NULL OR ("provenance" IS NOT NULL AND "detached_at" >= 0)),
	CONSTRAINT "route_revision" CHECK("revision" >= 1),
	CONSTRAINT "route_generation" CHECK("generation" >= 1),
	CONSTRAINT "route_observed_generation" CHECK("observed_generation" BETWEEN 0 AND "generation"),
	CONSTRAINT "route_match" CHECK("match" IN ('exact', 'prefix')),
	CONSTRAINT "route_path" CHECK(substr("path", 1, 1) = '/'),
	CONSTRAINT "route_applied_generation" CHECK("applied_generation" BETWEEN 0 AND "generation"),
	CONSTRAINT "route_destination" CHECK(("kind" = 'application' AND "space_id" IS NOT NULL AND "installation_id" IS NOT NULL AND "entrypoint" IS NOT NULL AND "redirect" IS NULL AND "redirect_status" IS NULL) OR ("kind" = 'redirect' AND "space_id" IS NULL AND "installation_id" IS NULL AND "entrypoint" IS NULL AND "redirect" IS NOT NULL AND "redirect_status" IN (301, 302, 303, 307, 308) AND "redirect_status" IS NOT NULL)),
	CONSTRAINT "route_id_not_null" CHECK("id" IS NOT NULL)
);
--> statement-breakpoint
CREATE TABLE `environment` (
	`id` text PRIMARY KEY,
	`created_at` integer NOT NULL,
	`updated_at` integer NOT NULL,
	`revision` integer DEFAULT 1 NOT NULL,
	`tags` text DEFAULT '{}' NOT NULL,
	`provenance` text,
	`detached_at` integer,
	`account_id` text NOT NULL,
	`name` text NOT NULL,
	CONSTRAINT `fk_environment_account_id_account_id_fk` FOREIGN KEY (`account_id`) REFERENCES `account`(`id`) ON DELETE RESTRICT,
	CONSTRAINT `environment_account_name` UNIQUE(`account_id`,`name`),
	CONSTRAINT `environment_account_id` UNIQUE(`account_id`,`id`),
	CONSTRAINT "environment_name" CHECK(length("name") > 0 AND substr("name", 1, 1) GLOB '[a-z]' AND "name" NOT GLOB '*[^a-z0-9-]*'),
	CONSTRAINT "environment_provenance_account" CHECK("provenance" IS NULL OR json_extract("provenance", '$.kind') <> 'account' OR json_extract("provenance", '$.accountId') = "account_id"),
	CONSTRAINT "environment_provenance_detached" CHECK("detached_at" IS NULL OR ("provenance" IS NOT NULL AND "detached_at" >= 0)),
	CONSTRAINT "environment_id_not_null" CHECK("id" IS NOT NULL)
);
--> statement-breakpoint
CREATE TABLE `account_source` (
	`account_id` text PRIMARY KEY,
	`repository_id` text NOT NULL,
	`reference` text NOT NULL,
	`directory` text NOT NULL,
	`entrypoint` text NOT NULL,
	`export` text NOT NULL,
	`parameters` text NOT NULL,
	`generation` integer DEFAULT 1 NOT NULL,
	`applied_revision_id` text,
	CONSTRAINT `fk_account_source_account_id_repository_id_repository_account_id_id_fk` FOREIGN KEY (`account_id`,`repository_id`) REFERENCES `repository`(`account_id`,`id`) ON DELETE RESTRICT,
	CONSTRAINT `fk_account_source_account_id_applied_revision_id_account_revision_account_id_id_fk` FOREIGN KEY (`account_id`,`applied_revision_id`) REFERENCES `account_revision`(`account_id`,`id`) ON DELETE RESTRICT,
	CONSTRAINT `fk_account_source_account_id_account_id_fk` FOREIGN KEY (`account_id`) REFERENCES `account`(`id`),
	CONSTRAINT "account_source_generation" CHECK("generation" > 0),
	CONSTRAINT "account_source_reference" CHECK("reference" LIKE 'refs/heads/_%' OR "reference" LIKE 'refs/tags/_%'),
	CONSTRAINT "account_source_directory" CHECK(length("directory") > 0 AND substr("directory", 1, 1) <> '/' AND "directory" <> '..' AND "directory" NOT LIKE '../%' AND "directory" NOT LIKE '%/../%' AND "directory" NOT LIKE '%/..'),
	CONSTRAINT "account_source_export" CHECK(length("export") > 0 AND ("entrypoint" = '.' OR "entrypoint" LIKE './_%')),
	CONSTRAINT "account_source_account_id_not_null" CHECK("account_id" IS NOT NULL)
);
--> statement-breakpoint
CREATE TABLE `account_revision` (
	`id` text PRIMARY KEY,
	`account_id` text NOT NULL,
	`source_generation` integer NOT NULL,
	`source` text NOT NULL,
	`parameters` text NOT NULL,
	`definition` text NOT NULL,
	`digest` text NOT NULL,
	`created_at` integer NOT NULL,
	CONSTRAINT `fk_account_revision_account_id_account_id_fk` FOREIGN KEY (`account_id`) REFERENCES `account`(`id`),
	CONSTRAINT `account_revision_account_id` UNIQUE(`account_id`,`id`),
	CONSTRAINT "account_revision_generation" CHECK("source_generation" > 0),
	CONSTRAINT "account_revision_digest" CHECK(length("digest") = 64 AND "digest" NOT GLOB '*[^a-f0-9]*'),
	CONSTRAINT "account_revision_id_not_null" CHECK("id" IS NOT NULL)
);
--> statement-breakpoint
CREATE TABLE `account_membership` (
	`id` text PRIMARY KEY,
	`created_at` integer NOT NULL,
	`updated_at` integer NOT NULL,
	`revision` integer DEFAULT 1 NOT NULL,
	`tags` text DEFAULT '{}' NOT NULL,
	`account_id` text NOT NULL,
	`user_id` text NOT NULL,
	CONSTRAINT `fk_account_membership_account_id_account_id_fk` FOREIGN KEY (`account_id`) REFERENCES `account`(`id`) ON DELETE CASCADE,
	CONSTRAINT `fk_account_membership_user_id_user_id_fk` FOREIGN KEY (`user_id`) REFERENCES `user`(`id`) ON DELETE RESTRICT,
	CONSTRAINT `account_membership_account_id` UNIQUE(`account_id`,`id`),
	CONSTRAINT `account_membership_account_user` UNIQUE(`account_id`,`user_id`),
	CONSTRAINT "account_membership_id_not_null" CHECK("id" IS NOT NULL)
);
--> statement-breakpoint
CREATE TABLE `account_invitation` (
	`id` text PRIMARY KEY,
	`created_at` integer NOT NULL,
	`updated_at` integer NOT NULL,
	`revision` integer DEFAULT 1 NOT NULL,
	`tags` text DEFAULT '{}' NOT NULL,
	`account_id` text NOT NULL,
	`invited_by` text NOT NULL,
	`email` text NOT NULL,
	`token_hash` text NOT NULL UNIQUE,
	`expires_at` integer NOT NULL,
	`accepted_by` text,
	`accepted_at` integer,
	`revoked_at` integer,
	CONSTRAINT `fk_account_invitation_account_id_account_id_fk` FOREIGN KEY (`account_id`) REFERENCES `account`(`id`),
	CONSTRAINT `fk_account_invitation_invited_by_user_id_fk` FOREIGN KEY (`invited_by`) REFERENCES `user`(`id`),
	CONSTRAINT `fk_account_invitation_accepted_by_user_id_fk` FOREIGN KEY (`accepted_by`) REFERENCES `user`(`id`),
	CONSTRAINT "account_invitation_expiry" CHECK("expires_at" > "created_at"),
	CONSTRAINT "account_invitation_acceptance" CHECK(("accepted_at" IS NULL) = ("accepted_by" IS NULL) AND ("accepted_at" IS NULL OR ("accepted_at" >= "created_at" AND "accepted_at" < "expires_at" AND "revoked_at" IS NULL))),
	CONSTRAINT "account_invitation_id_not_null" CHECK("id" IS NOT NULL)
);
--> statement-breakpoint
CREATE UNIQUE INDEX `role_provenance` ON `role` (json_extract("provenance", '$.kind'),coalesce(json_extract("provenance", '$.accountId'), json_extract("provenance", '$.spaceId'), json_extract("provenance", '$.installationId')),json_extract("provenance", '$.name')) WHERE "role"."provenance" IS NOT NULL AND "role"."detached_at" IS NULL;--> statement-breakpoint
CREATE UNIQUE INDEX `role_scope_name` ON `role` (`account_id`,`name`);--> statement-breakpoint
CREATE UNIQUE INDEX `role_binding_provenance` ON `role_binding` (json_extract("provenance", '$.kind'),coalesce(json_extract("provenance", '$.accountId'), json_extract("provenance", '$.spaceId'), json_extract("provenance", '$.installationId')),json_extract("provenance", '$.name')) WHERE "role_binding"."provenance" IS NOT NULL AND "role_binding"."detached_at" IS NULL;--> statement-breakpoint
CREATE UNIQUE INDEX `role_binding_active` ON `role_binding` (`role_id`,coalesce("space_id", ''),coalesce("account_membership_id", "group_id", "service_account_id")) WHERE "role_binding"."revoked_at" IS NULL;--> statement-breakpoint
CREATE UNIQUE INDEX `role_permission_scope` ON `role_permission` (`role_id`,`package_id`,`type`,`name`,coalesce("object_id", ''));--> statement-breakpoint
CREATE UNIQUE INDEX `preference_account` ON `preference` (`account_id`,`package_id`,`name`) WHERE "preference"."user_id" IS NULL AND "preference"."device_id" IS NULL;--> statement-breakpoint
CREATE UNIQUE INDEX `preference_user` ON `preference` (`account_id`,`user_id`,`package_id`,`name`) WHERE "preference"."user_id" IS NOT NULL AND "preference"."device_id" IS NULL;--> statement-breakpoint
CREATE UNIQUE INDEX `preference_device` ON `preference` (`account_id`,`user_id`,`device_id`,`package_id`,`name`) WHERE "preference"."device_id" IS NOT NULL;--> statement-breakpoint
CREATE UNIQUE INDEX `connection_oauth` ON `connected_account` (`account_id`,`provider`,`issuer`,`application_id`,`subject`) WHERE "connected_account"."kind" = 'oauth';--> statement-breakpoint
CREATE UNIQUE INDEX `connection_installation` ON `connected_account` (`account_id`,`provider`,`issuer`,`application_id`,`installation_id`) WHERE "connected_account"."kind" = 'installation';--> statement-breakpoint
CREATE INDEX `connection_user` ON `connected_account` (`user_id`);--> statement-breakpoint
CREATE UNIQUE INDEX `service_token_permission_scope` ON `service_token_permission` (`token_id`,coalesce("space_id", ''),`package_id`,`type`,`name`,coalesce("object_id", ''));--> statement-breakpoint
CREATE INDEX `organisation_membership_user_id` ON `organisation_membership` (`user_id`);--> statement-breakpoint
CREATE INDEX `personal_access_token_user` ON `personal_access_token` (`user_id`);--> statement-breakpoint
CREATE UNIQUE INDEX `personal_access_token_permission_scope` ON `personal_access_token_permission` (`token_id`,coalesce("space_id", ''),`package_id`,`type`,`name`,coalesce("object_id", ''));--> statement-breakpoint
CREATE INDEX `identity_user` ON `identity` (`user_id`);--> statement-breakpoint
CREATE INDEX `session_user` ON `session` (`user_id`);--> statement-breakpoint
CREATE INDEX `session_device` ON `session` (`device_id`);--> statement-breakpoint
CREATE INDEX `session_expiry` ON `session` (`expires_at`);--> statement-breakpoint
CREATE INDEX `passkey_user` ON `passkey` (`user_id`);--> statement-breakpoint
CREATE INDEX `two_factor_user` ON `two_factor` (`user_id`);--> statement-breakpoint
CREATE INDEX `two_factor_secret` ON `two_factor` (`secret`);--> statement-breakpoint
CREATE INDEX `device_authorization_expiry` ON `device_authorization` (`expires_at`);--> statement-breakpoint
CREATE INDEX `device_authorization_user` ON `device_authorization` (`user_id`);--> statement-breakpoint
CREATE INDEX `signing_key_created` ON `signing_key` (`created_at`);--> statement-breakpoint
CREATE INDEX `authentication_replay_expiry` ON `authentication_replay` (`expires_at`);--> statement-breakpoint
CREATE INDEX `oauth_client_user` ON `oauth_client` (`user_id`);--> statement-breakpoint
CREATE INDEX `oauth_client_account` ON `oauth_client` (`account_id`);--> statement-breakpoint
CREATE INDEX `organisation_invitation_organisation` ON `organisation_invitation` (`organisation_id`);--> statement-breakpoint
CREATE INDEX `oauth_client_resource_resource` ON `oauth_client_resource` (`resource_id`);--> statement-breakpoint
CREATE INDEX `oauth_consent_client` ON `oauth_consent` (`client_id`);--> statement-breakpoint
CREATE INDEX `oauth_consent_user` ON `oauth_consent` (`user_id`);--> statement-breakpoint
CREATE INDEX `oauth_access_client` ON `oauth_access_token` (`client_id`);--> statement-breakpoint
CREATE INDEX `oauth_access_user` ON `oauth_access_token` (`user_id`);--> statement-breakpoint
CREATE INDEX `oauth_access_session` ON `oauth_access_token` (`session_id`);--> statement-breakpoint
CREATE INDEX `oauth_access_refresh` ON `oauth_access_token` (`refresh_id`);--> statement-breakpoint
CREATE INDEX `oauth_access_code` ON `oauth_access_token` (`authorization_code_id`);--> statement-breakpoint
CREATE INDEX `oauth_access_expiry` ON `oauth_access_token` (`expires_at`);--> statement-breakpoint
CREATE INDEX `oauth_refresh_client` ON `oauth_refresh_token` (`client_id`);--> statement-breakpoint
CREATE INDEX `oauth_refresh_user` ON `oauth_refresh_token` (`user_id`);--> statement-breakpoint
CREATE INDEX `oauth_refresh_session` ON `oauth_refresh_token` (`session_id`);--> statement-breakpoint
CREATE INDEX `oauth_refresh_code` ON `oauth_refresh_token` (`authorization_code_id`);--> statement-breakpoint
CREATE INDEX `oauth_refresh_expiry` ON `oauth_refresh_token` (`expires_at`);--> statement-breakpoint
CREATE INDEX `oauth_client_assertion_expiry` ON `oauth_client_assertion` (`expires_at`);--> statement-breakpoint
CREATE INDEX `device_enrolled_by` ON `device` (`enrolled_by`);--> statement-breakpoint
CREATE INDEX `account_user` ON `account` (`user_id`);--> statement-breakpoint
CREATE INDEX `account_organisation` ON `account` (`organisation_id`);--> statement-breakpoint
CREATE INDEX `tunnel_host_expiry` ON `tunnel` (`host_id`,`expires_at`);--> statement-breakpoint
CREATE UNIQUE INDEX `route_provenance` ON `route` (json_extract("provenance", '$.kind'),coalesce(json_extract("provenance", '$.accountId'), json_extract("provenance", '$.spaceId'), json_extract("provenance", '$.installationId')),json_extract("provenance", '$.name')) WHERE "route"."provenance" IS NOT NULL AND "route"."detached_at" IS NULL;--> statement-breakpoint
CREATE UNIQUE INDEX `environment_provenance` ON `environment` (json_extract("provenance", '$.kind'),coalesce(json_extract("provenance", '$.accountId'), json_extract("provenance", '$.spaceId'), json_extract("provenance", '$.installationId')),json_extract("provenance", '$.name')) WHERE "environment"."provenance" IS NOT NULL AND "environment"."detached_at" IS NULL;--> statement-breakpoint
CREATE INDEX `account_membership_user` ON `account_membership` (`user_id`);