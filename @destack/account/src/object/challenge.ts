import { type Select } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { user } from "./user.ts";
import { expiring } from "@destack/object";
import { argument, plural, t } from "@destack/locale";

/** A pending second-factor step of a sign-in: its token's digest and expiry. */
export const challenge = defineObject({
    name: "challenge",
    plural: "challenges",
    title: t`${plural(argument("count"), { one: "Challenge", other: "Challenges" })}`,
    scope: user,
    fields: {
        /** The user signing in. */
        user: field.reference(user, { delete: "cascade" }),
        /** The digest of the challenge's token, which its browser stores in a cookie. */
        digest: field.string().sensitive("deterministic"),
        /** When the challenge lapses. */
        expiresAt: field.time(),
    },
    constraints: {
        digest: { kind: "unique", on: ["digest"] },
        expiry: { kind: "index", on: ["expiresAt"] },
    },
    access: {
        permissions: ["read"],
    },
    traits: [expiring([{ after: { milliseconds: 0 }, from: "expiresAt" }])],
});
/** A persisted second-factor challenge. */
export type Challenge = Select<typeof challenge.table>;
