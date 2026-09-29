import { account, type Account } from "../../object/account.ts";
import { Scope } from "@destack/sync";
import { Snapshot } from "@destack/db/log";
import { DirectoryDatabase } from "@destack/directory";
import { serviceAccount, ServiceAccountStanding } from "../../object/service.ts";
import {
    Authorization,
    principal,
    sameSubject,
    Authorizer,
    type Restriction,
    type Subject,
    Access,
    subjectKey,
} from "@destack/access";
import { and, eq, isNull, type DatabaseConnection } from "@destack/db";
import * as object from "../../object/index.ts";
import { ServiceError } from "@destack/service/error";
import {
    CALLER_LIFETIME_MILLISECONDS,
    TokenIssuer,
    Caller,
    TokenVerifier,
} from "@destack/service/authentication";
import type { PackageId } from "@destack/package";
import { identifier, type Identifier } from "@destack/schema";
import { implement, type ServiceContext } from "@destack/service/server";
import { accountPackage } from "../../audit/index.ts";
import type { Authentication } from "../../authentication/authentication.ts";
import { AccountCaller } from "../../authentication/caller.ts";
import { accountService } from "../../service/service.ts";

/** Typed implementations of the authentication procedures. */
const implementation = implement(accountService.router.authentication).$context<ServiceContext>();

/** A description of callers for one space and receiving service, under one transaction. */
class SpaceVerification {
    /** The space the caller acts in. */
    readonly spaceId: Identifier<"space">;
    /** The receiving service. */
    readonly audience: PackageId;
    /** The transaction reading the callers' standing. */
    readonly transaction: DatabaseConnection;

    /** Describe callers for a space and receiving service. */
    constructor(
        spaceId: Identifier<"space">,
        audience: PackageId,
        transaction: DatabaseConnection,
    ) {
        this.spaceId = spaceId;
        this.audience = audience;
        this.transaction = transaction;
    }

    /** Keep the restrictions both a current and an original credential allow, absent when neither restricts. */
    static intersect(
        current: readonly Restriction[] | undefined,
        original: readonly Restriction[] | undefined,
    ): Restriction[] | undefined {
        // keep the other side when one side is unrestricted
        if (current === undefined) {
            return original === undefined ? undefined : [...original];
        } else if (original === undefined) {
            return [...current];
        }

        // retain only matching operations and the narrower object selection
        const permissions: Restriction[] = [];
        for (const left of current) {
            for (const right of original) {
                if (
                    left.packageId === right.packageId &&
                    left.type === right.type &&
                    left.name === right.name &&
                    left.scope === right.scope &&
                    (left.objectId === undefined ||
                        right.objectId === undefined ||
                        left.objectId === right.objectId)
                ) {
                    permissions.push({ ...left, objectId: left.objectId ?? right.objectId });
                }
            }
        }

        return permissions;
    }

    /** Describe a host acting for the space it serves. */
    async host(caller: Caller) {
        // require the space's zone to be placed in the host's cell or its region's
        const cells = new Set(caller.authentication.subjects.map((subject) => subject.id));
        const zone = await new DirectoryDatabase(this.transaction).locate(this.spaceId);
        if (zone === undefined || !cells.has(zone.cell)) {
            throw new ServiceError("FORBIDDEN", { message: `${this.spaceId} is served elsewhere` });
        }

        // act as the host and its region in the space
        const verifiedAt = Date.now();
        const credential = caller.credential as { readonly kind: string; readonly id: string };

        return {
            scope: this.spaceId,
            audience: this.audience,
            verifiedAt,
            expiresAt: Math.min(
                caller.authentication.expiresAt,
                verifiedAt + CALLER_LIFETIME_MILLISECONDS,
            ),
            credential: { kind: credential.kind, id: credential.id },
            subject: caller.authentication.subject,
            subjects: [...caller.authentication.subjects],
        };
    }

