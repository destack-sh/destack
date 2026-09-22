CREATE TABLE "vault_value" (
	"secret_id" text,
	"version" bigint,
	"format" bigint NOT NULL,
	"key_id" text NOT NULL,
	"ciphertext" text NOT NULL,
	"nonce" text NOT NULL,
	"wrapped_key" text NOT NULL,
	"key_nonce" text,
	CONSTRAINT "vault_value_pkey" PRIMARY KEY("secret_id","version")
);
--> statement-breakpoint
CREATE TABLE "vault_request" (
	"caller" text,
	"scope" text,
	"procedure" text,
	"request_id" text,
	"digest" jsonb NOT NULL,
	"key_id" text,
	"response" jsonb,
	"created_at" bigint NOT NULL,
	"expires_at" bigint NOT NULL,
	CONSTRAINT "vault_request_pkey" PRIMARY KEY("caller","scope","procedure","request_id")
);
--> statement-breakpoint
CREATE INDEX "vault_request_expiry" ON "vault_request" ("expires_at");--> statement-breakpoint
CREATE INDEX "vault_request_key" ON "vault_request" ("key_id","scope");--> statement-breakpoint
ALTER TABLE "vault_value" ADD CONSTRAINT "vault_value_jMdJN3WV29x2_fkey" FOREIGN KEY ("secret_id","version") REFERENCES "secret_version"("secret_id","version") ON DELETE RESTRICT;