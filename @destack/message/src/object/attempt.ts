import { none, through } from "@destack/access";
import type { Select } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { Outcome } from "../provider/provider.ts";
import { message } from "./message.ts";
import { nested } from "@destack/object";
import { argument, plural, t } from "@destack/locale";

/** One try of a provider at sending a message. */
export const messageAttempt = defineObject(
    {
        name: "attempt",
        identity: "message-attempt",
        plural: "attempts",
        title: t`${plural(argument("count"), { one: "Attempt", other: "Attempts" })}`,
        fields: {
            /** What the provider made of the try. */
            outcome: field.json(Outcome),
        },
        access: {
            permissions: { read: through("parent", "read"), record: none() },
        },
        traits: [nested({ in: message, delete: "cascade", add: "send" })],
    },
    (method) => ({
        /** Record a provider's try, as the message controller does. */
        create: method.create("record", { visibility: "package" }),
    }),
);
/** A message attempt as its table stores it. */
export type MessageAttempt = Select<typeof messageAttempt.table>;
