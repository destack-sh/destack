import { type Select } from "@destack/db";
import { type MethodContext, defineObject, field } from "@destack/object";
import { schema } from "@destack/schema";
import { user } from "./user.ts";

/** A user's consent to an OAuth client's scopes and claims, its calls given by the user's grant. */
export const oauthConsent = defineObject(
    {
        name: "oauth-consent",
        plural: "oauthConsents",
        scope: user,
        fields: {
            /** The authorized client: an OAuth client's public identifier or a device's. */
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
            /** When the user last granted the consent, before which no token of it stands. */
            grantedAt: field.time(),
            /** When the user revoked the consent, which ends its tokens until granted again. */
            revokedAt: field.time().optional(),
        },
        permissions: ["read", "revoke"],
        constraints: {
            clientUser: { kind: "unique", on: ["clientId", "userId"] },
            user: { kind: "index", on: ["userId"] },
        },
    },
    (method) => ({
        revoke: method.mutation({ permission: "revoke" }),
    }),
);
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
