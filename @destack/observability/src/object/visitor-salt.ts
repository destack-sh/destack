import { none } from "@destack/access";
import { space } from "@destack/account/object";
import { type Select } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { schema } from "@destack/schema";
import { expiring } from "@destack/object";
import { argument, plural, t } from "@destack/locale";

/** The salt of one day, which hashes a space's anonymous visitors until it expires a day later. */
export const visitorSalt = defineObject(
    {
        name: "visitor-salt",
        plural: "visitorSalts",
        title: t`${plural(argument("count"), { one: "Visitor salt", other: "Visitor salts" })}`,
        scope: space,
        fields: {
            /** The day, in UTC, such as 2026-10-07. */
            day: field.string(schema.iso.date()),
            /** The random salt, in hexadecimal. */
            salt: field.string(),
        },
        constraints: {
            day: { kind: "unique", on: ["scope", "day"] },
        },
        access: {
            permissions: {
                read: none(),
                // let observability's code create the day's salt, through its role
                write: none(),
            },
        },
        traits: [
            // expire a salt a day after its day's first visit, as a visitor's hash lasts one day
            expiring([{ after: { days: 1 }, from: "createdAt" }]),
        ],
    },
    (method) => ({
        /** Create the salt of a new day. */
        create: method.create("write", { visibility: "package" }),
    }),
);

/** A day's visitor salt, as its table stores it. */
export type VisitorSalt = Select<typeof visitorSalt.table>;
