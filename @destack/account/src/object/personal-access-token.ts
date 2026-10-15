import { GrantPermission, principal } from "@destack/access";
import { type Select } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { Digest, Instant, schema } from "@destack/schema";
import { user } from "./user.ts";
import { sudo } from "../authentication/sudo.ts";
import { DisplayName } from "./display-name.ts";

/** The most permissions a token's grant gives, 32 of 150 bytes below 8 KiB headers. */
const MAX_PERMISSIONS = 32;

/** A bearer token of a user, acting for them under their grant to it. */
export const personalAccessToken = defineObject({
    name: "personal-access-token",
    identity: "token",
    plural: "personalAccessTokens",
    scope: user,
    represents: principal.personalAccessToken,
    fields: {
        /** The holder's name for the token. */
        name: field.string(DisplayName),
        /** The SHA-256 digest of the token's secret. */
        digest: field.string(Digest).sensitive("deterministic"),
    },
    permissions: ["read", "issue", "revoke"],
    elevated: { issue: sudo },
    constraints: {
        digest: { kind: "unique", on: ["digest"] },
    },
}).methods((method) => ({
    issue: method.create("issue", {
        fields: ["name", "digest"],
        input: schema.object({
            /** The permissions the token's grant gives. */
            permissions: schema.array(GrantPermission.schema).max(MAX_PERMISSIONS),
            /** The time the token's grant lapses, in UTC epoch milliseconds. */
            expiresAt: Instant,
        }),
        isPredicted: false,
    }),
    revoke: method.delete("revoke"),
}));
/** A persisted personal access token. */
export type PersonalAccessToken = Select<typeof personalAccessToken.table>;
