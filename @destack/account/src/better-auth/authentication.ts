import { accessGrant, type Caller, Contact, Grant, principal } from "@destack/access";
import { and, eq, Snapshot, type DatabaseConnection } from "@destack/db";
import type { PackageId } from "@destack/package";
import { Digest } from "@destack/schema";
import { Bearer, TokenVerifier } from "@destack/runtime/authentication";
import { CallError } from "@destack/runtime/error";
import { ObjectReference, Scope } from "@destack/sync";
import { Identity } from "../object/identity.ts";
import { personalAccessToken } from "../object/personal-access-token.ts";
import { session } from "../object/session.ts";
import { user, User } from "../object/user.ts";
import type { VerifiedCaller } from "../oauth/issuance.ts";
import { ActiveSession } from "./active-session.ts";

/** The callers of account's packages: access tokens, and the credentials exchanged for them. */
export const AccountAuthentication = {
    /** Verify a request's bearer access token for a package by the universe's keys. */
    async caller(
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
    ): Promise<Caller | undefined> {
        // leave a request without a bearer token to the protocol it speaks, such as a client's own credentials
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

    /** Verify a session's token as its person acting through the session's client. */
    async session(
        token: string,
        database: DatabaseConnection,
        audience: PackageId,
        now: number,
    ): Promise<VerifiedCaller> {
        // find the session by its token
        const [found] = await database
            .select({ id: session.table.id, clientId: session.table.clientId })
            .from(session.table)
            .where(eq(session.table.token, token))
            .limit(1);
        if (found === undefined) {
            throw new CallError("UNAUTHORIZED", { message: "invalid session" });
        }

        // require it active, and describe its person with its assurance and verified email
        const active = await ActiveSession.read(found.id, database, now).catch(() => {
            throw new CallError("UNAUTHORIZED", { message: "session is no longer authorized" });
        });
        const caller = AccountAuthentication.sessionCaller(active, audience, now);

        return found.clientId === null
            ? caller
            : { ...caller, credential: { ...caller.credential, client: found.clientId } };
    },

    /** Describe a person signed in through an active session, with its assurance and email. */
    sessionCaller(active: ActiveSession, audience: PackageId, now: number): VerifiedCaller {
        const { assurance } = active;
        const caller = callerOf(
            "session",
            active.id,
            active.userId,
            active.expiresAt,
            audience,
            now,
        );

        return {
            ...caller,
            ...(assurance === undefined ? {} : { assurance }),
            ...(active.emailVerified ? { contacts: [Contact.email(active.email)] } : {}),
        };
    },

    /** Verify a personal access token of an active user under the user's live grant to it. */
    async personalToken(
        secret: string,
        database: DatabaseConnection,
        audience: PackageId,
        now: number,
    ): Promise<VerifiedCaller> {
        // read the token of an active user by its secret's digest
        const table = personalAccessToken.table;
        const [row] = await database
            .select({ id: table.id, userId: user.table.id })
            .from(table)
            .innerJoin(user.table, eq(user.table.id, table.scope))
            .where(and(eq(table.digest, await Digest.of(secret)), User.isActive()))
            .limit(1);
        if (row === undefined) {
            throw new CallError("UNAUTHORIZED", { message: "invalid personal access token" });
        }

        // read the user's unexpired grant to it
        const delegate = personalAccessToken.reference(row.userId, row.id);
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
            throw new CallError("UNAUTHORIZED", { message: "invalid personal access token" });
        }

        // act as the user through the token, under the grant until it lapses
        const lapse = grant.expiresAt ?? Number.POSITIVE_INFINITY;
        const caller = callerOf("personal-access-token", row.id, row.userId, lapse, audience, now);

        return { ...caller, delegates: [delegate], grant: Grant.claim(grant) };
    },
};

/** Describe the person of a credential as a caller of an audience, until the credential lapses. */
function callerOf(
    kind: string,
    id: string,
    userId: string,
    expiresAt: number,
    audience: PackageId,
    now: number,
): VerifiedCaller {
    const subject = principal.user.reference(Scope.universe.id, userId);

    return {
        subject,
        subjects: [subject],
        credential: {
            kind,
            id,
            audience,
            verifiedAt: now,
            expiresAt,
        },
    };
}
