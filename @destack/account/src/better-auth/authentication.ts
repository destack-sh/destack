import {
    accessGrant,
    type Authentication,
    Caller,
    Contact,
    Grant,
    principal,
} from "@destack/access";
import { and, eq, isNull, Snapshot, type DatabaseConnection } from "@destack/db";
import type { PackageId } from "@destack/package";
import { Digest } from "@destack/schema";
import { Bearer, TokenVerifier } from "@destack/access/token";
import { ObjectReference, Scope } from "@destack/sync";
import { Identity } from "../object/identity.ts";
import { client } from "../object/client.ts";
import { user, User } from "../object/user.ts";
import { ActiveSession } from "./active-session.ts";
import type {} from "@destack/package/import-meta";
import { AccountError } from "../error/error.ts";

/** The callers of account's packages: access tokens, and the credentials exchanged for them. */
export const AccountAuthentication = {
    /** Verify a request's bearer access token for a package by the universe's keys. */
    async bearer(
        request: Request,
        verifier: {
            /** The account database with the universe's identity. */
            readonly database: DatabaseConnection;
            /** The universe's issuer. */
            readonly issuer: string;
            /** The package receiving the call. */
            readonly audience: PackageId;
            /** The time of the verification. */
            readonly now: number;
        },
    ): Promise<Authentication | undefined> {
        // leave a request without a bearer token to the protocol it speaks, such as a client's credentials
        const authorization = request.headers.get("authorization");
        if (authorization === null || !/^bearer /iu.test(authorization)) {
            return undefined;
        }

        // verify the access token by the universe's current keys
        const keys = await Identity.keys(Snapshot.live(verifier.database), Scope.universe.id);
        const tokens = new TokenVerifier({
            authority: { kind: "universe" },
            issuer: verifier.issuer,
            audience: verifier.audience,
            keys,
        });

        return tokens.verify(Bearer.require(request.headers), verifier.now);
    },

    /** Authenticate a person signed in through an active session, with its assurance and email. */
    activeSession(active: ActiveSession, audience: PackageId, now: number): Authentication {
        const { assurance } = active;
        const { caller, credential } = authenticationOf(
            "session",
            active.id,
            active.userId,
            active.expiresAt,
            audience,
            now,
        );

        return {
            caller: active.emailVerified
                ? { ...caller, contacts: [Contact.email(active.email)] }
                : caller,
            credential: assurance === undefined ? credential : { ...credential, assurance },
        };
    },

    /** Verify a token client of an active user under the user's live grant to it. */
    async personalToken(
        secret: string,
        database: DatabaseConnection,
        audience: PackageId,
        now: number,
    ): Promise<Authentication> {
        // read the unrevoked token client of an active user by its secret's digest
        const table = client.table;
        const [row] = await database
            .select({ id: table.id, userId: user.table.id })
            .from(table)
            .innerJoin(user.table, eq(user.table.id, table.scope))
            .where(
                and(
                    eq(table.digest, await Digest.of(secret)),
                    isNull(table.revokedAt),
                    User.isActive(),
                ),
            )
            .limit(1);
        if (row === undefined) {
            throw AccountError.of("INVALID_PERSONAL_TOKEN", "invalid personal access token");
        }

        // read the user's unexpired grant to it
        const delegate = principal.client.reference(Scope.universe.id, row.id);
        const grant = await Snapshot.live(database)
            .read(accessGrant)
            .findFirst({
                where: {
                    scope: row.userId,
                    ...ObjectReference.columns("delegate", delegate),
                    expiresAt: { gt: now },
                },
            });
        if (grant === undefined) {
            throw AccountError.of("INVALID_PERSONAL_TOKEN", "invalid personal access token");
        }

        // act as the user through the token, under the grant until it lapses
        const lapse = grant.expiresAt ?? Number.POSITIVE_INFINITY;
        const { caller, credential } = authenticationOf(
            "personal-access-token",
            row.id,
            row.userId,
            lapse,
            audience,
            now,
        );

        return {
            caller: { ...caller, actors: [delegate], grants: [Grant.claim(grant)] },
            credential,
        };
    },
};

/** Authenticate the person of a credential for an audience, until the credential lapses. */
function authenticationOf(
    kind: string,
    id: string,
    userId: string,
    expiresAt: number,
    audience: PackageId,
    now: number,
): Authentication {
    const subject = principal.user.reference(Scope.universe.id, userId);

    return {
        caller: Caller.of(subject),
        credential: { kind, id, audience, verifiedAt: now, expiresAt },
    };
}
