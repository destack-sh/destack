import {
    Access,
    AccessContext,
    accessGrant,
    type Authentication,
    Authorization,
    type Authorizer,
    Caller,
    Credential,
    Grant,
    principal,
} from "@destack/access";
import type { OAuthExtensionGrantHandler } from "@better-auth/oauth-provider";
import { and, type DatabaseConnection, eq, isNull, Snapshot, Transaction } from "@destack/db";
import { PackageId } from "@destack/package";
import { type Identifier, nth, schema } from "@destack/schema";
import { TOKEN_LIFETIME_MILLISECONDS, TokenVerifier } from "@destack/runtime/authentication";
import { CallError } from "@destack/runtime/error";
import { ObjectReference, Scope, Subject } from "@destack/sync";
import { AccountAuthentication } from "../better-auth/authentication.ts";
import { account, type Account } from "../object/account.ts";
import { Identity } from "../object/identity.ts";
import * as object from "../object/index.ts";
import { Placement } from "../object/placement.ts";
import accountPackage from "../package.ts";
import { Issuance } from "./issuance.ts";
import { PackageResource } from "./resource.ts";
import type { Clock } from "@destack/environment";

/** The grant type of a token exchanged for another (RFC 8693 2.1). */
export const TOKEN_EXCHANGE = "urn:ietf:params:oauth:grant-type:token-exchange";

/** The type of a subject token that is an access token the token endpoint issued (RFC 8693 3). */
export const ACCESS_TOKEN_TYPE = "urn:ietf:params:oauth:token-type:access_token";

/** The type of a subject token that is a person's personal access token. */
export const PERSONAL_ACCESS_TOKEN_TYPE = "urn:destack:token-type:personal-access-token";

/** The type of a subject token that is a signed-in person's session token. */
export const SESSION_TOKEN_TYPE = "urn:destack:token-type:session";

/** A token exchange request: the subject token, the package and space, and whom to act for. */
const ExchangeRequest = schema.looseObject({
    /** The token exchanged. */
    subject_token: schema.string().min(1),
    /** The kind of the token exchanged. */
    subject_token_type: schema.enum([
        ACCESS_TOKEN_TYPE,
        SESSION_TOKEN_TYPE,
        PERSONAL_ACCESS_TOKEN_TYPE,
    ]),
    /** The package the token is for, as its resource (RFC 8707). */
    resource: schema.string().min(1),
    /** The space the token acts in, absent under a grant toward a universe package. */
    space: schema.identifier("space").exactOptional(),
    /** The person to act as, or for under their grant to the caller, as a subject key. */
    subject: schema.string().min(1).exactOptional(),
    /** The chain of grants to act under, space-delimited. */
    grants: schema
        .string()
        .min(1)
        .transform((value) => value.split(" "))
        .pipe(schema.array(schema.identifier("grant")))
        .exactOptional(),
});

/** What the token exchange reads and verifies with. */
export interface ExchangeOptions {
    /** The account database with the people, grants and placements. */
    readonly database: DatabaseConnection;
    /** The policies expanding memberships. */
    readonly authorizer: Authorizer;
    /** The universe's issuer, which the exchanged access tokens carry. */
    readonly issuer: string;
    /** The clock tokens are verified at. */
    readonly clock: Clock;
    /** The lifetimes of the packages' tokens, a minute when absent. */
    readonly lifetimes: ReadonlyMap<PackageId, number>;
}

