import { account } from "../object/account.ts";
import { Scope } from "@destack/sync";
import { type User, user } from "../object/user.ts";
import { emailIdentifier, principal, type Restriction } from "@destack/access";
import { identifier } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import type { Authentication } from "./authentication.ts";
import { isAPIError } from "better-auth/api";
import { ActiveSession } from "./session.ts";
import { and, eq, gt, isNull, type DatabaseConnection } from "@destack/db";
import { serviceAccount, ServiceAccountStanding, type ServiceAccount } from "../object/service.ts";
import {
    personalAccessToken,
    serviceToken,
    type PersonalAccessToken,
    type ServiceToken,
} from "../object/token.ts";
import { Credential, type TokenKind } from "../object/credential.ts";
import { readBearer } from "./bearer.ts";
import { Digest } from "../object/digest.ts";
import { type Session } from "../object/authentication.ts";
import { type Account } from "../object/account.ts";
import { Caller } from "@destack/service/authentication";
import { accountPackage } from "../audit/record.ts";
import type { PackageId } from "@destack/package";

/** The credential a verified caller presented. */
export type AccountCredential =
    | {
          /** A signed-in person's session. */
          readonly kind: "session";
          /** The verified session. */
          readonly id: Session["id"];
          /** The authenticated user. */
          readonly userId: User["id"];
      }
    | {
          /** A token a user holds, acting as the user within its restrictions. */
          readonly kind: "personal-access-token";
          /** The verified token. */
          readonly id: PersonalAccessToken["id"];
          /** The user holding the token. */
          readonly userId: User["id"];
      }
    | {
          /** A token that acts as its service account within its restrictions. */
          readonly kind: "service-token";
          /** The verified token. */
          readonly id: ServiceToken["id"];
          /** The account the service account belongs to. */
          readonly accountId: Account["id"];
          /** The service account holding the token. */
          readonly serviceAccountId: ServiceAccount["id"];
      };

/** A caller the account service verified. */
export class AccountCaller extends Caller<AccountCredential> {
    /** Verify a request's token or session. */
    static async authenticate(
        request: Request,
        authentication: Authentication,
        audience: PackageId = accountPackage.id,
    ): Promise<AccountCaller> {
        // require an unambiguous native bearer credential
        const hasAuthorization = request.headers.has("authorization");
        const bearer = readBearer(request.headers);
        if (hasAuthorization && bearer === undefined) {
            throw new ServiceError("UNAUTHORIZED", { message: "invalid bearer credential" });
        }

        // verify tokens by their secret's digest and refuse a client secret
        const kind = bearer === undefined ? undefined : Credential.kind(bearer);
        if (kind === "oauth-client") {
            throw new ServiceError("UNAUTHORIZED", {
                message: "a client secret cannot authenticate a request",
            });
        } else if (bearer !== undefined && kind !== undefined) {
            return AccountCaller.#token(
                kind,
                { digest: await Digest.hex(bearer) },
                authentication.database,
                audience,
            );
        }

        // require an allowed origin before accepting a cookie authenticated mutation
        if (!hasAuthorization && !["GET", "HEAD", "OPTIONS"].includes(request.method)) {
            const origin = request.headers.get("origin");
            const origins = authentication.options.trustedOrigins;
            if (!origin || !Array.isArray(origins) || !origins.includes(origin)) {
                throw new ServiceError("FORBIDDEN", { message: "untrusted request origin" });
            }
        }

        // read the session Better Auth resolves
        let result;
        try {
            result = await authentication.api.getSession({ headers: request.headers });
        } catch (error) {
            if (isAPIError(error) && error.statusCode === 401) {
                throw new ServiceError("UNAUTHORIZED", { message: "the person is not signed in" });
            }
            throw error;
        }
        if (!result) {
            throw new ServiceError("UNAUTHORIZED", { message: "the person is not signed in" });
        }
        const current = ActiveSession.of(result);

