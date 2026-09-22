CREATE TABLE "audit_outbox" (
	"id" text PRIMARY KEY,
	"event" jsonb NOT NULL,
	"content" text NOT NULL,
	"recorded_at" bigint NOT NULL
);
--> statement-breakpoint
CREATE TABLE "audit_sender" (
	"name" text PRIMARY KEY,
	"id" text NOT NULL,
	"sequence" bigint NOT NULL,
	"event_id" text
);
--> statement-breakpoint
CREATE INDEX "audit_outbox_order" ON "audit_outbox" ("recorded_at","id");--> statement-breakpoint
CREATE UNIQUE INDEX "audit_sender_id" ON "audit_sender" ("id");--> statement-breakpoint
ALTER TABLE "audit_sender" ADD CONSTRAINT "audit_sender_event_id_audit_outbox_id_fkey" FOREIGN KEY ("event_id") REFERENCES "audit_outbox"("id");