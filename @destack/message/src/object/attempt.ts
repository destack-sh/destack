import { none, through } from "@destack/access";
import type { Select } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { Outcome } from "../provider/provider.ts";
import { message } from "./message.ts";

/** One try of a provider at sending a message. */
export const messageAttempt = defineObject({
    name: "attempt",
    identity: "message-attempt",
    plural: "attempts",
    nested: { in: message, delete: "cascade", receive: "send" },
    fields: {
        /** What the provider made of the try. */
        outcome: field.json(Outcome),
    },
    permissions: { read: through("parent", "read"), record: none() },
    methods: (method) => ({
        get: method.get("read"),
        list: method.list("read"),
        /** Record a provider's try, as the message controller does. */
        create: method.create("record", { isInternal: true }),
    }),
});
/** A message attempt as its table stores it. */
export type MessageAttempt = Select<typeof messageAttempt.table>;