/** Exchange a person's, machine's or installation's token for a token of a package (RFC 8693). */
export function exchangeGrant(options: ExchangeOptions): OAuthExtensionGrantHandler {
    return async (input) => {
        // read the request and the package it asks a token for
        const request = ExchangeRequest.safeParse(input.ctx.body);
        if (!request.success) {
            throw Issuance.refusal("invalid_request", "invalid token exchange request");
        }
        const asked = request.data;
        const audience = PackageResource.package(asked.resource);
        if (audience === undefined) {
            throw Issuance.refusal("invalid_target", `${asked.resource} names no package`);
        }

        // describe the subject token's caller toward the package, refusing an invalid token
        const now = options.clock.now();
        const { authentication, lapse } = await describe(options, asked, audience, now).catch(
            (error: unknown) => {
                const refused = CallError.of(error);
                throw refused === undefined
                    ? error
                    : Issuance.refusal("invalid_grant", refused.message);
            },
        );

        return Issuance.issue(input, authentication, {
            now,
            lifetimes: options.lifetimes,
            scopes: [],
            ...(lapse === undefined ? {} : { lapse }),
        });
    };
}

/** Describe the caller a token exchange asks a token for, and when its session or grant lapses. */
async function describe(
    options: ExchangeOptions,
    asked: schema.Infer<typeof ExchangeRequest>,
    audience: PackageId,
    now: number,
): Promise<{ readonly authentication: Authentication; readonly lapse?: number }> {
    // verify the subject token before the transaction, a session or a token lapsing with its credential
    const verified = await subjectOf(options, asked, now);
    const isAccessToken = asked.subject_token_type === ACCESS_TOKEN_TYPE;
    const lapses = isAccessToken ? [] : [verified.credential.expiresAt];
    const subject = asked.subject === undefined ? undefined : Subject.read(asked.subject);

    return options.database.transaction(async (transaction) => {
        // describe a person acting through a delegate under a grant, outside a space toward a universe package
        const isInstallation = principal.installation.is(verified.caller.subject);
        const granted = await readGrants(
            transaction,
            verified.caller,
            {
                ...(subject === undefined ? {} : { subject }),
                ...(asked.grants === undefined ? {} : { grants: asked.grants }),
            },
            isInstallation,
            now,
        );
        const expiries = granted.map((grant) => grant.expiresAt ?? Number.POSITIVE_INFINITY);
        const lapse = Math.min(...lapses, ...expiries);
        const lapsing = Number.isFinite(lapse) ? { lapse } : {};
        if (granted.length > 0 && asked.space === undefined) {
            return {
                authentication: describeDelegation(verified, granted, audience, now),
                ...lapsing,
            };
        }
        // describe the caller toward a universe package, outside any space
        else if (asked.space === undefined) {
            if (subject !== undefined) {
                throw new CallError("BAD_REQUEST", {
                    message: "a token acting as another person acts in a space",
                });
            }
            const { scope: _scope, ...credential } = verified.credential;

            return {
                authentication: {
                    caller: verified.caller,
                    credential: { ...credential, audience, verifiedAt: now },
                },
                ...lapsing,
            };
        }

        // describe the caller in the space, a person it impersonates, or a person acting through a delegate
        const verification = new SpaceVerification(asked.space, audience, transaction, now);
        if (granted.length > 0) {
            return {
                authentication: await verification.delegation(
                    verified,
                    granted,
                    options.authorizer,
                ),
                ...lapsing,
            };
        } else if (subject !== undefined) {
            return {
                authentication: await verification.impersonation(
                    verified,
                    subject,
                    options.authorizer,
                ),
                ...lapsing,
            };
        }

        return {
            authentication: await verification.authentication(verified, options.authorizer),
            ...lapsing,
        };
    });
}

