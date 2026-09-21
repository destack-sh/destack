CREATE TABLE `audit_outbox` (
	`id` text PRIMARY KEY,
	`event` text NOT NULL,
	`content` text NOT NULL,
	`recorded_at` integer NOT NULL,
	CONSTRAINT "audit_outbox_id_not_null" CHECK("id" IS NOT NULL)
);
--> statement-breakpoint
CREATE TABLE `audit_sender` (
	`name` text PRIMARY KEY,
	`id` text NOT NULL,
	`sequence` integer NOT NULL,
	`event_id` text,
	CONSTRAINT `fk_audit_sender_event_id_audit_outbox_id_fk` FOREIGN KEY (`event_id`) REFERENCES `audit_outbox`(`id`),
	CONSTRAINT "audit_sender_name_not_null" CHECK("name" IS NOT NULL)
);
--> statement-breakpoint
CREATE INDEX `audit_outbox_order` ON `audit_outbox` (`recorded_at`,`id`);--> statement-breakpoint
CREATE UNIQUE INDEX `audit_sender_id` ON `audit_sender` (`id`);