import { type Select, Snapshot } from "@destack/db";
import { type MethodContext, defineObject, field } from "@destack/object";
import type { ObjectStore } from "@destack/object/server";
import { present, schema } from "@destack/schema";
import { RequestId } from "@destack/sync";
import { user } from "./user.ts";

/** A user's consent to a client's scopes, which every refresh token of the client rests on. */
export const oauthConsent = defineObject(
    {
        name: "oauth-consent",
        plural: "oauthConsents",
        scope: user,
        fields: {
            /** The authorized client: an application's public client identifier or a device's. */
            clientId: field.string(),
            /** The approved OAuth scopes. */
            scopes: field.json(schema.array(schema.string())),
            /** When the user granted the consent, before which no refresh token of it stands. */
            grantedAt: field.time(),
        },
        permissions: ["read", "revoke", "grant"],
        constraints: {
            clientUser: { kind: "unique", on: ["clientId", "scope"] },
            user: { kind: "index", on: ["scope"] },
        },
    },
    (method) => ({
        /** Grant a client the scopes a person approved. */
        create: method.create("grant", {
            fields: ["clientId", "scopes", "grantedAt"],
            visibility: "package",
            isPredicted: false,
        }),
        /** Widen a consent to the further scopes a person approved. */
        widen: method.update("grant", {
            fields: ["scopes"],
            visibility: "package",
            isPredicted: false,
        }),
        /** Revoke a consent, ending the client's refresh tokens. */
        revoke: method.delete("revoke"),
    }),
);
/** A stored OAuth consent. */
export type OAuthConsent = Select<typeof oauthConsent.table>;

/** The consents people give clients, and their ends. */
export const OAuthConsent = {
    /** Grant a client a person's scopes, widening a narrower consent, answering the consent. */
    async grant(
        objects: Pick<ObjectStore, "mutate" | "as" | "database" | "environment">,
        terms: {
            /** The client. */
            readonly clientId: string;
            /** The consenting person. */
            readonly userId: string;
            /** The scopes the person approved. */
            readonly scopes: readonly string[];
        },
        now: number,
    ): Promise<OAuthConsent> {
        // read the person's consent to the client
        const { clientId } = terms;
        const userId = user.identifier(terms.userId);
        const read = () =>
            Snapshot.live(objects.database)
                .read(oauthConsent.table)
                .findFirst({ where: { clientId, scope: userId } });
        const standing = await read();
        const scopes = [...new Set([...(standing?.scopes ?? []), ...terms.scopes])];

        // leave a consent covering the scopes as it is
        if (standing !== undefined && scopes.length === standing.scopes.length) {
            return standing;
        }

        // grant a new consent, or widen the standing one
        const call =
            standing === undefined
                ? oauthConsent.call("create", {
                      ...oauthConsent.in(userId),
                      clientId,
                      scopes,
                      grantedAt: now,
                  })
                : oauthConsent.call("widen", { ...oauthConsent.target(standing), scopes });
        await objects.mutate(objects.as(oauthConsent, now), {
            id: RequestId.create(objects.environment),
            calls: [call],
        });

        return present(await read(), `the consent of ${userId} to ${clientId}`);
    },

    /** Revoke every user's consent to a deleted or revoked client, ending its refresh tokens. */
    async withdraw(
        context: Pick<MethodContext, "read" | "mutate" | "self">,
        clientId: string,
    ): Promise<void> {
        // revoke the consents in one mutation as the store
        const consents = await context.read(oauthConsent).findMany({ where: { clientId } });
        if (consents.length > 0) {
            await context.mutate(
                context.self,
                consents.map((row) => oauthConsent.call("revoke", oauthConsent.target(row))),
            );
        }
    },
};