/** Verify a subject token: an access token the endpoint issued, or a personal access token. */
async function subjectOf(
    options: ExchangeOptions,
    asked: schema.Infer<typeof ExchangeRequest>,
    now: number,
): Promise<Authentication> {
    // verify a personal access token or a session against the account's records
    const { subject_token: token, subject_token_type: type } = asked;
    if (type === PERSONAL_ACCESS_TOKEN_TYPE) {
        return AccountAuthentication.personalToken(token, options.database, accountPackage.id, now);
    } else if (type === SESSION_TOKEN_TYPE) {
        return AccountAuthentication.session(token, options.database, accountPackage.id, now);
    }

    // verify an access token for the account package by the universe's current keys, refusing a machine's
    const keys = await Identity.keys(Snapshot.live(options.database), Scope.universe.id);
    const verifier = new TokenVerifier({
        authority: { kind: "universe" },
        issuer: options.issuer,
        audience: accountPackage.id,
        keys,
    });

    // refuse a machine's own token, which it asks with its assertion
    const verified = await verifier.verify(token, now);
    if (principal.machine.is(verified.caller.subject)) {
        throw new CallError("FORBIDDEN", {
            message: "a machine asks its tokens with its assertion",
        });
    }

    return verified;
}

/** A description of callers for one space and receiving package, under one transaction. */
class SpaceVerification {
    /** The space the caller acts in. */
    readonly spaceId: Identifier<"space">;
    /** The receiving package. */
    readonly audience: PackageId;
    /** The transaction reading the callers' standing. */
    readonly transaction: DatabaseConnection;
    /** The time of the verification, in UTC epoch milliseconds. */
    readonly now: number;
    /** The account owning the space, read on first use. */
    #owner: Promise<Account["id"]> | undefined;

    /** Describe callers for a space and receiving package at a time. */
    constructor(
        spaceId: Identifier<"space">,
        audience: PackageId,
        transaction: DatabaseConnection,
        now: number,
    ) {
        // store the space, the audience, the transaction and the time
        this.spaceId = spaceId;
        this.audience = audience;
        this.transaction = transaction;
        this.now = now;
    }

    /** Describe a user or an installation of the account's spaces in the space. */
    async authentication(
        verified: Authentication,
        authorizer: Authorizer,
        subject: Subject = verified.caller.subject,
    ): Promise<Authentication> {
        // select the account that owns the space, of which an installation acting as itself is one
        const { caller, credential } = verified;
        const acting = caller.subject;
        const accountId = await this.#account();
        if (subject === acting) {
            await this.#requireInstalled(acting, accountId);
        }

        // resolve the subject's memberships and groups in the account and the scopes enclosing it
        const sets = await this.#sets(authorizer, subject, accountId);

        // bound the returned verification by both the credential and assertion lifetimes
        const verifiedAt = this.now;

        // store what the acting caller proved and its actors only for itself, not for a person it acts as
        const isOwn = subject === acting;

        return {
            caller: {
                subject,
                actors: isOwn ? [...caller.actors] : [],
                groups: sets,
                contacts: isOwn ? [...caller.contacts] : [],
                grants: isOwn ? [...caller.grants] : [],
            },
            credential: {
                kind: credential.kind,
                id: credential.id,
                ...(credential.client === undefined ? {} : { client: credential.client }),
                audience: this.audience,
                scope: this.spaceId,
                verifiedAt,
                expiresAt: Math.min(credential.expiresAt, verifiedAt + TOKEN_LIFETIME_MILLISECONDS),
                ...(credential.assurance === undefined ? {} : { assurance: credential.assurance }),
                ...(isOwn && credential.deployments !== undefined
                    ? { deployments: [...credential.deployments] }
                    : {}),
            },
        };
    }

    /** Read a subject's memberships and groups in the account and its enclosing scopes. */
    async #sets(authorizer: Authorizer, subject: Subject, accountId: Account["id"]) {
        // expand the subject across the account's scope chain
        const snapshot = Snapshot.live(this.transaction);
        const chain = await Scope.chain(snapshot, accountId);
        const { subjects: expanded } = await Access.expand(
            snapshot,
            [subject],
            { subject, now: this.now, linkSecrets: [] },
            authorizer,
            chain.map((link) => link.object),
        );

