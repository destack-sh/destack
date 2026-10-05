import { through } from "@destack/access";
import type { Select } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { space } from "@destack/space/object";
import { message, MessageError } from "./message.ts";

/** What a provider made of one try: sent, refused for now, or refused for good. */
export const ATTEMPT_OUTCOMES = ["sent", "retry", "failed"] as const;

/** One try of a provider at sending a message, after Svix's message attempts. */
export const messageAttempt = defineObject({
    name: "attempt",
    identity: "message-attempt",
    plural: "attempts",
    scope: space,
    nested: { in: message, delete: "cascade", receive: "send" },
    fields: {
        /** What the provider made of the try. */
        outcome: field.enum(ATTEMPT_OUTCOMES),
        /** Why the provider refused it, absent once sent. */
        error: field.json(MessageError).optional(),
    },
    permissions: { read: through("parent", "read") },
    methods: (method) => ({
        get: method.get("read"),
        list: method.list("read"),
        /** Record a provider's try as the system. */
        record: method.create(null, { isSystem: true, fields: ["outcome", "error"] }),
    }),
});
/** A message attempt as its table stores it. */
export type MessageAttempt = Select<typeof messageAttempt.table>;
