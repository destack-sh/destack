CREATE TABLE `audit_event` (
	`id` text PRIMARY KEY,
	`attempt_id` text,
	`account_id` text,
	`space_id` text,
	`host_id` text,
	`action` text NOT NULL,
	`package_id` text NOT NULL,
	`actor` text NOT NULL,
	`stage` text NOT NULL,
	`outcome` text,
	`occurred_at` integer NOT NULL,
	`recorded_at` integer NOT NULL,
	`event` text NOT NULL,
	CONSTRAINT "audit_event_id_not_null" CHECK("id" IS NOT NULL)
);
--> statement-breakpoint
CREATE TABLE `audit_target` (
	`event_id` text NOT NULL,
	`role` text NOT NULL,
	`type` text NOT NULL,
	`id` text NOT NULL,
	CONSTRAINT `fk_audit_target_event_id_audit_event_id_fk` FOREIGN KEY (`event_id`) REFERENCES `audit_event`(`id`) ON DELETE CASCADE
);
--> statement-breakpoint
CREATE TABLE `audit_producer` (
	`id` text PRIMARY KEY,
	`sequence` integer NOT NULL,
	`digest` text NOT NULL,
	`retired_at` integer,
	CONSTRAINT "audit_producer_id_not_null" CHECK("id" IS NOT NULL)
);
--> statement-breakpoint
CREATE UNIQUE INDEX `audit_attempt_result` ON `audit_event` (`attempt_id`);--> statement-breakpoint
CREATE INDEX `audit_account_time` ON `audit_event` (`account_id`,`recorded_at`,`id`);--> statement-breakpoint
CREATE INDEX `audit_space_time` ON `audit_event` (`space_id`,`recorded_at`,`id`);--> statement-breakpoint
CREATE INDEX `audit_host_time` ON `audit_event` (`host_id`,`recorded_at`,`id`);--> statement-breakpoint
CREATE INDEX `audit_action_time` ON `audit_event` (`action`,`recorded_at`,`id`);--> statement-breakpoint
CREATE INDEX `audit_package_time` ON `audit_event` (`package_id`,`recorded_at`,`id`);--> statement-breakpoint
CREATE INDEX `audit_actor_time` ON `audit_event` (`actor`,`recorded_at`,`id`);--> statement-breakpoint
CREATE UNIQUE INDEX `audit_target_role` ON `audit_target` (`event_id`,`role`);--> statement-breakpoint
CREATE INDEX `audit_target_object` ON `audit_target` (`type`,`id`,`event_id`);