        return expanded
            .filter(
                (entry) =>
                    entry.relation !== undefined &&
                    entry.packageId === object.account.typeReference.packageId &&
                    (entry.scope === accountId ||
                        (entry.type === object.account.name && entry.id === accountId)),
            )
            .toSorted((left, right) => (Subject.key(left) < Subject.key(right) ? -1 : 1));
    }

    /** Describe a user acting as another user in the space. */
    async impersonation(
        verified: Authentication,
        subject: Subject,
        authorizer: Authorizer,
    ): Promise<Authentication> {
        // require a user acting as another user
        const acting = verified.caller.subject;
        if (
            !principal.user.is(acting) ||
            !principal.user.is(subject) ||
            Subject.same(acting, subject)
        ) {
            throw new CallError("FORBIDDEN", { message: "only a user acts as another user" });
        }

        // describe the represented user within the space's account
        const represented = await this.authentication(verified, authorizer, subject);
        const accountId = await this.#account();
        const isMember = represented.caller.groups.some(
            (entry) =>
                entry.packageId === object.account.typeReference.packageId &&
                entry.type === object.account.name &&
                entry.id === accountId &&
                entry.relation === "member",
        );

        // require the lent authority or administration of the member's account
        const { now } = this;
        Credential.requireCurrent(verified.credential, accountPackage.id, now);
        const reading = Transaction.reading(this.transaction);
        const authorization = new Authorization(authorizer, reading, () =>
            AccessContext.of(verified.caller, now, { credential: verified.credential }),
        );
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
            throw new CallError("FORBIDDEN", {
                message: "acting as another user is not allowed",
            });
        }

        return { ...represented, caller: { ...represented.caller, actors: [acting] } };
    }

    /** Describe a person acting through a chain of delegates under their grants in the space. */
    async delegation(
        verified: Authentication,
        chain: readonly Grant[],
        authorizer: Authorizer,
    ): Promise<Authentication> {
        // describe the first delegator within the space's account, without what it proved elsewhere
        const represented = await this.authentication(
            verified,
            authorizer,
            ObjectReference.at("delegator", nth(chain, 0)),
        );

        // act through the delegates under the grants, until the first grant lapses
        const { credential } = represented;
        const lapse = Math.min(...chain.map((grant) => grant.expiresAt ?? credential.expiresAt));

        return {
            caller: {
                ...represented.caller,
                actors: chain.map((grant) => ObjectReference.at("delegate", grant)),
                contacts: [],
                grants: chain.map((grant) => Grant.claim(grant)),
            },
            credential: { ...credential, expiresAt: Math.min(credential.expiresAt, lapse) },
        };
    }

    /** Require an acting installation to live in a space of the account. */
    async #requireInstalled(acting: Subject, accountId: Account["id"]): Promise<void> {
        // leave every other principal to its credential
        if (!principal.installation.is(acting)) {
            return;
        }

        // locate the installation's space within the account
        const [placement] = await Placement.find(Snapshot.live(this.transaction), [
            schema.identifier("space").parse(acting.scope),
        ]);
        if (placement?.scope !== accountId) {
            throw new CallError("FORBIDDEN", {
                message: "the installation belongs to another account",
            });
        }
    }

    /** Read the live account that owns the space, once per verification. */
    #account(): Promise<Account["id"]> {
        this.#owner ??= this.#readAccount();

        return this.#owner;
    }

    /** Read the live account that owns the space. */
    async #readAccount(): Promise<Account["id"]> {
        // locate the space's placement and require its account
        const [placement] = await Placement.find(Snapshot.live(this.transaction), [this.spaceId]);
        const selected =
            placement === undefined
                ? undefined
                : await this.transaction
                      .select({ accountId: account.table.id })
                      .from(account.table)
                      .innerJoin(Scope.table, eq(Scope.table.scope, account.table.id))
                      .where(
                          and(
                              eq(
                                  account.table.id,
                                  schema.identifier("account").parse(placement.scope),
                              ),
                              isNull(Scope.table.suspendedAt),
                              isNull(account.table.deletedAt),
                          ),
                      )
                      .get();
        if (selected === undefined) {
            throw new CallError("FORBIDDEN", { message: "the space has no active account" });
        }

        return selected.accountId;
    }
}

