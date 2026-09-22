CREATE TABLE `vault_value` (
	`secret_id` text NOT NULL,
	`version` integer NOT NULL,
	`format` integer NOT NULL,
	`key_id` text NOT NULL,
	`ciphertext` text NOT NULL,
	`nonce` text NOT NULL,
	`wrapped_key` text NOT NULL,
	`key_nonce` text,
	CONSTRAINT `vault_value_pk` PRIMARY KEY(`secret_id`, `version`),
	CONSTRAINT `fk_vault_value_secret_id_version_secret_version_secret_id_version_fk` FOREIGN KEY (`secret_id`,`version`) REFERENCES `secret_version`(`secret_id`,`version`) ON DELETE RESTRICT
);
--> statement-breakpoint
CREATE TABLE `vault_request` (
	`caller` text NOT NULL,
	`scope` text NOT NULL,
	`procedure` text NOT NULL,
	`request_id` text NOT NULL,
	`digest` text NOT NULL,
	`key_id` text,
	`response` text,
	`created_at` integer NOT NULL,
	`expires_at` integer NOT NULL,
	CONSTRAINT `vault_request_pk` PRIMARY KEY(`caller`, `scope`, `procedure`, `request_id`)
);
--> statement-breakpoint
CREATE INDEX `vault_request_expiry` ON `vault_request` (`expires_at`);--> statement-breakpoint
CREATE INDEX `vault_request_key` ON `vault_request` (`key_id`,`scope`);