    /** Describe an account caller, or the subject it represents, in the space. */
    async caller(caller: AccountCaller, subject: Subject = caller.authentication.subject) {
        // select the account that owns the space for the service token to belong to
        const credential = caller.credential;
        const accountId = await this.#account();
        if (credential.kind === "service-token" && credential.accountId !== accountId) {
            throw new ServiceError("FORBIDDEN", {
                message: "service token belongs to another account",
            });
        }

        // resolve the subject's memberships and groups in the account, in key order
        const { subjects: expanded } = await Access.expand(
            Snapshot.live(this.transaction),
            [subject],
            { now: Date.now() },
        );
        const sets = expanded
            .filter(
                (entry) =>
                    entry.relation !== undefined &&
                    entry.packageId === object.account.policy.definition.packageId &&
                    (entry.scope === accountId ||
                        (entry.type === object.account.name && entry.id === accountId)),
            )
            .toSorted((left, right) => (subjectKey(left) < subjectKey(right) ? -1 : 1));

        // carry a token's restrictions in the account or the space into the space
        const permissions = caller.authentication.permissions
            ?.filter((entry) => entry.scope === accountId || entry.scope === this.spaceId)
            .map((entry) => ({ ...entry, scope: this.spaceId }));

        // bound the returned verification by both the credential and assertion lifetimes
        const verifiedAt = Date.now();

        return {
            scope: this.spaceId,
            audience: this.audience,
            verifiedAt,
            expiresAt: Math.min(
                caller.authentication.expiresAt,
                verifiedAt + CALLER_LIFETIME_MILLISECONDS,
            ),
            credential: { kind: credential.kind, id: credential.id },
            subject,
            ...(caller.authentication.assurance === undefined
                ? {}
                : { assurance: caller.authentication.assurance }),
            ...(caller.authentication.identifiers === undefined ||
            subject !== caller.authentication.subject
                ? {}
                : { identifiers: [...caller.authentication.identifiers] }),
            subjects: [subject, ...sets],
            permissions,
        };
    }

    /** Describe a user acting as another user in the space. */
    async impersonation(caller: AccountCaller, subject: Subject, authorizer: Authorizer) {
        // require a user acting as another user
        const acting = caller.authentication.subject;
        if (
            !principal.user.is(acting) ||
            !principal.user.is(subject) ||
            sameSubject(acting, subject)
        ) {
            throw new ServiceError("FORBIDDEN", { message: "only a user acts as another user" });
        }

        // describe the represented user within the space's account
        const represented = await this.caller(caller, subject);
        const isMember = represented.subjects.length > 1;

        // require the lent authority, or administering the account of a member
        const authorization = new Authorization(authorizer, this.transaction, (scope) =>
            caller.context(accountPackage.id, Date.now(), scope),
        );
        const accountId = await this.#account();
        const lent = await authorization.check(
            object.user.permission("impersonate"),
            principal.user.reference(Scope.universe.id, subject.id),
        );
        const isLent = lent.isAllowed;
        const isAdministered =
            isMember &&
            (
                await authorization.check(
                    object.account.permission("impersonate"),
                    await Scope.object(Snapshot.live(this.transaction), accountId),
                )
            ).isAllowed;
        if (!isLent && !isAdministered) {
            throw new ServiceError("FORBIDDEN", {
                message: "acting as another user is not allowed",
            });
        }

        return { ...represented, delegates: [{ subject: acting, authority: "full" as const }] };
    }

    /** Read the live account that owns the space. */
    async #account(): Promise<Account["id"]> {
        // locate the space's zone, then require its account to stand
        const zone = await new DirectoryDatabase(this.transaction).locate(this.spaceId);
        const selected =
            zone === undefined
                ? undefined
                : await this.transaction
                      .select({ accountId: account.table.id })
                      .from(account.table)
                      .innerJoin(Scope.table, eq(Scope.table.scope, account.table.id))
                      .where(
                          and(
                              eq(account.table.id, identifier("account").parse(zone.scope)),
                              isNull(Scope.table.suspendedAt),
                              isNull(account.table.deletionRequestedAt),
                          ),
                      )
                      .get();
        if (!selected) {
            throw new ServiceError("FORBIDDEN", { message: "the space has no active account" });
        }

        return selected.accountId;
    }
}