/** Describe a person acting through delegates under their grants toward a universe package. */
function describeDelegation(
    verified: Authentication,
    chain: readonly Grant[],
    audience: PackageId,
    now: number,
): Authentication {
    // lapse at the earliest of the credential, the token lifetime and the grants
    const { credential } = verified;
    const lapse = Math.min(
        credential.expiresAt,
        now + TOKEN_LIFETIME_MILLISECONDS,
        ...chain.map((grant) => grant.expiresAt ?? Number.POSITIVE_INFINITY),
    );

    // act as the first grant's delegator through each delegate
    const delegator = ObjectReference.at("delegator", nth(chain, 0));

    return {
        caller: {
            ...Caller.of(delegator),
            actors: chain.map((grant) => ObjectReference.at("delegate", grant)),
            grants: chain.map((grant) => Grant.claim(grant)),
        },
        credential: {
            kind: credential.kind,
            id: credential.id,
            audience,
            verifiedAt: now,
            expiresAt: lapse,
        },
    };
}

/** Read the chain of grants an exchange acts under, from the caller or from a person to it. */
async function readGrants(
    transaction: DatabaseConnection,
    caller: Caller,
    input: { readonly subject?: Subject; readonly grants?: readonly Identifier<"grant">[] },
    isInstallation: boolean,
    now: number,
): Promise<Grant[]> {
    // require a person's named chain to the calling installation
    const snapshot = Snapshot.live(transaction);
    if (isInstallation && input.subject !== undefined && input.grants !== undefined) {
        return requireChain(snapshot, input.grants, input.subject, caller.subject, now);
    }
    // require the person's only live grant to the calling installation
    else if (isInstallation && input.subject !== undefined) {
        const parties = { delegator: input.subject, delegate: caller.subject };
        const grants = await snapshot.read(accessGrant).findMany({
            where: {
                scope: parties.delegator.id,
                ...ObjectReference.columns("delegate", caller.subject),
            },
        });
        const live = grants.filter((grant) => grant.expiresAt === null || grant.expiresAt > now);
        const [only, ...others] = live;

        // refuse none or several
        if (only === undefined || others.length > 0) {
            throw new CallError("FORBIDDEN", {
                message:
                    live.length === 0
                        ? `${parties.delegator.id} has no grant to ${caller.subject.id}`
                        : `${parties.delegator.id} has ${live.length} grants to ${caller.subject.id}, name one`,
            });
        }

        return [Grant.require(only, only.id, parties, now)];
    }
    // read the caller's own chain to a delegate, as the caller acting as itself
    else if (input.grants !== undefined) {
        if (Caller.isDelegated(caller)) {
            throw new CallError("FORBIDDEN", {
                message: "a delegate acts under its own grants alone",
            });
        }

        return requireChain(snapshot, input.grants, caller.subject, undefined, now);
    }

    return [];
}

/** Require a chain of live grants from a delegator, each from the prior grant's delegate. */
async function requireChain(
    snapshot: Snapshot,
    ids: readonly Identifier<"grant">[],
    delegator: Subject,
    delegate: Subject | undefined,
    now: number,
): Promise<Grant[]> {
    const chain: Grant[] = [];
    for (const [index, id] of ids.entries()) {
        // link each grant from the prior grant's delegate, the last one to the given delegate
        const grant = await Grant.read(snapshot, id);
        const from =
            index === 0 ? delegator : ObjectReference.at("delegate", nth(chain, index - 1));
        const last = index === ids.length - 1 ? delegate : undefined;
        const own = grant === undefined ? from : ObjectReference.at("delegate", grant);
        chain.push(Grant.require(grant, id, { delegator: from, delegate: last ?? own }, now));
    }

    return chain;
}
