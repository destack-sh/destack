import { none, through } from "@destack/access";
import { space } from "@destack/account/object";
import { type Select } from "@destack/db";
import { defineObject, field } from "@destack/object";

/** A home's scheduled summary at one time, which the notifications waiting for it go out in. */
export const summary = defineObject(
    {
        name: "summary",
        plural: "summaries",
        scope: space,
        fields: {
            /** When it goes out. */
            at: field.time().immutable(),
            /** When it went out, absent until then. */
            sentAt: field.time().optional(),
        },
        permissions: { read: through("scope", "read"), plan: none() },
        // keep a summary as long as the notifications it sent
        expiring: [{ after: { days: 180 }, from: "at" }],
        constraints: {
            at: { kind: "unique", on: ["scope", "at"] },
            unsent: { kind: "index", on: ["sentAt"] },
        },
    },
    (method) => ({
        /** Schedule the summary at a time, answering the one scheduled there already. */
        create: method.create("plan", { isInternal: true, fields: ["at"] }),
        /** Send it to each summary destination of the notifications waiting for it. */
        send: method.mutation({ permission: "plan", isInternal: true }),
        update: false,
        delete: false,
    }),
);

/** A summary as its table stores it. */
export type Summary = Select<typeof summary.table>;
