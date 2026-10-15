import { type Select } from "@destack/db";
import { type MethodContext, defineObject, field } from "@destack/object";
import { defineSchema, schema } from "@destack/schema";
import { user } from "./user.ts";

/** How a granted method runs for a client. */
export const GRANT_MODES = ["allowed", "approval", "refused"] as const;

/** One grant of a consent in the shape of an RFC 9396 authorization detail. */
export const AuthorizationDetail = defineSchema(
    schema.object({
        /** The detail type, Destack's methods. */
        type: schema.literal("method"),
        /** The spaces the methods are called in, by identifier. */
        locations: schema.array(schema.identifier("space")).min(1),
        /** The methods. */
        actions: schema.array(schema.string().min(1)).min(1),
        /** How the methods run for the client. */
        mode: schema.enum(GRANT_MODES),
    }),
);
/** One grant of a consent. */
export type AuthorizationDetail = schema.Infer<typeof AuthorizationDetail>;

/** A user's consent to a client's scopes, and the methods it may call in which spaces. */
export const oauthConsent = defineObject({
    name: "oauth-consent",
    plural: "oauthConsents",
    scope: user,
    fields: {
        /** The authorized client: a registered OAuth client's public identifier, or a device's. */
        clientId: field.string(),
        /** The consenting user. */
        user: field.reference(user, { delete: "cascade" }),
        /** The authorization provider's external subject reference. */
        referenceId: field.string().optional(),
        /** The approved OAuth audience URLs. */
        resources: field.json(schema.array(schema.string())).optional(),
        /** The approved OpenID user claims. */
        requestedUserInfoClaims: field.json(schema.array(schema.string())).optional(),
        /** The approved OAuth scopes. */
        scopes: field.json(schema.array(schema.string())),
        /** The methods the client may call, in which spaces and how (RFC 9396). */
        authorizationDetails: field.json(schema.array(AuthorizationDetail)).default([]),
        /** When the user last granted the consent, before which no token of it stands. */
        grantedAt: field.time(),
        /** When the user revoked the consent, which ends its tokens until granted again. */
        revokedAt: field.time().optional(),
    },
    permissions: ["read", "grant", "revoke"],
    constraints: {
        clientUser: { kind: "unique", on: ["clientId", "userId"] },
        user: { kind: "index", on: ["userId"] },
    },
}).methods((method) => ({
    /** Set the methods the client may call, in which spaces and how. */
    grant: method.update("grant", { fields: ["authorizationDetails"] }),
    revoke: method.mutation({ permission: "revoke" }),
}));
/** A persisted OAuth consent. */
export type OAuthConsent = Select<typeof oauthConsent.table>;

/** The ends of the provider's grants to clients. */
export const OAuthConsent = {
    /** Revoke every user's consent to a deleted or revoked client, ending its tokens' use. */
    async withdraw(
        context: Pick<MethodContext, "read" | "mutate" | "self">,
        clientId: string,
    ): Promise<void> {
        // revoke the standing consents in one mutation as the store
        const consents = await context
            .read(oauthConsent)
            .findMany({ where: { clientId, revokedAt: { isNull: true } } });
        if (consents.length > 0) {
            await context.mutate(
                context.self,
                consents.map((row) => oauthConsent.call("revoke", oauthConsent.target(row))),
            );
        }
    },
};
