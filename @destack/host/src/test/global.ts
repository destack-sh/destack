import { exportJWK, generateKeyPair, SignJWT, type JSONWebKeySet } from "jose";
import { directoryTables } from "@destack/directory";
import { Scope, type ObjectReference, type Subject } from "@destack/sync";
import { RequestId } from "@destack/service/request";
import { accessRelationship, principal, Relationship } from "@destack/access";
import { copyOwner, copyRole, copyScope } from "@destack/access/test";
import { createAuthenticator } from "@destack/account/authentication";
import { accountTables } from "@destack/account/stack";
import { accountService } from "@destack/account";
import { ServiceMount } from "@destack/service";
import { account, region, session, user } from "@destack/account/object";
import { AccountAuthentication } from "@destack/account/authentication";
import * as accountServer from "@destack/account/server";
import type { AuditDestination } from "@destack/audit";
import type { ObjectType } from "@destack/object";
import { and, eq, gt, type DatabaseConnection, defineDatabase, type Database } from "@destack/db";
import { ServiceError } from "@destack/service/error";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { ResourceContext } from "@destack/resource/context";
import { found, schema, Identifier } from "@destack/schema";
import {
    Authentication,
    TokenVerifier,
    type TokenIssuerOptions,
} from "@destack/service/authentication";
import { Health } from "@destack/service/health";
import { Server, type ServiceImplementation } from "@destack/service/server";
import { connect } from "@destack/account/client";
import { HostIdentity } from "../identity/index.ts";
import { MemoryKeychain } from "../keychain/index.ts";
import { HostKey } from "@destack/account/object";
import { testCallKey } from "@destack/service/test";

/** The global tier's origin, the issuer of enrolled hosts. */
export const ISSUER = "https://global.destack.test";

/** The global tier's database: accounts and hosts. */
export const globalDatabase = defineDatabase({
    name: "global",
    tier: "global",
    tables: [...accountTables, ...directoryTables],
});

/** The URL the fixture's account service answers at. */
export const ACCOUNTS_URL = "https://accounts.test";

/** How long a fixture session lasts, a day in milliseconds. */
const SESSION_MILLISECONDS = 24 * 60 * 60 * 1000;

/** The identities the global tier keeps. */
export const ids = {
    owner: schema.identifier("user").parse("user-01996ab0-0000-7000-8000-00000000000a"),
    stranger: schema.identifier("user").parse("user-01996ab0-0000-7000-8000-00000000000b"),
    operator: schema.identifier("user").parse("user-01996ab0-0000-7000-8000-00000000000c"),
    account: schema.identifier("account").parse("account-01996ab0-0000-7000-8000-000000000001"),
    other: schema.identifier("account").parse("account-01996ab0-0000-7000-8000-000000000002"),
    platform: schema.identifier("account").parse("account-01996ab0-0000-7000-8000-000000000003"),
    region: schema.identifier("region").parse("region-01996ab0-0000-7000-8000-000000000004"),
};

/** The global tier in one database, serving accounts with their hosts. */
export class GlobalFixture implements AsyncDisposable {
    /** The global test database. */
    readonly storage: TestDatabase;
    /** The global database. */
    readonly database: DatabaseConnection;
    /** The account service. */
    readonly accounts: Server;
    /** The public keys of the universe's token authority. */
    readonly keys: JSONWebKeySet;

    /** The empty audit history the served services deliver to. */
    static readonly history: AuditDestination = { ingest: async () => 0 };

    /** Hold a served global tier. */
    private constructor(storage: TestDatabase, accounts: Server, keys: JSONWebKeySet) {
        // keep the database, the account service and the token authority's keys
        this.storage = storage;
        this.database = storage.database;
        this.accounts = accounts;
        this.keys = keys;
    }

