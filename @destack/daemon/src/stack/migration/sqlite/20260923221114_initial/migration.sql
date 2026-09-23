CREATE TABLE `host` (
	`id` integer PRIMARY KEY,
	`device_id` text UNIQUE,
	`host_id` text NOT NULL UNIQUE,
	`enrollment` text,
	`credential` text,
	`execution` text DEFAULT 'disabled' NOT NULL,
	`name` text NOT NULL,
	`created_at` integer NOT NULL,
	CONSTRAINT "host_singleton" CHECK("id" = 1)
);
--> statement-breakpoint
CREATE TABLE `checkout` (
	`id` text PRIMARY KEY,
	`repository_id` text,
	`path` text NOT NULL UNIQUE,
	`created_at` integer NOT NULL,
	CONSTRAINT "checkout_id_not_null" CHECK("id" IS NOT NULL)
);
--> statement-breakpoint
CREATE TABLE `login` (
	`id` text PRIMARY KEY,
	`issuer` text NOT NULL,
	`session_id` text,
	`subject` text NOT NULL,
	`name` text NOT NULL,
	`credential` text NOT NULL,
	`status` text NOT NULL,
	`verified_at` integer,
	CONSTRAINT "login_id_not_null" CHECK("id" IS NOT NULL)
);
--> statement-breakpoint
CREATE TABLE `deployment` (
	`id` text PRIMARY KEY,
	`space_id` text NOT NULL,
	`generation` integer NOT NULL,
	`host_epoch` integer NOT NULL,
	`authorized_until` integer NOT NULL,
	`status` text NOT NULL,
	`instances` integer NOT NULL,
	`restart` text NOT NULL,
	`verified_at` integer NOT NULL,
	CONSTRAINT "deployment_id_not_null" CHECK("id" IS NOT NULL)
);
--> statement-breakpoint
CREATE TABLE `instance` (
	`id` text PRIMARY KEY,
	`deployment_id` text NOT NULL,
	`host_epoch` integer NOT NULL,
	`status` text NOT NULL,
	`observed_at` integer NOT NULL,
	`started_at` integer,
	`stopped_at` integer,
	`error` text,
	`conditions` text NOT NULL,
	CONSTRAINT `fk_instance_deployment_id_deployment_id_fk` FOREIGN KEY (`deployment_id`) REFERENCES `deployment`(`id`) ON DELETE RESTRICT,
	CONSTRAINT "instance_id_not_null" CHECK("id" IS NOT NULL)
);
