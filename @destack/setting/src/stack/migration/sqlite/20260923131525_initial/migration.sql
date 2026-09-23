CREATE TABLE `setting_assignment` (
	`id` text PRIMARY KEY,
	`package_id` text NOT NULL,
	`scope` text NOT NULL,
	`user_authority` text,
	`user_id` text,
	`space_id` text,
	`host_id` text,
	`consumer_package_id` text,
	`installation_id` text,
	`device_id` text,
	`name` text NOT NULL,
	`value` text NOT NULL,
	`revision` text NOT NULL,
	`provenance` text,
	`source` text,
	`detached_at` integer,
	`created_at` integer NOT NULL,
	`updated_at` integer NOT NULL,
	CONSTRAINT "setting_assignment_scope" CHECK((
            ("scope" = 'user' AND "user_authority" IS NOT NULL AND "user_id" IS NOT NULL AND "host_id" IS NULL)
            OR ("scope" = 'space' AND "space_id" IS NOT NULL AND "user_authority" IS NULL AND "user_id" IS NULL AND "host_id" IS NULL AND "consumer_package_id" IS NULL AND "device_id" IS NULL)
            OR ("scope" = 'host' AND "host_id" IS NOT NULL AND "user_authority" IS NULL AND "user_id" IS NULL AND "space_id" IS NULL AND "installation_id" IS NULL AND "consumer_package_id" IS NULL AND "device_id" IS NULL)
        ) AND ("installation_id" IS NULL OR "space_id" IS NOT NULL)),
	CONSTRAINT "setting_assignment_id_not_null" CHECK("id" IS NOT NULL)
);
--> statement-breakpoint
CREATE TABLE `setting_policy` (
	`id` text PRIMARY KEY,
	`authority` text NOT NULL,
	`account_id` text,
	`space_id` text,
	`host_id` text,
	`setting` text NOT NULL,
	`installation_id` text,
	`mode` text NOT NULL,
	`value` text NOT NULL,
	`revision` text NOT NULL,
	`provenance` text,
	`source` text,
	`detached_at` integer,
	`created_at` integer NOT NULL,
	`updated_at` integer NOT NULL,
	CONSTRAINT "setting_policy_authority" CHECK(
            ("authority" = 'account' AND "account_id" IS NOT NULL AND "space_id" IS NULL AND "host_id" IS NULL)
            OR ("authority" = 'space' AND "space_id" IS NOT NULL AND "account_id" IS NULL AND "host_id" IS NULL)
            OR ("authority" = 'host' AND "host_id" IS NOT NULL AND "account_id" IS NULL AND "space_id" IS NULL)),
	CONSTRAINT "setting_policy_id_not_null" CHECK("id" IS NOT NULL)
);
--> statement-breakpoint
CREATE TABLE `setting_request` (
	`caller` text NOT NULL,
	`scope` text NOT NULL,
	`procedure` text NOT NULL,
	`request_id` text NOT NULL,
	`digest` text NOT NULL,
	`key_id` text,
	`response` text,
	`created_at` integer NOT NULL,
	`expires_at` integer NOT NULL,
	CONSTRAINT `setting_request_pk` PRIMARY KEY(`caller`, `scope`, `procedure`, `request_id`)
);
--> statement-breakpoint
CREATE TABLE `setting_source` (
	`key` text PRIMARY KEY,
	`revision` integer NOT NULL,
	CONSTRAINT "setting_source_key_not_null" CHECK("key" IS NOT NULL)
);
--> statement-breakpoint
CREATE INDEX `setting_assignment_user` ON `setting_assignment` (`user_authority`,`user_id`,`package_id`,`name`);--> statement-breakpoint
CREATE INDEX `setting_assignment_space` ON `setting_assignment` (`space_id`,`package_id`,`name`);--> statement-breakpoint
CREATE INDEX `setting_assignment_host` ON `setting_assignment` (`host_id`,`package_id`,`name`);--> statement-breakpoint
CREATE UNIQUE INDEX `setting_assignment_target` ON `setting_assignment` (`package_id`,`name`,`scope`,coalesce("user_authority", ''),coalesce("user_id", ''),coalesce("space_id", ''),coalesce("host_id", ''),coalesce("consumer_package_id", ''),coalesce("installation_id", ''),coalesce("device_id", ''));--> statement-breakpoint
CREATE INDEX `setting_assignment_source` ON `setting_assignment` (`source`);--> statement-breakpoint
CREATE INDEX `setting_policy_account` ON `setting_policy` (`account_id`,`id`);--> statement-breakpoint
CREATE INDEX `setting_policy_space` ON `setting_policy` (`space_id`,`id`);--> statement-breakpoint
CREATE INDEX `setting_policy_host` ON `setting_policy` (`host_id`,`id`);--> statement-breakpoint
CREATE INDEX `setting_policy_source` ON `setting_policy` (`source`);--> statement-breakpoint
CREATE INDEX `setting_request_expiry` ON `setting_request` (`expires_at`);--> statement-breakpoint
CREATE INDEX `setting_request_key` ON `setting_request` (`key_id`,`scope`);