    /** Serve the global tier with the owner's accounts and the region, relaying the rows of some inherited types. */
    static async open(
        definition: Database = globalDatabase,
        inherited: readonly ObjectType[] = [],
    ): Promise<GlobalFixture> {
        // keep the global database in the last test dialect
        const dialect = TEST_DIALECTS.at(-1);
        if (dialect === undefined) {
            throw new TypeError("the global fixture needs a test dialect");
        }
        const storage = await TestDatabase.create(dialect, definition, {
            isMigrated: true,
        });
        const database = storage.database;
        const record = { createdAt: 1, updatedAt: 1 };

        // keep the owner of every account and the operator of Destack's regions
        const [owner, operator] = [
            principal.user.reference(Scope.universe.id, ids.owner),
            principal.user.reference(Scope.universe.id, ids.operator),
        ];
        for (const [userId, person, name] of [
            [ids.owner, owner, "Owner"],
            [ids.operator, operator, "Operator"],
        ] as const) {
            await database.insert(user.table).values({
                ...record,
                id: userId,
                name,
                email: `${userId}@example.test`,
            });
            await copyScope(database, person);
            await copyOwner(database, person, person);
        }

        // make the owner the owner of every account, and let its hosts read it
        const ownerRoles = new Map<string, string>();
        for (const [id, handle] of [
            [ids.account, "acme"],
            [ids.other, "rival"],
            [ids.platform, "destack"],
        ] as const) {
            await database.insert(account.table).values({
                ...record,
                id,
                handle,
                name: handle,
                defaultResidency: "eu",
                kind: id === ids.account ? ("personal" as const) : ("shared" as const),
                scope: ids.owner,
            });
            const scope = account.reference(ids.owner, id);
            await copyScope(database, scope);
            ownerRoles.set(id, await copyOwner(database, scope, owner));
            await GlobalFixture.relate(
                database,
                id,
                scope,
                "host",
                principal.host.reference(id, "*"),
            );
        }

        // keep the platform's region, and make the operator the owner of the platform and the server of the region
        await database.insert(region.table).values({
            ...record,
            id: ids.region,
            code: "eu-central",
            name: "Europe",
            residency: "eu",
        });
        await copyRole(
            database,
            Scope.universe,
            {
                name: "operator",
                description: "Serve Destack's regions with hosts",
                permissions: [region.policy.permission("serve")],
            },
            operator,
        );
        await GlobalFixture.relate(
            database,
            ids.platform,
            account.reference(ids.owner, ids.platform),
            { role: found(ownerRoles, ids.platform) },
            operator,
        );

        // sign the universe's tokens with a fixture key
        const pair = await generateKeyPair("ES256");
        const keys = {
            keys: [{ ...(await exportJWK(pair.publicKey)), kid: "universe", alg: "ES256" }],
        };
        const tokens: Pick<TokenIssuerOptions, "issuer" | "sign"> = {
            issuer: ISSUER,
            sign: (payload) =>
                new SignJWT(payload)
                    .setProtectedHeader({ alg: "ES256", kid: "universe" })
                    .sign(pair.privateKey),
        };

        // serve the account service to hosts by their tokens
        const authenticator = createAuthenticator({
            callKey: testCallKey,
            database,
            origin: ISSUER,
            trustedOrigins: [ISSUER],
            ipAddress: { disableIpTracking: true },
            secret: "global-host-test-secret-32-characters-minimum",
            providers: {},
            signInUri: `${ISSUER}/sign-in`,
            consentUri: `${ISSUER}/consent`,
            verificationUri: `${ISSUER}/device`,
            secondFactorUri: `${ISSUER}/sign-in/two-factor`,
            handleUri: `${ISSUER}/sign-in/handle`,
            service: (request) => accounts.fetch(request),
            sendMagicLink: async () => {},
            sendCode: async () => {},
        });
        const connections = new accountServer.Connections({
            providers: [],
            vault: {
                write: async () => {
                    throw new TypeError("the fixture keeps no connection credentials");
                },
                read: async () => {
                    throw new TypeError("the fixture keeps no connection credentials");
                },
                destroy: async () => {
                    throw new TypeError("the fixture keeps no connection credentials");
                },
            },
        });
        const accountImplementation = accountServer.implementService(authenticator, {
            callKey: testCallKey,
            connections,
            history: GlobalFixture.history,
            inherited,
            resolver: { txt: async () => [] },
            tokens,
        });
        const accounts = GlobalFixture.serve(accountImplementation, database, keys);

        return new GlobalFixture(storage, accounts, keys);
    }

    /** Route a request of the global tier's origin as the universe does: by mount, else to the account service's issuer paths. */
    fetch(request: Request): Promise<Response> {
        // find the mounted service
        const routed = ServiceMount.route(request);

        // answer sign-in at the issuer's paths
        if (routed === undefined) {
            return this.accounts.fetch(request);
        }
        // hand the request over below the account mount
        else if (routed.packageId === accountService.package.id) {
            return this.accounts.fetch(routed.request);
        }
        // refuse a mount of no served package
        else {
            return Promise.resolve(new Response(null, { status: 404 }));
        }
    }

    /** Sign a user in, returning the bearer token of the session both services accept. */
    async signIn(userId: Identifier<"user">): Promise<string> {
        // record an active session of the user
        const id = Identifier.create("session");
        const now = Date.now();
        await this.database.insert(session.table).values({
            id,
            scope: userId,
            userId,
            token: crypto.randomUUID(),
            expiresAt: now + SESSION_MILLISECONDS,
            createdAt: now,
            updatedAt: now,
        });

        return id;
    }

