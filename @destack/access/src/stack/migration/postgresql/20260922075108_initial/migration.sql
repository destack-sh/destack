CREATE TABLE "access_grant" (
	"id" text PRIMARY KEY,
	"package_id" text NOT NULL,
	"type" text NOT NULL,
	"scope" text NOT NULL,
	"object_id" text NOT NULL,
	"relation" text NOT NULL,
	"subject_kind" text NOT NULL,
	"subject_authority" text NOT NULL,
	"subject_id" text NOT NULL,
	"created_at" bigint NOT NULL,
	"expires_at" bigint,
	"revoked_at" bigint,
	CONSTRAINT "access_grant_expiry" CHECK ("expires_at" IS NULL OR "expires_at" > "created_at"),
	CONSTRAINT "access_grant_subject" CHECK (("subject_kind" = 'everyone' AND "subject_authority" = '' AND "subject_id" = '') OR ("subject_kind" <> 'everyone' AND length("subject_authority") > 0 AND length("subject_id") > 0))
);
--> statement-breakpoint
CREATE TABLE "access_token" (
	"id" text PRIMARY KEY,
	"scope" text NOT NULL,
	"digest" text NOT NULL UNIQUE,
	"created_at" bigint NOT NULL,
	"expires_at" bigint,
	"revoked_at" bigint,
	CONSTRAINT "access_token_expiry" CHECK ("expires_at" IS NULL OR "expires_at" > "created_at")
);
--> statement-breakpoint
CREATE INDEX "access_grant_object" ON "access_grant" ("scope","package_id","type","object_id","relation");--> statement-breakpoint
CREATE INDEX "access_grant_subject" ON "access_grant" ("subject_kind","subject_authority","subject_id","scope");