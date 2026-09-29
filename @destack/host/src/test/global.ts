import { v7 } from "uuid";
import { directoryTables } from "@destack/directory";
import { Scope, type ObjectReference } from "@destack/sync";
import { RequestId } from "@destack/service/request";
import { accessRelationship, principal, Relationship, type Subject } from "@destack/access";
import { copyOwner, copyRole, copyScope } from "@destack/access/test";
import { createAuthentication } from "@destack/account/authentication";
import { accountTables } from "@destack/account/stack";
import { accountService } from "@destack/account";
import { ServiceMount } from "@destack/service";
import { hostService } from "../service/index.ts";
import { account, region, session, user } from "@destack/account/object";
import { AccountCaller } from "@destack/account/authentication";
import * as accountServer from "@destack/account/server";
import type { AuditDestination } from "@destack/audit/outbox";
import type { ObjectType } from "@destack/object";
import { and, eq, gt, type DatabaseConnection } from "@destack/db";
import { ServiceError } from "@destack/service/error";
import { defineDatabase, type Database } from "@destack/db/declare";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { ResourceContext } from "@destack/resource/context";
import { identifier } from "@destack/schema";
import { Caller } from "@destack/service/authentication";
import { Health } from "@destack/service/health";
import { Server, type ServiceImplementation } from "@destack/service/server";
import { connect } from "../client/index.ts";
import { HostCaller, HostIdentity } from "../identity/index.ts";
import { MemoryKeychain } from "../keychain/index.ts";
import { implementService } from "../server/index.ts";
import { hostTables } from "../stack/index.ts";

/** The global tier's origin, the issuer of enrolled hosts. */
export const ISSUER = "https://global.destack.test";

/** The global tier's database: accounts and hosts. */
export const globalDatabase = defineDatabase({
    name: "global",
    tier: "global",
    tables: [...accountTables, ...directoryTables, ...hostTables],
});

/** How long a fixture session lasts, a day in milliseconds. */
const SESSION_MILLISECONDS = 24 * 60 * 60 * 1000;

/** The identities the global tier holds. */
export const ids = {
    owner: identifier("user").parse("user-01996ab0-0000-7000-8000-00000000000a"),
    stranger: identifier("user").parse("user-01996ab0-0000-7000-8000-00000000000b"),
    operator: identifier("user").parse("user-01996ab0-0000-7000-8000-00000000000c"),
    account: identifier("account").parse("account-01996ab0-0000-7000-8000-000000000001"),
    other: identifier("account").parse("account-01996ab0-0000-7000-8000-000000000002"),
    platform: identifier("account").parse("account-01996ab0-0000-7000-8000-000000000003"),
    region: identifier("region").parse("region-01996ab0-0000-7000-8000-000000000004"),
};

/** The global tier in one database, serving hosts and accounts. */
export class GlobalFixture implements AsyncDisposable {
    /** The global test database. */
    readonly storage: TestDatabase;
    /** The global database. */
    readonly database: DatabaseConnection;
    /** The host service. */
    readonly hosts: Server;
    /** The account service. */
    readonly accounts: Server;

    /** The empty audit history the served services deliver to. */
    static readonly history: AuditDestination = { ingest: async () => 0 };

    /** Hold a served global tier. */
    private constructor(storage: TestDatabase, hosts: Server, accounts: Server) {
        // hold the database and both services
        this.storage = storage;
        this.database = storage.database;
        this.hosts = hosts;
        this.accounts = accounts;
    }

    /** Serve the global tier with the owner's accounts and the region, relaying the rows of some inherited types. */
    static async open(
        definition: Database = globalDatabase,
        inherited: readonly ObjectType[] = [],
    ): Promise<GlobalFixture> {
        // hold the global database
        const storage = await TestDatabase.create(TEST_DIALECTS.at(-1)!, definition, {
            isMigrated: true,
        });
        const database = storage.database;
        const record = { createdAt: 1, updatedAt: 1 };

        // keep the owner of every account and the operator of Destack's regions
        const [owner, operator] = [
            principal.user.reference(Scope.universe.id, ids.owner),
            principal.user.reference(Scope.universe.id, ids.operator),
        ];
        for (const [person, name] of [
            [owner, "Owner"],
            [operator, "Operator"],
        ] as const) {
            await database.insert(user.table).values({
                ...record,
                id: person.id as never,
                name,
                email: `${person.id}@example.test`,
            });
            await copyScope(database, person);
            await copyOwner(database, person, person);
        }

        // let the owner own every account and its hosts read it
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

        // keep the platform's region, and let the operator own the platform and serve the region
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
            { role: ownerRoles.get(ids.platform)! },
            operator,
        );

