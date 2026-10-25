import { none } from "@destack/access";
import { type DatabaseConnection, eq, type Select, Snapshot } from "@destack/db";
import type { Environment } from "@destack/environment";
import { defineObject, field } from "@destack/object";
import type { ObjectStore } from "@destack/object/server";
import { Digest, present, schema } from "@destack/schema";
import { RequestId } from "@destack/sync";
import { OAuthError } from "../authentication/token.ts";
import { AccountError } from "../error/error.ts";
import { type OAuthConsent, oauthConsent } from "./oauth-consent.ts";
import { RefreshToken } from "./refresh-token.ts";
import { session } from "./session.ts";
import { user } from "./user.ts";

/** The states of an authorization code: issued, then redeemed once. */
export const AUTHORIZATION_CODE_STATUSES = ["issued", "redeemed"] as const;

/** A code a person's consent returns to a client, redeemed once for tokens (RFC 6749 4.1). */
export const authorizationCode = defineObject(
    {
        name: "authorization-code",
        plural: "authorizationCodes",
        scope: user,
        expiring: [{ after: { milliseconds: 0 }, from: "expiresAt" }],
        fields: {
            /** The SHA-256 digest of the code. */
            digest: field.string(Digest).sensitive("deterministic"),
            /** The consent the code returns, whose revocation voids it. */
            consent: field.reference(oauthConsent, { delete: "cascade" }),
            /** The browser session that consented, which describes the person in the tokens. */
            session: field.reference(session, { delete: "null" }).optional(),
            /** The redirect the code returned to, which its redemption repeats. */
            redirectUri: field.string(schema.url()),
            /** The granted OAuth scopes. */
            scopes: field.json(schema.array(schema.string())),
            /** The client's value the ID token repeats. */
            nonce: field.string().optional(),
            /** The S256 challenge the redemption's verifier answers (RFC 7636). */
            codeChallenge: field.string(),
            /** The package the tokens are for, as its resource (RFC 8707). */
            resource: field.string().optional(),
            /** Whether the code is unused, or spent. */
            status: field.enum(AUTHORIZATION_CODE_STATUSES),
            /** When the code lapses. */
            expiresAt: field.time(),
        },
        permissions: { issue: none(), redeem: none() },
        constraints: {
            digest: { kind: "unique", on: ["digest"] },
            expiry: { kind: "index", on: ["expiresAt"] },
        },
    },
    (method) => ({
        /** Issue a code for a consent. */
        create: method.create("issue", {
            fields: [
                "digest",
                "consentId",
                "sessionId",
                "redirectUri",
                "scopes",
                "nonce",
                "codeChallenge",
                "resource",
                "status",
                "expiresAt",
            ],
            isInternal: true,
            isPredicted: false,
        }),
        /** Spend a code, refusing one spent before. */
        redeem: method.mutation({ permission: "redeem", isInternal: true }),
    }),
).implement({
    methods: {
        redeem: (call) => {
            // refuse a code spent before
            if (call.target.status === "redeemed") {
                throw AccountError.of("CODE_REUSED", "the authorization code was redeemed before");
            }

            return call.update({ status: "redeemed" });
        },
    },
});
/** A stored authorization code. */
export type AuthorizationCode = Select<typeof authorizationCode.table>;

/** The authorization codes of consents, each redeemed once (RFC 6749 4.1). */
export const AuthorizationCode = {
    /** Redeem a client's code, spending it, and revoking what it issued when spent before. */
    async redeem(
        code: string,
        clientId: string,
        options: {
            /** The account database with the codes and consents. */
            readonly database: DatabaseConnection;
            /** The account objects, which spend codes and revoke refresh tokens. */
            readonly objects: Pick<ObjectStore, "mutate" | "as">;
            /** The clock codes lapse on, the WebCrypto digesting them and the randomness. */
            readonly environment: Pick<Environment, "clock" | "crypto" | "random">;
        },
    ): Promise<{ readonly row: AuthorizationCode; readonly consent: OAuthConsent }> {
        // find the client's code by its digest
        const { database, objects, environment } = options;
        const digest = await Digest.of(code, environment.crypto);
        const [row] = await database
            .select()
            .from(authorizationCode.table)
            .where(eq(authorizationCode.table.digest, digest))
            .limit(1);
        if (row === undefined) {
            throw OAuthError.of("INVALID_GRANT", "invalid code");
        }

        // refuse another client's code
        const consent = present(
            await Snapshot.live(database)
                .read(oauthConsent.table)
                .findFirst({ where: { id: row.consentId } }),
            `the consent of code ${row.id}`,
        );
        if (consent.clientId !== clientId) {
            throw OAuthError.of("INVALID_GRANT", "invalid code");
        }

        // spend it, revoking the family it started when spent before (RFC 6749 4.1.2)
        const isSpent = await objects
            .mutate(objects.as(authorizationCode), {
                id: RequestId.create(environment),
                calls: [authorizationCode.call("redeem", authorizationCode.target(row))],
            })
            .then(
                () => false,
                (error: unknown) => {
                    // pass on any other failure
                    if (!AccountError.is(error, "CODE_REUSED")) {
                        throw error;
                    }

                    return true;
                },
            );
        if (isSpent) {
            await RefreshToken.revoke(
                objects,
                database,
                { scope: row.scope, family: row.id },
                environment,
            );
            throw OAuthError.of("INVALID_GRANT", "invalid code");
        }
        // refuse a lapsed code
        else if (row.expiresAt <= environment.clock.now()) {
            throw OAuthError.of("INVALID_GRANT", "the code expired");
        }

        return { row, consent };
    },
};
