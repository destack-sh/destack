import { none } from "@destack/access";
import { type DatabaseConnection, eq, type Select, Snapshot } from "@destack/db";
import type { Environment } from "@destack/environment";
import { defineObject, field } from "@destack/object";
import type { ObjectStore } from "@destack/object/server";
import { Digest, present, schema } from "@destack/schema";
import { RequestId } from "@destack/sync";
import { AccountError } from "../error/error.ts";
import { Credential } from "../credential/credential.ts";
import { type OAuthConsent, oauthConsent } from "./oauth-consent.ts";
import { user } from "./user.ts";

/** How long a refresh token lives unused: thirty days, after which the person signs in again. */
const REFRESH_TOKEN_MILLISECONDS = 30 * 24 * 60 * 60_000;

/** The states of a refresh token: live, then rotated into its successor (RFC 9700 4.14.2). */
export const REFRESH_TOKEN_STATUSES = ["live", "rotated"] as const;

/** A token a client renews its access with, rotated at each use (RFC 6749 6). */
export const refreshToken = defineObject(
    {
        name: "refresh-token",
        plural: "refreshTokens",
        scope: user,
        expiring: [{ after: { milliseconds: 0 }, from: "expiresAt" }],
        fields: {
            /** The SHA-256 digest of the token. */
            digest: field.string(Digest).sensitive("deterministic"),
            /** The family the token rotates within: the code, device or session it began with. */
            family: field.string(),
            /** The consent the token rests on, whose revocation ends it. */
            consent: field.reference(oauthConsent, { delete: "cascade" }),
            /** The granted OAuth scopes. */
            scopes: field.json(schema.array(schema.string())),
            /** The package the renewed access tokens are for, as its resource (RFC 8707). */
            resource: field.string().optional(),
            /** Whether the token renews, or was spent on its successor. */
            status: field.enum(REFRESH_TOKEN_STATUSES),
            /** When the token lapses. */
            expiresAt: field.time(),
        },
        permissions: { issue: none(), rotate: none(), revoke: none() },
        constraints: {
            digest: { kind: "unique", on: ["digest"] },
            family: { kind: "index", on: ["family"] },
            expiry: { kind: "index", on: ["expiresAt"] },
        },
    },
    (method) => ({
        /** Issue a token in a family. */
        create: method.create("issue", {
            fields: ["digest", "family", "consentId", "scopes", "resource", "status", "expiresAt"],
            isInternal: true,
            isPredicted: false,
        }),
        /** Spend a token on its successor, refusing one spent before. */
        rotate: method.mutation({ permission: "rotate", isInternal: true }),
        /** Remove a token, as its family ends. */
        delete: method.delete("revoke", { isInternal: true }),
    }),
).implement({
    methods: {
        rotate: (call) => {
            // refuse a token spent before
            if (call.target.status === "rotated") {
                throw AccountError.of("TOKEN_REUSED", "the refresh token was rotated before");
            }

            return call.update({ status: "rotated" });
        },
    },
});
/** A stored refresh token. */
export type RefreshToken = Select<typeof refreshToken.table>;

/** The refresh tokens of consents, rotated at each use within a family (RFC 9700 4.14.2). */
export const RefreshToken = {
    /** Issue a refresh token of a consent in a family, answering the token. */
    async issue(
        objects: Pick<ObjectStore, "mutate" | "as">,
        terms: {
            /** The consent the token rests on. */
            readonly row: OAuthConsent;
            /** The family the token joins. */
            readonly family: string;
            /** The granted OAuth scopes. */
            readonly scopes: readonly string[];
            /** The package the renewed access tokens are for. */
            readonly resource?: string | undefined;
            /** The token the new one succeeds, spent in the same mutation. */
            readonly rotating?: RefreshToken | undefined;
        },
        environment: Pick<Environment, "clock" | "crypto" | "random">,
    ): Promise<string> {
        // draw the token and store its digest, spending its predecessor with it
        const drawn = await Credential.create("refresh-token", environment);
        const { rotating } = terms;
        await objects.mutate(objects.as(refreshToken), {
            id: RequestId.create(environment),
            calls: [
                ...(rotating === undefined
                    ? []
                    : [refreshToken.call("rotate", refreshToken.target(rotating))]),
                refreshToken.call("create", {
                    ...refreshToken.in(terms.row.scope),
                    digest: drawn.digest,
                    family: terms.family,
                    consentId: terms.row.id,
                    scopes: [...terms.scopes],
                    resource: terms.resource ?? null,
                    status: "live",
                    expiresAt: environment.clock.now() + REFRESH_TOKEN_MILLISECONDS,
                }),
            ],
        });

        return drawn.secret;
    },

    /** Spend a refresh token without a successor, as a renewal dropping the offline scope does. */
    async spend(
        objects: Pick<ObjectStore, "mutate" | "as">,
        rotating: RefreshToken | undefined,
        environment: Pick<Environment, "clock" | "random">,
    ): Promise<undefined> {
        if (rotating !== undefined) {
            await objects.mutate(objects.as(refreshToken), {
                id: RequestId.create(environment),
                calls: [refreshToken.call("rotate", refreshToken.target(rotating))],
            });
        }

        return undefined;
    },

    /** Find an unexpired refresh token by its digest, with the consent it rests on. */
    async find(
        token: string,
        database: DatabaseConnection,
        environment: Pick<Environment, "clock" | "crypto">,
    ): Promise<{ readonly row: RefreshToken; readonly consent: OAuthConsent } | undefined> {
        // read the token by its digest, and its consent
        const digest = await Digest.of(token, environment.crypto);
        const [row] = await database
            .select()
            .from(refreshToken.table)
            .where(eq(refreshToken.table.digest, digest))
            .limit(1);
        if (row === undefined || row.expiresAt <= environment.clock.now()) {
            return undefined;
        }
        const consent = await Snapshot.live(database)
            .read(oauthConsent.table)
            .findFirst({ where: { id: row.consentId } });

        return { row, consent: present(consent, `the consent of refresh token ${row.id}`) };
    },

    /** Revoke a family's tokens, as a replayed code or token reveals a theft (RFC 9700 4.14.2). */
    async revoke(
        objects: Pick<ObjectStore, "mutate" | "as">,
        database: DatabaseConnection,
        family: Pick<RefreshToken, "scope" | "family">,
        environment: Pick<Environment, "clock" | "random">,
    ): Promise<void> {
        // remove every token of the family
        const rows = await Snapshot.live(database)
            .read(refreshToken.table)
            .findMany({ where: { scope: family.scope, family: family.family } });
        await objects.mutate(objects.as(refreshToken), {
            id: RequestId.create(environment),
            calls: rows.map((row) => refreshToken.call("delete", refreshToken.target(row))),
        });
    },
};
