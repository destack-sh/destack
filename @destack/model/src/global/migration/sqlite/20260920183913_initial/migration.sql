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
	`user_id` text UNIQUE,
	CONSTRAINT `fk_account_package_policy_region_id_region_id_fk` FOREIGN KEY (`package_policy_region_id`) REFERENCES `region`(`id`) ON DELETE RESTRICT,
	CONSTRAINT `fk_account_network_policy_region_id_region_id_fk` FOREIGN KEY (`network_policy_region_id`) REFERENCES `region`(`id`) ON DELETE RESTRICT,
	CONSTRAINT `fk_account_user_id_user_id_fk` FOREIGN KEY (`user_id`) REFERENCES `user`(`id`) ON DELETE RESTRICT,
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
	CONSTRAINT "account_user" CHECK(("kind" = 'personal' AND "user_id" IS NOT NULL) OR ("kind" = 'organisation' AND "user_id" IS NULL)),
	CONSTRAINT "account_handle" CHECK(length("handle") BETWEEN 1 AND 63 AND "handle" NOT GLOB '*[^a-z0-9-]*' AND "handle" NOT LIKE '-%' AND "handle" NOT LIKE '%-'),
	CONSTRAINT "account_id_not_null" CHECK("id" IS NOT NULL)
);
--> statement-breakpoint
CREATE TABLE `identity` (
	`id` text PRIMARY KEY,
	`created_at` integer NOT NULL,
	`updated_at` integer NOT NULL,
	`revision` integer DEFAULT 1 NOT NULL,
	`tags` text DEFAULT '{}' NOT NULL,
	`user_id` text NOT NULL,
	`issuer` text NOT NULL,
	`subject` text NOT NULL,
	CONSTRAINT `fk_identity_user_id_user_id_fk` FOREIGN KEY (`user_id`) REFERENCES `user`(`id`) ON DELETE CASCADE,
	CONSTRAINT `identity_issuer_subject` UNIQUE(`issuer`,`subject`),
	CONSTRAINT "identity_id_not_null" CHECK("id" IS NOT NULL)
);
--> statement-breakpoint
CREATE TABLE `role_permission` (
	`id` text PRIMARY KEY,
	`role_id` text NOT NULL,
	`resource` text NOT NULL,
	`action` text NOT NULL,
	`resource_id` text,
	CONSTRAINT `fk_role_permission_role_id_role_id_fk` FOREIGN KEY (`role_id`) REFERENCES `role`(`id`) ON DELETE CASCADE,
	CONSTRAINT "role_permission_resource" CHECK(length("resource") > 0 AND instr("resource", '.') > 0 AND instr("resource", '*') = 0),
	CONSTRAINT "role_permission_action" CHECK(length("action") > 0 AND instr("action", '*') = 0),
	CONSTRAINT "role_permission_identifier" CHECK("resource_id" IS NULL OR length("resource_id") > 0),
	CONSTRAINT "role_permission_id_not_null" CHECK("id" IS NOT NULL)
);
--> statement-breakpoint
CREATE TABLE `sign_in_request` (
	`id` text PRIMARY KEY,
	`created_at` integer NOT NULL,
	`updated_at` integer NOT NULL,
	`revision` integer DEFAULT 1 NOT NULL,
	`tags` text DEFAULT '{}' NOT NULL,
	`client` text NOT NULL,
	`token_hash` text NOT NULL UNIQUE,
	`code_hash` text NOT NULL UNIQUE,
	`challenge` text NOT NULL,
	`user_id` text,
	`expires_at` integer NOT NULL,
	`approved_at` integer,
	`denied_at` integer,
	`consumed_at` integer,
	CONSTRAINT `fk_sign_in_request_user_id_user_id_fk` FOREIGN KEY (`user_id`) REFERENCES `user`(`id`),
	CONSTRAINT "sign_in_request_client" CHECK("client" IN ('desktop', 'cli')),
	CONSTRAINT "sign_in_request_expiry" CHECK("expires_at" > "created_at"),
	CONSTRAINT "sign_in_request_approval" CHECK(("approved_at" IS NULL) = ("user_id" IS NULL) AND ("approved_at" IS NULL OR ("denied_at" IS NULL AND "approved_at" >= "created_at" AND "approved_at" < "expires_at"))),
	CONSTRAINT "sign_in_request_exchange" CHECK("consumed_at" IS NULL OR ("approved_at" IS NOT NULL AND "consumed_at" >= "approved_at" AND "consumed_at" < "expires_at")),
	CONSTRAINT "sign_in_request_id_not_null" CHECK("id" IS NOT NULL)
);
--> statement-breakpoint
CREATE TABLE `user` (
	`id` text PRIMARY KEY,
	`created_at` integer NOT NULL,
	`updated_at` integer NOT NULL,
	`revision` integer DEFAULT 1 NOT NULL,
	`tags` text DEFAULT '{}' NOT NULL,
	`name` text NOT NULL,
	CONSTRAINT "user_id_not_null" CHECK("id" IS NOT NULL)
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
	`service_account_id` text NOT NULL,
	`name` text NOT NULL,
	`token_hash` text NOT NULL UNIQUE,
	`expires_at` integer NOT NULL,
	`revoked_at` integer,
	CONSTRAINT `fk_service_token_service_account_id_service_account_id_fk` FOREIGN KEY (`service_account_id`) REFERENCES `service_account`(`id`),
	CONSTRAINT "service_token_expiry" CHECK("expires_at" > "created_at"),
	CONSTRAINT "service_token_id_not_null" CHECK("id" IS NOT NULL)
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
CREATE TABLE `device_key` (
	`id` text PRIMARY KEY,
	`created_at` integer NOT NULL,
	`updated_at` integer NOT NULL,
	`revision` integer DEFAULT 1 NOT NULL,
	`tags` text DEFAULT '{}' NOT NULL,
	`device_id` text NOT NULL,
	`public_key` text NOT NULL UNIQUE,
	`expires_at` integer NOT NULL,
	`revoked_at` integer,
	CONSTRAINT `fk_device_key_device_id_device_id_fk` FOREIGN KEY (`device_id`) REFERENCES `device`(`id`),
	CONSTRAINT `device_key_device_id` UNIQUE(`device_id`,`id`),
	CONSTRAINT "device_key_expiry" CHECK("expires_at" > "created_at"),
	CONSTRAINT "device_key_id_not_null" CHECK("id" IS NOT NULL)
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
CREATE TABLE `device` (
	`id` text PRIMARY KEY,
	`created_at` integer NOT NULL,
	`updated_at` integer NOT NULL,
	`revision` integer DEFAULT 1 NOT NULL,
	`tags` text DEFAULT '{}' NOT NULL,
	`account_id` text NOT NULL,
	`name` text NOT NULL,
	`revoked_at` integer,
	`last_seen_at` integer,
	CONSTRAINT `fk_device_account_id_account_id_fk` FOREIGN KEY (`account_id`) REFERENCES `account`(`id`) ON DELETE RESTRICT,
	CONSTRAINT `device_account_id` UNIQUE(`account_id`,`id`),
	CONSTRAINT "device_id_not_null" CHECK("id" IS NOT NULL)
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
CREATE TABLE `route` (
	`id` text PRIMARY KEY,
	`created_at` integer NOT NULL,
	`updated_at` integer NOT NULL,
	`revision` integer DEFAULT 1 NOT NULL,
	`tags` text DEFAULT '{}' NOT NULL,
	`source` text,
	`source_detached_at` integer,
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
	CONSTRAINT `fk_route_space_id_space_directory_id_fk` FOREIGN KEY (`space_id`) REFERENCES `space_directory`(`id`) ON DELETE RESTRICT,
	CONSTRAINT `route_domain_path_match` UNIQUE(`domain_id`,`path`,`match`),
	CONSTRAINT "route_source_detached" CHECK("source_detached_at" IS NULL OR ("source" IS NOT NULL AND "source_detached_at" >= 0)),
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
CREATE TABLE `space_directory` (
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
	CONSTRAINT `fk_space_directory_region_id_residency_region_id_residency_fk` FOREIGN KEY (`region_id`,`residency`) REFERENCES `region`(`id`,`residency`) ON DELETE RESTRICT,
	CONSTRAINT `fk_space_directory_account_id_account_id_fk` FOREIGN KEY (`account_id`) REFERENCES `account`(`id`) ON DELETE RESTRICT,
	CONSTRAINT `space_directory_name` UNIQUE(`account_id`,`name`),
	CONSTRAINT `space_directory_account` UNIQUE(`account_id`,`id`),
	CONSTRAINT "space_directory_revision" CHECK("revision" >= 1),
	CONSTRAINT "space_directory_generation" CHECK("generation" >= 1),
	CONSTRAINT "space_directory_observed_generation" CHECK("observed_generation" BETWEEN 0 AND "generation"),
	CONSTRAINT "space_directory_name_value" CHECK(length("name") BETWEEN 1 AND 63 AND "name" NOT GLOB '*[^a-z0-9-]*' AND "name" NOT LIKE '-%' AND "name" NOT LIKE '%-'),
	CONSTRAINT "space_directory_id_not_null" CHECK("id" IS NOT NULL)
);
--> statement-breakpoint
CREATE TABLE `repository_directory` (
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
	CONSTRAINT `fk_repository_directory_region_id_residency_region_id_residency_fk` FOREIGN KEY (`region_id`,`residency`) REFERENCES `region`(`id`,`residency`) ON DELETE RESTRICT,
	CONSTRAINT `fk_repository_directory_account_id_account_id_fk` FOREIGN KEY (`account_id`) REFERENCES `account`(`id`) ON DELETE RESTRICT,
	CONSTRAINT `repository_directory_name` UNIQUE(`account_id`,`name`),
	CONSTRAINT `repository_directory_account` UNIQUE(`account_id`,`id`),
	CONSTRAINT "repository_directory_revision" CHECK("revision" >= 1),
	CONSTRAINT "repository_directory_generation" CHECK("generation" >= 1),
	CONSTRAINT "repository_directory_observed_generation" CHECK("observed_generation" BETWEEN 0 AND "generation"),
	CONSTRAINT "repository_directory_name_value" CHECK(length("name") BETWEEN 1 AND 63 AND "name" NOT GLOB '*[^a-z0-9-]*' AND "name" NOT LIKE '-%' AND "name" NOT LIKE '%-'),
	CONSTRAINT "repository_directory_id_not_null" CHECK("id" IS NOT NULL)
);
--> statement-breakpoint
CREATE TABLE `package_directory` (
	`id` text PRIMARY KEY,
	`created_at` integer NOT NULL,
	`updated_at` integer NOT NULL,
	`revision` integer DEFAULT 1 NOT NULL,
	`tags` text DEFAULT '{}' NOT NULL,
	`account_id` text NOT NULL,
	`name` text NOT NULL,
	`repository_id` text NOT NULL,
	CONSTRAINT `fk_package_directory_account_id_repository_id_repository_directory_account_id_id_fk` FOREIGN KEY (`account_id`,`repository_id`) REFERENCES `repository_directory`(`account_id`,`id`) ON DELETE RESTRICT,
	CONSTRAINT `package_directory_name` UNIQUE(`account_id`,`name`),
	CONSTRAINT "package_directory_id_not_null" CHECK("id" IS NOT NULL)
);
--> statement-breakpoint
CREATE TABLE `role_binding` (
	`id` text PRIMARY KEY,
	`created_at` integer NOT NULL,
	`updated_at` integer NOT NULL,
	`revision` integer DEFAULT 1 NOT NULL,
	`tags` text DEFAULT '{}' NOT NULL,
	`source` text,
	`source_detached_at` integer,
	`account_id` text NOT NULL,
	`role_id` text NOT NULL,
	`space_id` text,
	`account_membership_id` text,
	`group_id` text,
	`service_account_id` text,
	`expires_at` integer,
	`revoked_at` integer,
	CONSTRAINT `fk_role_binding_account_id_space_id_space_directory_account_id_id_fk` FOREIGN KEY (`account_id`,`space_id`) REFERENCES `space_directory`(`account_id`,`id`) ON DELETE RESTRICT,
	CONSTRAINT `fk_role_binding_account_id_role_id_role_account_id_id_fk` FOREIGN KEY (`account_id`,`role_id`) REFERENCES `role`(`account_id`,`id`),
	CONSTRAINT `fk_role_binding_account_id_account_membership_id_account_membership_account_id_id_fk` FOREIGN KEY (`account_id`,`account_membership_id`) REFERENCES `account_membership`(`account_id`,`id`),
	CONSTRAINT `fk_role_binding_account_id_group_id_group_account_id_id_fk` FOREIGN KEY (`account_id`,`group_id`) REFERENCES `group`(`account_id`,`id`),
	CONSTRAINT `fk_role_binding_account_id_service_account_id_service_account_account_id_id_fk` FOREIGN KEY (`account_id`,`service_account_id`) REFERENCES `service_account`(`account_id`,`id`),
	CONSTRAINT "role_binding_source_detached" CHECK("source_detached_at" IS NULL OR ("source" IS NOT NULL AND "source_detached_at" >= 0)),
	CONSTRAINT "role_binding_subject" CHECK(CAST("account_membership_id" IS NOT NULL AS integer) + CAST("group_id" IS NOT NULL AS integer) + CAST("service_account_id" IS NOT NULL AS integer) = 1),
	CONSTRAINT "role_binding_expiry" CHECK("expires_at" IS NULL OR "expires_at" > "created_at"),
	CONSTRAINT "role_binding_id_not_null" CHECK("id" IS NOT NULL)
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
	CONSTRAINT `fk_connected_account_account_id_secret_space_id_space_directory_account_id_id_fk` FOREIGN KEY (`account_id`,`secret_space_id`) REFERENCES `space_directory`(`account_id`,`id`) ON DELETE RESTRICT,
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
CREATE TABLE `role` (
	`id` text PRIMARY KEY,
	`created_at` integer NOT NULL,
	`updated_at` integer NOT NULL,
	`revision` integer DEFAULT 1 NOT NULL,
	`tags` text DEFAULT '{}' NOT NULL,
	`source` text,
	`source_detached_at` integer,
	`account_id` text NOT NULL,
	`name` text NOT NULL,
	`description` text NOT NULL,
	CONSTRAINT `fk_role_account_id_account_id_fk` FOREIGN KEY (`account_id`) REFERENCES `account`(`id`),
	CONSTRAINT `role_account_id` UNIQUE(`account_id`,`id`),
	CONSTRAINT "role_source_detached" CHECK("source_detached_at" IS NULL OR ("source" IS NOT NULL AND "source_detached_at" >= 0)),
	CONSTRAINT "role_id_not_null" CHECK("id" IS NOT NULL)
);
--> statement-breakpoint
CREATE TABLE `session` (
	`id` text PRIMARY KEY,
	`user_id` text NOT NULL,
	`device_id` text,
	`token_hash` text NOT NULL UNIQUE,
	`created_at` integer NOT NULL,
	`expires_at` integer NOT NULL,
	`revoked_at` integer,
	CONSTRAINT `fk_session_user_id_user_id_fk` FOREIGN KEY (`user_id`) REFERENCES `user`(`id`) ON DELETE CASCADE,
	CONSTRAINT `fk_session_device_id_device_id_fk` FOREIGN KEY (`device_id`) REFERENCES `device`(`id`) ON DELETE RESTRICT,
	CONSTRAINT "session_expiry_order" CHECK("expires_at" > "created_at"),
	CONSTRAINT "session_id_not_null" CHECK("id" IS NOT NULL)
);
--> statement-breakpoint
CREATE INDEX `identity_user` ON `identity` (`user_id`);--> statement-breakpoint
CREATE UNIQUE INDEX `role_permission_object` ON `role_permission` (`role_id`,`resource`,`action`,`resource_id`) WHERE "role_permission"."resource_id" IS NOT NULL;--> statement-breakpoint
CREATE UNIQUE INDEX `role_permission_all` ON `role_permission` (`role_id`,`resource`,`action`) WHERE "role_permission"."resource_id" IS NULL;--> statement-breakpoint
CREATE UNIQUE INDEX `preference_account` ON `preference` (`account_id`,`name`) WHERE "preference"."user_id" IS NULL AND "preference"."device_id" IS NULL;--> statement-breakpoint
CREATE UNIQUE INDEX `preference_user` ON `preference` (`account_id`,`user_id`,`name`) WHERE "preference"."user_id" IS NOT NULL AND "preference"."device_id" IS NULL;--> statement-breakpoint
CREATE UNIQUE INDEX `preference_device` ON `preference` (`account_id`,`user_id`,`device_id`,`name`) WHERE "preference"."device_id" IS NOT NULL;--> statement-breakpoint
CREATE INDEX `tunnel_host_expiry` ON `tunnel` (`host_id`,`expires_at`);--> statement-breakpoint
CREATE INDEX `account_membership_user` ON `account_membership` (`user_id`);--> statement-breakpoint
CREATE UNIQUE INDEX `route_source` ON `route` (json_extract("source", '$.kind'),coalesce(json_extract("source", '$.spaceId'), json_extract("source", '$.installationId')),json_extract("source", '$.name')) WHERE "route"."source" IS NOT NULL AND "route"."source_detached_at" IS NULL;--> statement-breakpoint
CREATE UNIQUE INDEX `role_binding_source` ON `role_binding` (json_extract("source", '$.kind'),coalesce(json_extract("source", '$.spaceId'), json_extract("source", '$.installationId')),json_extract("source", '$.name')) WHERE "role_binding"."source" IS NOT NULL AND "role_binding"."source_detached_at" IS NULL;--> statement-breakpoint
CREATE UNIQUE INDEX `role_binding_active` ON `role_binding` (`role_id`,coalesce("space_id", ''),coalesce("account_membership_id", "group_id", "service_account_id")) WHERE "role_binding"."revoked_at" IS NULL;--> statement-breakpoint
CREATE UNIQUE INDEX `connection_oauth` ON `connected_account` (`account_id`,`provider`,`issuer`,`application_id`,`subject`) WHERE "connected_account"."kind" = 'oauth';--> statement-breakpoint
CREATE UNIQUE INDEX `connection_installation` ON `connected_account` (`account_id`,`provider`,`issuer`,`application_id`,`installation_id`) WHERE "connected_account"."kind" = 'installation';--> statement-breakpoint
CREATE INDEX `connection_user` ON `connected_account` (`user_id`);--> statement-breakpoint
CREATE UNIQUE INDEX `role_source` ON `role` (json_extract("source", '$.kind'),coalesce(json_extract("source", '$.spaceId'), json_extract("source", '$.installationId')),json_extract("source", '$.name')) WHERE "role"."source" IS NOT NULL AND "role"."source_detached_at" IS NULL;--> statement-breakpoint
CREATE UNIQUE INDEX `role_scope_name` ON `role` (`account_id`,`name`);--> statement-breakpoint
CREATE INDEX `session_user` ON `session` (`user_id`);--> statement-breakpoint
CREATE INDEX `session_expiry` ON `session` (`expires_at`);