import { Restriction } from "@destack/access";
import { unique, type Select } from "@destack/db";
import { defineObject, field, method } from "@destack/object";
import { Digest, schema } from "@destack/schema";
import { account } from "./account.ts";
import { serviceAccount } from "./service.ts";
import { user } from "./user.ts";
import { sudo } from "./sudo.ts";
import { revokeOnce } from "./revocation.ts";

/** The most restrictions a token carries, 32 of 150 bytes below 8 KiB headers. */
const RESTRICTIONS = 32;

/** The fields every token holds. */
const TOKEN_FIELDS = {
    /** The holder's name for the token. */
    name: field.string(schema.string().min(1).max(200)),
    /** The SHA-256 digest of the token's secret. */
    digest: field.string(Digest).sensitive(),
    /** The permissions the token allows. */
    restrictions: field.json(schema.array(Restriction.schema).max(RESTRICTIONS)),
    /** The time the token stops authenticating. */
    expiresAt: field.time(),
    /** The time the holder withdrew the token. */
    revokedAt: field.time().optional(),
};

/** A bearer token a user holds. */
export const personalAccessToken = defineObject({
    name: "personal-access-token",
    tier: "global",
    identity: "token",
    plural: "personalAccessTokens",
    scope: user,
    fields: TOKEN_FIELDS,
    permissions: ["read", "issue", "revoke"],
    elevated: { issue: sudo },
    methods: {
        get: method.get("read"),
        list: method.list("read"),
        issue: method.create("issue", {
            fields: ["name", "digest", "restrictions", "expiresAt"],
            isPredicted: false,
        }),
        revoke: method({ permission: "revoke" }).handle((call) =>
            revokeOnce(call, call.object.name),
        ),
    },
    constraints: (token) => [unique("personal_access_token_digest").on(token.digest)],
});
/** A persisted personal access token. */
export type PersonalAccessToken = Select<typeof personalAccessToken.table>;

/** A bearer token that acts as its service account within its restrictions. */
export const serviceToken = defineObject({
    name: "service-token",
    tier: "global",
    identity: "token",
    plural: "serviceTokens",
    scope: account,
    nested: { in: serviceAccount, receive: "issue", delete: "cascade" },
    fields: TOKEN_FIELDS,
    permissions: ["read", "issue", "revoke"],
    methods: {
        get: method.get("read"),
        list: method.list("read"),
        issue: method.create("issue", {
            fields: ["name", "digest", "restrictions", "expiresAt"],
            isPredicted: false,
        }),
        revoke: method({ permission: "revoke" }).handle((call) =>
            revokeOnce(call, call.object.name),
        ),
    },
    constraints: (token) => [unique("service_token_digest").on(token.digest)],
});
/** A persisted service token. */
export type ServiceToken = Select<typeof serviceToken.table>;