        // serve hosts to users and to hosts by their proofs
        const hosts = GlobalFixture.serve(implementService({ database }), database);

        // serve the account service to hosts by their proofs
        const authentication = createAuthentication({
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
        const accountImplementation = accountServer.implementService(authentication, {
            connections,
            history: GlobalFixture.history,
            inherited,
        });
        const accounts = GlobalFixture.serve(accountImplementation, database);

        return new GlobalFixture(storage, hosts, accounts);
    }

    /** Route a request of the global tier's origin as the universe does: by mount, else to the account service's issuer paths. */
    fetch(request: Request): Promise<Response> {
        // find the mounted service
        const routed = ServiceMount.route(request);

        // answer sign-in at the issuer's paths
        if (routed === undefined) {
            return this.accounts.fetch(request);
        }
        // hand the request over below the host or account mount
        else if (routed.packageId === hostService.package.id) {
            return this.hosts.fetch(routed.request);
        } else if (routed.packageId === accountService.package.id) {
            return this.accounts.fetch(routed.request);
        }
        // refuse a mount of no served package
        else {
            return Promise.resolve(new Response(null, { status: 404 }));
        }
    }

    /** Sign a user in, returning the bearer token of the session both services accept. */
    async signIn(userId: string): Promise<string> {
        // record an active session of the user
        const id = identifier("session").parse(`session-${v7()}`);
        const now = Date.now();
        await this.database.insert(session.table).values({
            id,
            scope: userId as never,
            userId: userId as never,
            token: crypto.randomUUID(),
            expiresAt: now + SESSION_MILLISECONDS,
            createdAt: now,
            updatedAt: now,
        });

        return id;
    }

    /** Connect to the host service as a user. */
    user(userId: string) {
        return connect({
            url: "https://hosts.test",
            headers: { "x-user": userId },
            fetch: (request) => this.hosts.fetch(request),
        });
    }

    /** Connect to the host service as a host. */
    host(identity: HostIdentity) {
        return connect({
            url: "https://hosts.test",
            fetch: identity.fetch((request) => this.hosts.fetch(request)),
        });
    }

    /** Relate a subject to an object by a relation or a role, as a grant held in a scope. */
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
                    id: identifier("relationship").parse(`relationship-${v7()}`),
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
        const identity = new HostIdentity(`host-${v7()}`, new MemoryKeychain());
        const enroller = accountId === ids.platform ? ids.operator : ids.owner;
        await identity.enroll(this.user(enroller), {
            accountId: identifier("account").parse(accountId),
            requestId: RequestId.create(),
            name: `machine-${identity.hostId.slice(-12)}`,
            kind,
            ...(accountId === ids.platform ? { region: ids.region } : {}),
        });

        return identity;
    }

    /** Serve an implementation to hosts by their proofs and to users by session or header. */
    static serve(implementation: ServiceImplementation, database: DatabaseConnection): Server {
        const audience = implementation.service.package.id;

        return Server.start({
            ...implementation,
            audience,
            resources: new ResourceContext(),
            health: new Health("universe"),
            drainTimeout: 1000,
            authorizeHost: async () => {},
            authenticate: async (request) => {
                // verify a host by its proof
                if (HostCaller.accepts(request)) {
                    return HostCaller.authenticate(request, database, audience);
                }

                // take the user of an unexpired signed-in session, refusing any other bearer
                const bearer = /^Bearer (session-\S+)$/.exec(
                    request.headers.get("authorization") ?? "",
                );
                if (bearer !== null) {
                    const now = Date.now();
                    const [signedIn] = await database
                        .select({ userId: session.table.userId })
                        .from(session.table)
                        .where(
                            and(
                                eq(session.table.id, bearer[1] as never),
                                gt(session.table.expiresAt, now),
                            ),
                        );
                    if (signedIn === undefined) {
                        throw new ServiceError("UNAUTHORIZED", {
                            message: "session is unknown or expired",
                        });
                    }
                    const subject = principal.user.reference(Scope.universe.id, signedIn.userId);

                    return new AccountCaller({
                        credential: {
                            kind: "session",
                            id: bearer[1] as never,
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

                return new Caller({
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

    /** Stop both services, then close the database. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.hosts.close();
        await this.accounts.close();
        await this.storage.close();
    }
}
