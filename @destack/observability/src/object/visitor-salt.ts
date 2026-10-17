import { contained, none } from "@destack/access";
import { space } from "@destack/account/object";
import { type Select } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { schema } from "@destack/schema";

/** The salt of one day, which hashes a space's anonymous visitors until it expires a day later. */
export const visitorSalt = defineObject(
    {
        name: "visitor-salt",
        plural: "visitorSalts",
        scope: space,
        // expire a salt a day after its day's first visit, as a visitor's hash lasts one day
        expiring: [{ after: { days: 1 }, from: "createdAt" }],
        fields: {
            /** The day, in UTC, such as 2026-10-07. */
            day: field.string(schema.iso.date()),
            /** The random salt, in hexadecimal. */
            salt: field.string(),
        },
        permissions: {
            read: none(),
            // let the space's own observability create the day's salt
            write: contained(),
        },
        constraints: {
            day: { kind: "unique", on: ["scope", "day"] },
        },
    },
    (method) => ({
        /** Create the salt of a new day. */
        create: method.create("write", { isInternal: true }),
    }),
);

/** A day's visitor salt, as its table stores it. */
export type VisitorSalt = Select<typeof visitorSalt.table>;
