PRAGMA foreign_keys=OFF;--> statement-breakpoint
CREATE TABLE `__new_deployment` (
	`id` text PRIMARY KEY,
	`created_at` integer NOT NULL,
	`updated_at` integer NOT NULL,
	`revision` integer DEFAULT 1 NOT NULL,
	`tags` text DEFAULT '{}' NOT NULL,
	`space_id` text NOT NULL,
	`installation_id` text NOT NULL,
	`generation` integer NOT NULL,
	`package_id` text NOT NULL,
	`version` text NOT NULL,
	`manifest` text NOT NULL,
	`output` text NOT NULL,
	`workload` text NOT NULL,
	`runtime` text NOT NULL,
	`service_account_id` text NOT NULL,
	`description` text NOT NULL,
	`policies` text NOT NULL,
	`host_id` text,
	`state` text DEFAULT 'prepared' NOT NULL,
	`conditions` text DEFAULT '{}' NOT NULL,
	`activated_at` integer,
	`retired_at` integer,
	CONSTRAINT `fk_deployment_space_id_installation_id_package_id_installation_space_id_id_package_id_fk` FOREIGN KEY (`space_id`,`installation_id`,`package_id`) REFERENCES `installation`(`space_id`,`id`,`package_id`) ON DELETE RESTRICT,
	CONSTRAINT `fk_deployment_space_id_host_id_space_host_space_id_host_id_fk` FOREIGN KEY (`space_id`,`host_id`) REFERENCES `space_host`(`space_id`,`host_id`) ON DELETE RESTRICT,
	CONSTRAINT `fk_deployment_installation_id_workload_service_account_id_service_account_installation_id_workload_id_fk` FOREIGN KEY (`installation_id`,`workload`,`service_account_id`) REFERENCES `service_account`(`installation_id`,`workload`,`id`) ON DELETE RESTRICT,
	CONSTRAINT `deployment_space_id` UNIQUE(`space_id`,`id`),
	CONSTRAINT `deployment_service_account` UNIQUE(`id`,`service_account_id`),
	CONSTRAINT `deployment_generation` UNIQUE(`installation_id`,`generation`,`output`,`workload`),
	CONSTRAINT "deployment_runtime" CHECK("runtime" IN ('bun', 'workerd')),
	CONSTRAINT "deployment_workload" CHECK(length("workload") > 0),
	CONSTRAINT "deployment_generation_positive" CHECK("generation" > 0),
	CONSTRAINT "deployment_state" CHECK("state" IN ('prepared', 'active', 'draining', 'retired')),
	CONSTRAINT "deployment_times" CHECK("retired_at" IS NULL OR "activated_at" IS NULL OR "retired_at" >= "activated_at"),
	CONSTRAINT "deployment_id_not_null" CHECK("id" IS NOT NULL)
);
--> statement-breakpoint
INSERT INTO `__new_deployment`(`id`, `created_at`, `updated_at`, `revision`, `tags`, `space_id`, `installation_id`, `generation`, `package_id`, `version`, `manifest`, `output`, `workload`, `runtime`, `service_account_id`, `description`, `policies`, `host_id`, `state`, `conditions`, `activated_at`, `retired_at`) SELECT `id`, `created_at`, `updated_at`, `revision`, `tags`, `space_id`, `installation_id`, `generation`, `package_id`, `version`, `manifest`, `output`, `workload`, `runtime`, `service_account_id`, `description`, `policies`, `host_id`, `state`, `conditions`, `activated_at`, `retired_at` FROM `deployment`;--> statement-breakpoint
DROP TABLE `deployment`;--> statement-breakpoint
ALTER TABLE `__new_deployment` RENAME TO `deployment`;--> statement-breakpoint
PRAGMA foreign_keys=ON;--> statement-breakpoint
CREATE INDEX `deployment_installation_state` ON `deployment` (`installation_id`,`state`);