        return AccountCaller.#create(
            { kind: "session", id: current.id, userId: current.userId },
            current.expiresAt,
            audience,
            current,
        );
    }

    /** Recheck a credential against current records. */
    static async read(
        credential: { readonly kind: string; readonly id: string },
        database: DatabaseConnection,
    ): Promise<AccountCaller> {
        // recheck a token by its identifier
        if (credential.kind === "personal-access-token" || credential.kind === "service-token") {
            return AccountCaller.#token(
                credential.kind,
                { id: credential.id },
                database,
                accountPackage.id,
            );
        }
        // apply session revocation and user suspension during access-token renewal
        else if (credential.kind === "session") {
            try {
                const current = await ActiveSession.read(credential.id, database);

                return AccountCaller.#create(
                    {
                        kind: "session",
                        id: identifier("session").parse(credential.id),
                        userId: current.userId,
                    },
                    current.expiresAt,
                    accountPackage.id,
                    current,
                );
            } catch (error) {
                if (isAPIError(error) && error.statusCode === 401) {
                    throw new ServiceError("UNAUTHORIZED", {
                        message: "session is no longer authorized",
                    });
                }
                throw error;
            }
        } else {
            throw new ServiceError("UNAUTHORIZED", {
                message: `unknown credential kind: ${credential.kind}`,
            });
        }
    }

    /** Narrow a verified caller to an account caller. */
    static require(caller: Caller): AccountCaller {
        if (!(caller instanceof AccountCaller)) {
            throw new ServiceError("UNAUTHORIZED", {
                message: "the caller is not an account caller",
            });
        }

        return caller;
    }

    /** Recheck the caller's credential within a transaction. */
    async verify(database: DatabaseConnection): Promise<AccountCaller> {
        this.context(accountPackage.id);

        return AccountCaller.read(this.credential, database);
    }

    /** Read the caller's public profile: a user's name and personal handle, or a service account's name. */
    async readProfile(transaction: DatabaseConnection) {
        // read the service account's name
        const subject = this.authentication.subject;
        if (!principal.user.is(subject)) {
            const software = await transaction
                .select({ name: serviceAccount.table.name })
                .from(serviceAccount.table)
                .where(eq(serviceAccount.table.id, identifier("service-account").parse(subject.id)))
                .get();
            if (!software) {
                throw new ServiceError("UNAUTHORIZED", {
                    message: "the service account is not active",
                });
            }

            return { subject, name: software.name, handle: null };
        }

        // read the user's name and their personal account's handle
        const person = await transaction
            .select({ name: user.table.name, handle: account.table.handle })
            .from(user.table)
            .leftJoin(
                account.table,
                and(eq(account.table.scope, user.table.id), eq(account.table.kind, "personal")),
            )
            .where(eq(user.table.id, identifier("user").parse(subject.id)))
            .get();
        if (!person) {
            throw new ServiceError("UNAUTHORIZED", { message: "the user is not active" });
        }

        return { subject, name: person.name, handle: person.handle };
    }

    /** Verify a token and its holder's standing. */
    static async #token(
        kind: TokenKind,
        key: { readonly id: string } | { readonly digest: string },
        database: DatabaseConnection,
        audience: PackageId,
    ): Promise<AccountCaller> {
        // read the unrevoked, unexpired personal token of an active user
        if (kind === "personal-access-token") {
            const table = personalAccessToken.table;
            const [row] = await database
                .select({
                    id: table.id,
                    userId: user.table.id,
                    restrictions: table.restrictions,
                    expiresAt: table.expiresAt,
                })
                .from(table)
                .innerJoin(user.table, eq(user.table.id, table.scope))
                .where(
                    and(
                        "id" in key
                            ? eq(table.id, key.id as PersonalAccessToken["id"])
                            : eq(table.digest, key.digest),
                        gt(table.expiresAt, Date.now()),
                        isNull(table.revokedAt),
                        isNull(user.table.suspendedAt),
                        isNull(user.table.deletionRequestedAt),
                    ),
                )
                .limit(1);
            if (!row) {
                throw new ServiceError("UNAUTHORIZED", {
                    message: "invalid personal access token",
                });
            }

            return AccountCaller.#create(
                { kind, id: row.id, userId: row.userId },
                row.expiresAt,
                audience,
                undefined,
                row.restrictions,
            );
        }
        // read the active token of a standing service account
        else {
            const table = serviceToken.table;
            const [row] = await database
                .select({
                    id: table.id,
                    accountId: table.scope,
                    serviceAccountId: table.parentId,
                    restrictions: table.restrictions,
                    expiresAt: table.expiresAt,
                })
                .from(table)
                .where(
                    and(
                        "id" in key
                            ? eq(table.id, key.id as ServiceToken["id"])
                            : eq(table.digest, key.digest),
                        gt(table.expiresAt, Date.now()),
                        isNull(table.revokedAt),
                    ),
                )
                .limit(1);
            const standing =
                row === undefined
                    ? undefined
                    : await ServiceAccountStanding.read(
                          row.accountId,
                          row.serviceAccountId,
                          database,
                      );
            if (row === undefined || standing === undefined || !standing.isActive) {
                throw new ServiceError("UNAUTHORIZED", { message: "invalid service token" });
            }

            return AccountCaller.#create(
                {
                    kind,
                    id: row.id,
                    accountId: row.accountId,
                    serviceAccountId: row.serviceAccountId,
                },
                row.expiresAt,
                audience,
                undefined,
                row.restrictions,
            );
        }
    }

    /** Construct the caller after authoritative credential verification. */
    static #create(
        credential: AccountCredential,
        expiresAt: number,
        audience: PackageId,
        session?: ActiveSession,
        restrictions?: readonly Restriction[],
    ): AccountCaller {
        // carry a session's assurance and email, and a token's restrictions
        const assurance = session?.assurance;
        const identifiers = session?.emailVerified ? [emailIdentifier(session.email)] : undefined;
        const subject =
            credential.kind === "service-token"
                ? serviceAccount.reference(credential.accountId, credential.serviceAccountId)
                : principal.user.reference(Scope.universe.id, credential.userId);

        return new AccountCaller({
            credential,
            audience,
            verifiedAt: Date.now(),
            expiresAt,
            subject,
            subjects: [subject],
            ...(assurance === undefined ? {} : { assurance }),
            ...(identifiers === undefined ? {} : { identifiers }),
            ...(restrictions === undefined ? {} : { permissions: [...restrictions] }),
        });
    }
}
