import { principal } from "@destack/access";
import { ResourceId } from "@destack/resource";
import { check, foreignKey, index, sql, unique, uniqueIndex, type Select } from "@destack/db";
import { defineObject, field, method } from "@destack/object";
import { identifier, schema } from "@destack/schema";
import { account } from "./account.ts";

/** Complete a pending authorization with the provider's callback. */
const complete = method({
    permission: "authorize",
    input: schema.object({
        /** The state the provider returned. */
        state: schema.string().min(1),
        /** The provider's callback parameters, such as its code. */
        parameters: schema.record(schema.string(), schema.string()),
    }),
});

/** Withdraw an authorization at its provider. */
const revoke = method({ permission: "revoke" });

/** An external account a user authorized for an account. */
export const connection = defineObject({
    name: "connection",
    tier: "global",
    identity: "connected-account",
    plural: "connections",
    scope: account,
    fields: {
        /** The user who authorized the external account. */
        userId: field.reference(principal.user).caller(),
        /** The external service provider. */
        provider: field.string(schema.string().min(1)),
        /** The provider instance, including enterprise or self-hosted installations. */
        issuer: field.string(schema.string().min(1)),
        /** The provider application receiving this authorization. */
        applicationId: field.string(schema.string().min(1)),
        /** A user OAuth grant or a provider application installation. */
        kind: field.enum(["oauth", "installation"]),
        /** The provider's stable account identifier, absent while pending. */
        subject: field.string(schema.string().min(1)).optional(),
        /** The provider's installation identifier, for installations once completed. */
        installationId: field.string(schema.string().min(1)).optional(),
        /** The requested or granted provider scopes. */
        scopes: field.json(schema.array(schema.string())),
        /** Provider-specific permission levels granted to the application. */
        permissions: field.json(schema.record(schema.string(), schema.string())).default({}),
        /** The space whose vault holds an OAuth credential. */
        secretSpaceId: field.string(identifier("space")).optional(),
        /** The vault in the secret space holding an OAuth credential. */
        vaultId: field.string(ResourceId).optional(),
        /** The secret holding an OAuth credential. */
        secretId: field.string(identifier("secret")).optional(),
        /** The provider page where the user authorizes the application, while pending. */
        authorizationUrl: field.string(schema.httpUrl()).optional(),
        /** The state the provider returns with its callback, while pending. */
        state: field.string(schema.string().min(1)).optional(),
        /** The PKCE code verifier the provider's code exchange requires, while pending. */
        verifier: field.string().sensitive().optional(),
        /** The time the provider's callback completed the authorization. */
        authorizedAt: field.time().optional(),
        /** The credential expiry, when the provider sets one. */
        expiresAt: field.time().optional(),
        /** The time the authorization was revoked. */
        revokedAt: field.time().optional(),
    },
    permissions: ["read", "authorize", "revoke"],
    methods: {
        get: method.get("read"),
        list: method.list("read"),
        authorize: method.create("authorize", {
            fields: ["provider", "scopes", "secretSpaceId", "vaultId"],
            isPredicted: false,
        }),
        complete,
        cancel: method.delete("authorize"),
        revoke,
    },
    constraints: (entry) => [
        foreignKey({ columns: [entry.scope], foreignColumns: [account.table.id] }).onDelete(
            "restrict",
        ),
        unique("connection_scope_id").on(entry.scope, entry.id),
        uniqueIndex("connection_oauth")
            .on(entry.scope, entry.provider, entry.issuer, entry.applicationId, entry.subject)
            .where(sql`${entry.kind} = 'oauth' AND ${entry.revokedAt} IS NULL`),
        uniqueIndex("connection_installation")
            .on(
                entry.scope,
                entry.provider,
                entry.issuer,
                entry.applicationId,
                entry.installationId,
            )
            .where(sql`${entry.kind} = 'installation' AND ${entry.revokedAt} IS NULL`),
        check(
            "connection_pending",
            sql`(${entry.authorizedAt} IS NULL
                AND ${entry.subject} IS NULL AND ${entry.installationId} IS NULL AND ${entry.secretId} IS NULL
                AND ${entry.authorizationUrl} IS NOT NULL AND ${entry.state} IS NOT NULL AND ${entry.verifier} IS NOT NULL
                AND ${entry.revokedAt} IS NULL)
            OR (${entry.authorizedAt} IS NOT NULL
                AND ${entry.subject} IS NOT NULL
                AND ${entry.authorizationUrl} IS NULL AND ${entry.state} IS NULL AND ${entry.verifier} IS NULL)`,
        ),
        check(
            "connection_authorisation",
            sql`(${entry.kind} = 'oauth' AND ${entry.installationId} IS NULL
                AND ${entry.secretSpaceId} IS NOT NULL AND ${entry.vaultId} IS NOT NULL
                AND (${entry.authorizedAt} IS NULL OR ${entry.secretId} IS NOT NULL))
            OR (${entry.kind} = 'installation'
                AND ${entry.secretSpaceId} IS NULL AND ${entry.vaultId} IS NULL AND ${entry.secretId} IS NULL
                AND (${entry.authorizedAt} IS NULL OR ${entry.installationId} IS NOT NULL))`,
        ),
        index("connection_user").on(entry.userId),
    ],
});
/** An authorized external account. */
export type Connection = Select<typeof connection.table>;
