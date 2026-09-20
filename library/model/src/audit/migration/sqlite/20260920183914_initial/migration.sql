CREATE TABLE `audit_event` (
	`id` text PRIMARY KEY,
	`account_id` text,
	`space_id` text,
	`action` text NOT NULL,
	`version` integer NOT NULL,
	`occurred_at` integer NOT NULL,
	`recorded_at` integer NOT NULL,
	`actor` text NOT NULL,
	`delegation` text DEFAULT '[]' NOT NULL,
	`targets` text NOT NULL,
	`outcome` text NOT NULL,
	`error_code` text,
	`service` text NOT NULL,
	`deployment_id` text,
	`device_id` text,
	`session_id` text,
	`service_token_id` text,
	`request_id` text,
	`trace_id` text,
	`address` text,
	`user_agent` text,
	`details` text NOT NULL,
	CONSTRAINT "audit_version" CHECK("version" > 0),
	CONSTRAINT "audit_action" CHECK(length("action") > 0 AND instr("action", '.') > 0),
	CONSTRAINT "audit_outcome" CHECK("outcome" IN ('success', 'failure', 'denied')),
	CONSTRAINT "audit_error" CHECK("outcome" <> 'success' OR "error_code" IS NULL),
	CONSTRAINT "audit_space_account" CHECK("space_id" IS NULL OR "account_id" IS NOT NULL),
	CONSTRAINT "audit_event_id_not_null" CHECK("id" IS NOT NULL)
);
--> statement-breakpoint
CREATE INDEX `audit_account_time` ON `audit_event` (`account_id`,`occurred_at`,`id`);--> statement-breakpoint
CREATE INDEX `audit_space_time` ON `audit_event` (`space_id`,`occurred_at`,`id`);--> statement-breakpoint
CREATE INDEX `audit_request` ON `audit_event` (`request_id`);