    /** Connect to the account service as a user. */
    user(userId: string) {
        return connect({
            url: ACCOUNTS_URL,
            headers: { "x-user": userId },
            fetch: (request) => this.accounts.fetch(request),
        });
    }

    /** Connect to the account service as a host. */
    host(identity: HostIdentity) {
        return connect({
            url: ACCOUNTS_URL,
            fetch: identity.fetch(
                (request) => this.accounts.fetch(request),
                accountService.package.id,
                ACCOUNTS_URL,
            ),
        });
    }

    /** Relate a subject to an object by a relation or a role, as a grant in a scope. */
    static async relate(
        database: DatabaseConnection,
        scope: string,
        object: ObjectReference,
        binding: string | { readonly role: string },
        subject: Subject,
    ): Promise<void> {
        await database.insert(accessRelationship).values(
            Relationship.encode(
                {
                    id: Identifier.create("relationship"),
                    object,
                    ...(typeof binding === "string" ? { relation: binding } : binding),
                    subject,
                    createdAt: 1,
                    expiresAt: null,
                },
                scope,
            ),
        );
    }

    /** Enroll a new host under an account, the platform's for its region as the operator, returning its identity. */
    async enroll(accountId: string, kind: "device" | "cloud" = "cloud"): Promise<HostIdentity> {
        // enroll a platform host for its region as the operator, any other as the owner
        const identity = new HostIdentity(Identifier.create("host"), new MemoryKeychain());
        const enroller = accountId === ids.platform ? ids.operator : ids.owner;
        await identity.enroll(this.user(enroller), {
            accountId: schema.identifier("account").parse(accountId),
            requestId: RequestId.create(),
            name: `machine-${identity.hostId.slice(-12)}`,
            kind,
            ...(accountId === ids.platform ? { regionId: ids.region } : {}),
        });

        return identity;
    }

    /** Serve an implementation to hosts by their tokens and to users by session or header. */
    static serve(
        implementation: ServiceImplementation,
        database: DatabaseConnection,
        keys: JSONWebKeySet,
    ): Server {
        const audience = implementation.service.package.id;
        const verifier = new TokenVerifier({
            authority: { kind: "universe" },
            issuer: ISSUER,
            audience,
            keys,
        });

        return Server.start({
            ...implementation,
            audience,
            resources: new ResourceContext(),
            health: new Health("universe"),
            drainTimeout: 1000,
            authorizeHost: async () => {},
            authenticate: async (request) => {
                // verify a token the universe signed
                const authorization = request.headers.get("authorization") ?? "";
                if (/^Bearer \S+\.\S+\.\S+$/u.test(authorization)) {
                    const authentication = await verifier.authenticate(request);
                    await HostKey.requireAuthenticating(database, authentication, Date.now());

                    return authentication;
                }

                // take the user of an unexpired signed-in session, refusing any other bearer
                const bearer = /^Bearer (session-\S+)$/u.exec(
                    request.headers.get("authorization") ?? "",
                );
                if (bearer !== null) {
                    const sessionId = schema.identifier("session").safeParse(bearer[1]);
                    const now = Date.now();
                    const [signedIn] = sessionId.success
                        ? await database
                              .select({ userId: session.table.userId })
                              .from(session.table)
                              .where(
                                  and(
                                      eq(session.table.id, sessionId.data),
                                      gt(session.table.expiresAt, now),
                                  ),
                              )
                        : [];
                    if (!sessionId.success || signedIn === undefined) {
                        throw new ServiceError("UNAUTHORIZED", {
                            message: "session is unknown or expired",
                        });
                    }
                    const subject = principal.user.reference(Scope.universe.id, signedIn.userId);

                    return new AccountAuthentication({
                        credential: {
                            kind: "session",
                            id: sessionId.data,
                            userId: signedIn.userId,
                        },
                        audience,
                        subject,
                        subjects: [subject],
                        verifiedAt: now,
                        expiresAt: now + 60_000,
                    });
                }

                // take the user from the fixture header
                const named = request.headers.get("x-user");
                if (named === null) {
                    return null;
                }
                const subject = principal.user.reference(Scope.universe.id, named);
                const now = Date.now();

                return new Authentication({
                    credential: { kind: "fixture", id: named },
                    audience,
                    subject,
                    subjects: [subject],
                    verifiedAt: now,
                    expiresAt: now + 60_000,
                });
            },
        });
    }

    /** Stop the account service, then close the database. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.accounts.close();
        await this.storage.close();
    }
}