/** Serve the authentication procedures. */
export function authenticationRouter(authentication: Authentication, authorizer: Authorizer) {
    return implementation.router({
        current: implementation.current.handler(async ({ context }) => {
            // read the profile and credential under the same authoritative transaction
            return authentication.database.transaction(
                async (transaction) => {
                    // verify the caller and read its profile
                    const caller = await AccountCaller.require(context.requireCaller()).verify(
                        transaction,
                    );
                    const { kind, id } = caller.credential;
                    const profile = await caller.readProfile(transaction);

                    return { ...caller.authentication, credential: { kind, id }, profile };
                },
                { signal: context.request.signal },
            );
        }),
        exchange: implementation.exchange.handler(async ({ input, context }) => {
            // describe the serving host, or recheck the caller
            const verified = context.requireCaller();
            const identity = await authentication.database.transaction(
                async (transaction) => {
                    // describe callers in the space for the receiving service
                    const verification = new SpaceVerification(
                        input.spaceId,
                        input.audience,
                        transaction,
                    );

                    // describe a host acting for the space it serves, as itself alone
                    if (principal.host.is(verified.authentication.subject)) {
                        if (input.subject !== undefined) {
                            throw new ServiceError("FORBIDDEN", {
                                message: "a host acts as no one but itself",
                            });
                        }

                        return verification.host(verified);
                    }

                    // describe the rechecked caller, or the user it acts as
                    const caller = await AccountCaller.require(verified).verify(transaction);

                    return input.subject === undefined
                        ? verification.caller(caller)
                        : verification.impersonation(caller, input.subject, authorizer);
                },
                { signal: context.request.signal },
            );

            // sign with Better Auth's keys
            const issuer = new TokenIssuer({
                authority: { kind: "universe" },
                issuer: authentication.options.baseURL as string,
                sign: async (payload) => {
                    const result = await authentication.api.signJWT({ body: { payload } });

                    return result.token;
                },
            });

            return issuer.issue(new Caller(identity));
        }),
        verify: implementation.verify.handler(async ({ input, context }) => {
            // resolve signing keys before the transaction
            const request = new Request(context.request.url, {
                headers: { authorization: `Bearer ${input.token}` },
            });
            let verified: Awaited<ReturnType<TokenVerifier["authenticate"]>> | undefined;
            let original: AccountCaller | undefined;
            if (input.token.split(".").length === 3) {
                const keys = await authentication.api.getJwks();
                const verifier = new TokenVerifier({
                    authority: { kind: "universe" },
                    issuer: authentication.options.baseURL as string,
                    audience: input.audience,
                    keys,
                });
                verified = await verifier.authenticate(request, input.spaceId);
            }
            // authenticate before the transaction
            else {
                original = await AccountCaller.authenticate(request, authentication);
            }

            return authentication.database.transaction(
                async (transaction) => {
                    // recheck the receiving service's credential, and confine it to the account's spaces
                    await AccountCaller.require(context.requireCaller()).verify(transaction);
                    const zone = await new DirectoryDatabase(transaction).locate(input.spaceId);
                    if (zone?.scope !== input.accountId) {
                        throw new ServiceError("FORBIDDEN", {
                            message: `${input.spaceId} is not a space of ${input.accountId}`,
                        });
                    }
                    let caller: AccountCaller;
                    // recheck the original credential
                    if (verified) {
                        caller = await AccountCaller.read(verified.credential, transaction);
                    }
                    // recheck the opaque credential
                    else {
                        caller = await original!.verify(transaction);
                    }

                    // describe the caller or the user it acts as
                    const verification = new SpaceVerification(
                        input.spaceId,
                        input.audience,
                        transaction,
                    );
                    const impersonated =
                        verified?.authentication.delegates?.[0]?.authority === "full"
                            ? verified.authentication.subject
                            : undefined;
                    const described =
                        impersonated === undefined
                            ? await verification.caller(caller)
                            : await verification.impersonation(caller, impersonated, authorizer);
                    if (verified) {
                        // recheck service account actors
                        if (!sameSubject(described.subject, verified.authentication.subject)) {
                            throw new ServiceError("UNAUTHORIZED", {
                                message: "the caller changed during verification",
                            });
                        }
                        for (const { subject: actor } of verified.authentication.delegates ?? []) {
                            if (!serviceAccount.policy.is(actor)) {
                                continue;
                            }
                            const standing = await ServiceAccountStanding.read(
                                identifier("account").parse(actor.scope),
                                identifier("service-account").parse(actor.id),
                                transaction,
                            );
                            if (standing === undefined || !standing.isActive) {
                                throw new ServiceError("UNAUTHORIZED", {
                                    message: "the acting service account is not active",
                                });
                            }
                        }

                        return {
                            ...described,
                            delegates: verified.authentication.delegates
                                ? [...verified.authentication.delegates]
                                : undefined,
                            deployments: verified.authentication.deployments
                                ? [...verified.authentication.deployments]
                                : undefined,
                            attributes: verified.authentication.attributes,
                            expiresAt: Math.min(
                                described.expiresAt,
                                verified.authentication.expiresAt,
                            ),
                            permissions: SpaceVerification.intersect(
                                described.permissions,
                                verified.authentication.permissions,
                            ),
                        };
                    }

                    return described;
                },
                { signal: context.request.signal },
            );
        }),
    });
}
