import type { JSONWebKeySet } from "jose";
import { Replica, Scope, type ObjectReference, type Subject } from "@destack/sync";
import { RequestId } from "@destack/service/request";
import {
    accessRelationship,
    type PermissionReference,
    type Policy,
    principal,
    Relationship,
} from "@destack/access";
import { AccessFixture } from "@destack/access/test";
import {
    AccountAuthentication,
    ActiveSession,
    createAuthenticator,
} from "@destack/account/better-auth";
import { accountDatabase } from "@destack/account/stack";
import { accountService } from "@destack/account";
import { ServiceMount } from "@destack/service";
import {
    account,
    Account,
    placement,
    region,
    Residency,
    session,
    user,
} from "@destack/account/object";
import * as accountServer from "@destack/account/server";
import type { AuditDestination } from "@destack/audit/server";
import type { ObjectType } from "@destack/object";
import type { ObjectServer } from "@destack/object/server";
import type { PackageId } from "@destack/package";
import { and, eq, gt, type DatabaseConnection, type Database } from "@destack/db";
import { ServiceError } from "@destack/service/error";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { ResourceContext } from "@destack/resource/context";
import { found, present, schema, Identifier } from "@destack/schema";
import {
    Authentication,
    AUTHENTICATION_LIFETIME_MILLISECONDS,
    TokenIssuer,
    TokenVerifier,
    type TokenIssuerOptions,
} from "@destack/service/authentication";
import { Health } from "@destack/service/health";
import { Server, type ServiceImplementation } from "@destack/service/server";
import { connect, WorkloadIdentity } from "@destack/account/client";
import { DirectoryStore } from "@destack/directory";
import { HostIdentity, SpaceToken } from "../identity/index.ts";
import { MemoryKeychain } from "../keychain/index.ts";
import { HostKey } from "@destack/account/object";
import { testCallKey } from "@destack/service/test";

/** The account service's origin, the issuer of enrolled hosts. */
export const ISSUER = "https://universe.destack.test";

/** The URL the fixture's account service answers at. */
export const ACCOUNTS_URL = "https://accounts.test";

/** How long a placed workload's copies take at most to catch up with the account service, in milliseconds. */
const SETTLE_MILLISECONDS = 5000;

/** How long a fixture session lasts, a day in milliseconds. */
const SESSION_MILLISECONDS = 24 * 60 * 60 * 1000;

/** How long a fixture's authentication by session or header stays valid, a minute in milliseconds. */
const FIXTURE_AUTHENTICATION_MILLISECONDS = 60_000;

/** The creation and update times of the rows the fixture keeps. */
const RECORD_TIMES = { createdAt: 1, updatedAt: 1 };

/** The identities the account service keeps. */
export const ids = {
    owner: schema.identifier("user").parse("user-01996ab0-0000-7000-8000-00000000000a"),
    stranger: schema.identifier("user").parse("user-01996ab0-0000-7000-8000-00000000000b"),
    operator: schema.identifier("user").parse("user-01996ab0-0000-7000-8000-00000000000c"),
    account: schema.identifier("account").parse("account-01996ab0-0000-7000-8000-000000000001"),
    other: schema.identifier("account").parse("account-01996ab0-0000-7000-8000-000000000002"),
    platform: schema.identifier("account").parse("account-01996ab0-0000-7000-8000-000000000003"),
    region: schema.identifier("region").parse("region-01996ab0-0000-7000-8000-000000000004"),
};

/** The account service in one database, serving accounts with their hosts. */
export class AccountFixture implements AsyncDisposable {
    /** The account service's test database. */
    readonly storage: TestDatabase;
    /** The account service's database. */
    readonly database: DatabaseConnection;
    /** The account service. */
    readonly server: Server;
    /** The public keys of the universe's token authority. */
    readonly keys: JSONWebKeySet;
    /** The account service's object server, which seeds system records such as placements. */
    readonly objects: ObjectServer;
    /** The issuer of the universe's tokens, signing with the sign-in's key set. */
    readonly #tokens: TokenIssuer;

    /** The empty audit history the served services deliver to. */
    static readonly history: AuditDestination = { ingest: async () => 0 };

    /** Hold a served account service. */
    private constructor(
        storage: TestDatabase,
        server: Server,
        keys: JSONWebKeySet,
        objects: ObjectServer,
        tokens: TokenIssuer,
    ) {
        // keep the served account service with its database, keys and issuer
        this.storage = storage;
        this.database = storage.database;
        this.server = server;
        this.keys = keys;
        this.objects = objects;
        this.#tokens = tokens;
    }

    /** Serve the account service with the owner's accounts and the platform's region. */
    static async open(
        options: {
            /** The account service's database, accountDatabase by default. */
            readonly database?: Database;
            /** The other object types with inherited rows the database holds and relays. */
            readonly inherited?: readonly ObjectType[];
            /** The policies of other packages' types whose permissions the accounts' roles grant. */
            readonly policies?: readonly Policy[];
            /** The providers accounts connect external accounts at, keeping no credentials. */
            readonly providers?: readonly accountServer.ConnectionProvider[];
        } = {},
    ): Promise<AccountFixture> {
        // keep the account database in the last test dialect
        const dialect = TEST_DIALECTS.at(-1);
        if (dialect === undefined) {
            throw new TypeError("the account fixture needs a test dialect");
        }
        const storage = await TestDatabase.create(dialect, options.database ?? accountDatabase, {
            isMigrated: true,
        });
        const database = storage.database;

        // keep each residency, as the platform's bootstrap seeds them
        await Residency.keep(database, { code: "eu", name: "European Union" }, 1);
        await Residency.keep(database, { code: "us", name: "United States" }, 1);

        // keep the people, their accounts and the platform's region
        const { owner, operator } = await AccountFixture.#keepPeople(database);
        const ownerRoles = await AccountFixture.#keepAccounts(database, owner);
        await AccountFixture.#keepRegion(database, operator, found(ownerRoles, ids.platform));

        // serve the account service to hosts by their tokens
        const authenticator = AccountFixture.#authenticator(database, (request) =>
            server.fetch(request),
        );

        // sign every universe token with the sign-in's one key set
        const tokens: Pick<TokenIssuerOptions, "issuer" | "sign"> = {
            issuer: ISSUER,
            sign: async (payload) => (await authenticator.api.signJWT({ body: { payload } })).token,
        };
        const keys = await authenticator.api.getJwks();
        const accountImplementation = accountServer.implementAccount({
            authentication: authenticator,
            callKey: testCallKey,
            connections: AccountFixture.#connections(options.providers ?? []),
            history: AccountFixture.history,
            inherited: options.inherited ?? [],
            policies: options.policies ?? [],
            resolver: { txt: async () => [] },
        });
        const server = AccountFixture.serve(accountImplementation, database, keys);

        return new AccountFixture(
            storage,
            server,
            keys,
            accountImplementation.objects,
            new TokenIssuer({ ...tokens, authority: { kind: "universe" } }),
        );
    }

    /** Sign people in at the fixture's issuer, sending neither links nor codes. */
    static #authenticator(
        database: DatabaseConnection,
        service: (request: Request) => Promise<Response>,
    ): ReturnType<typeof createAuthenticator> {
        return createAuthenticator({
            callKey: testCallKey,
            database,
            origin: ISSUER,
            trustedOrigins: [ISSUER],
            ipAddress: { disableIpTracking: true },
            secret: "account-host-test-secret-32-characters-minimum",
            providers: {},
            signInUri: `${ISSUER}/sign-in`,
            consentUri: `${ISSUER}/consent`,
            verificationUri: `${ISSUER}/device`,
            secondFactorUri: `${ISSUER}/sign-in/two-factor`,
            handleUri: `${ISSUER}/sign-in/handle`,
            service,
            sendMagicLink: async () => {},
            sendCode: async () => {},
        });
    }

    /** Keep the owner of every account and the operator of Destack's regions, each a person in the universe. */
    static async #keepPeople(
        database: DatabaseConnection,
    ): Promise<{ owner: Subject; operator: Subject }> {
        // describe both people
        const [owner, operator] = [
            principal.user.reference(Scope.universe.id, ids.owner),
            principal.user.reference(Scope.universe.id, ids.operator),
        ];

        // keep each one as a user owning its own scope
        for (const [userId, person, name] of [
            [ids.owner, owner, "Owner"],
            [ids.operator, operator, "Operator"],
        ] as const) {
            await database.insert(user.table).values({
                ...RECORD_TIMES,
                id: userId,
                name,
                email: `${userId}@example.test`,
                residencyId: "eu",
            });
            const accessCopies = new AccessFixture(database);
            await accessCopies.copyScope(person);
            await accessCopies.copyOwner(person, person);
        }

        return { owner, operator };
    }

    /** Keep the owner's accounts, returning the owner role of each account by identifier. */
    static async #keepAccounts(
        database: DatabaseConnection,
        owner: Subject,
    ): Promise<Map<string, string>> {
        const ownerRoles = new Map<string, string>();
        for (const [id, handle] of [
            [ids.account, "acme"],
            [ids.other, "rival"],
            [ids.platform, "destack"],
        ] as const) {
            // keep the account in the owner's scope
            await database.insert(account.table).values({
                ...RECORD_TIMES,
                id,
                handle,
                name: handle,
                residencyId: "eu",
                kind: id === ids.account ? ("personal" as const) : ("shared" as const),
                scope: ids.owner,
            });

            // make the owner the owner of the account
            const scope = account.reference(ids.owner, id);
            await new AccessFixture(database).copyScope(scope);
            ownerRoles.set(id, await new AccessFixture(database).copyOwner(scope, owner));

            // let the account's hosts read it
            const hosts = principal.host.reference(id, "*");
            await AccountFixture.relate(database, id, scope, "host", hosts);

            // let the workloads of the account's residency serve it
            await AccountFixture.relate(database, id, scope, "resident", Account.resident("eu"));
        }

        return ownerRoles;
    }

    /** Keep the platform's region, which the operator serves as an owner of the platform's account. */
    static async #keepRegion(
        database: DatabaseConnection,
        operator: Subject,
        platformOwner: string,
    ): Promise<void> {
        // keep the region
        await database.insert(region.table).values({
            ...RECORD_TIMES,
            id: ids.region,
            code: "eu-central",
            name: "Europe",
            residencyId: "eu",
        });

        // make the operator the server of the region
        await new AccessFixture(database).copyRole(
            Scope.universe,
            {
                name: "operator",
                description: "Serve Destack's regions with hosts",
                permissions: [region.policy.permission("serve")],
            },
            operator,
        );

        // make the operator an owner of the platform
        const platform = account.reference(ids.owner, ids.platform);
        await AccountFixture.relate(
            database,
            ids.platform,
            platform,
            { role: platformOwner },
            operator,
        );
    }

    /** Connect accounts at providers, refusing to keep any connection credentials. */
    static #connections(
        providers: readonly accountServer.ConnectionProvider[],
    ): accountServer.ConnectionOptions {
        return {
            providers,
            vault: {
                write: refuseCredentials,
                read: refuseCredentials,
                destroy: refuseCredentials,
            },
        };
    }

    /** Route a request of the account service's origin as the universe does: by mount, else to the account service's issuer paths. */
    fetch(request: Request): Promise<Response> {
        // find the mounted service
        const mount = ServiceMount.route(request);

        // answer sign-in at the issuer's paths
        if (mount === undefined) {
            return this.server.fetch(request);
        }
        // hand the request over below the account mount
        else if (mount.packageId === accountService.package.id) {
            return this.server.fetch(mount.request);
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

    /** Record a second factor as the last ceremony of a user's sessions, as a step-up sign-in does. */
    async elevate(userId: Identifier<"user">): Promise<void> {
        await this.database
            .update(session.table)
            .set({ authenticationMethod: "totp", authenticatedAt: Date.now() })
            .where(eq(session.table.userId, userId));
    }

    /** Sign a universe token for a user just authenticated at the highest assurance, for a service in a space. */
    async token(userId: Identifier<"user">, audience: PackageId, spaceId: string): Promise<string> {
        // describe the user calling the service in the space, just authenticated
        const now = Date.now();
        const subject = principal.user.reference(Scope.universe.id, userId);
        const caller = new Authentication({
            credential: { kind: "session", id: Identifier.create("session") },
            audience,
            scope: spaceId,
            subject,
            subjects: [subject],
            assurance: { level: 2, authenticatedAt: now },
            verifiedAt: now,
            expiresAt: now + AUTHENTICATION_LIFETIME_MILLISECONDS,
        });

        return (await this.#tokens.issue(caller)).accessToken;
    }

    /** Connect to the account service as a user. */
    user(userId: string) {
        return connect({
            url: ACCOUNTS_URL,
            headers: { "x-user": userId },
            fetch: (request) => this.server.fetch(request),
        });
    }

    /** Connect to the account service as a host. */
    host(identity: HostIdentity) {
        return connect({
            url: ACCOUNTS_URL,
            fetch: identity.fetch(
                (request) => this.server.fetch(request),
                accountService.package.id,
                ACCOUNTS_URL,
            ),
        });
    }

    /** Run as a placed workload, through a host of its region exchanging its token for the workload's. */
    identity(host: HostIdentity, placementId: Identifier<"placement">): WorkloadIdentity {
        return new WorkloadIdentity(
            placementId,
            connect({
                url: ACCOUNTS_URL,
                fetch: host.fetch(
                    (request) => this.server.fetch(request),
                    accountService.package.id,
                    ACCOUNTS_URL,
                    { placementId },
                ),
            }),
        );
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

    /** Place a package's workload in the platform's region with permissions through a universe role. */
    async place(
        packageId: PackageId,
        permissions: readonly PermissionReference[] = [],
    ): Promise<Identifier<"placement">> {
        // place the workload
        const [row] = await this.objects.executeAsSystem(
            placement,
            "create",
            [{ scope: Scope.universe.id, input: { packageId, regionId: ids.region } }],
            Date.now(),
        );
        const { id } = present(row, "the placement");

        // bind it a universe role with the permissions
        if (permissions.length > 0) {
            await new AccessFixture(this.database).copyRole(
                Scope.universe,
                {
                    name: `workload-${id.slice(-12)}`,
                    description: "Serves a workload",
                    permissions: [...permissions],
                },
                principal.workload.reference(Scope.universe.id, id),
            );
        }

        return id;
    }

    /** Wait until a placed workload's copies catch up with the account service. */
    async settle(objects: ObjectServer, placementId: string): Promise<void> {
        // wait for the copies to catch up with the account service's latest change they did not originate
        const signal = AbortSignal.timeout(SETTLE_MILLISECONDS);
        const head = await this.database.log.position(await objects.database.log.epoch());
        const done = new Set<string>();
        for (let isGrowing = true; isGrowing;) {
            // wait for every scope the requested copies read, not caught up yet
            const requests = await objects.source.workloadSubscriptions(placementId);
            const read = requests.flatMap((request) =>
                Object.values(objects.source.replicaOf(request).queries).flatMap((query) =>
                    query.scopes === "every" ? [] : query.scopes,
                ),
            );
            const scopes = [...new Set(read)].filter((scope) => !done.has(scope));
            for (const scope of scopes) {
                if (!(await Replica.reach(objects.database, scope, head, signal))) {
                    throw new Error(`the copy of ${scope} did not reach the account service`);
                }
                done.add(scope);
            }
            isGrowing = scopes.length > 0;
        }
    }

    /** Enroll a new host under an account and return its identity. */
    async enroll(
        accountId: string,
        kind: "device" | "cloud" = "cloud",
        regionId: Identifier<"region"> = ids.region,
    ): Promise<HostIdentity> {
        // enroll a platform host for its region as the operator, any other as the owner
        const identity = new HostIdentity(Identifier.create("host"), new MemoryKeychain());
        const enroller = accountId === ids.platform ? ids.operator : ids.owner;
        await identity.enroll(this.user(enroller), {
            accountId: schema.identifier("account").parse(accountId),
            requestId: RequestId.create(),
            name: `machine-${identity.hostId.slice(-12)}`,
            kind,
            ...(accountId === ids.platform ? { regionId } : {}),
        });

        return identity;
    }

    /** Serve an implementation to spaces and hosts by their tokens and to users by session or header. */
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
            authenticate: (request) =>
                AccountFixture.#authenticate(request, verifier, database, audience),
        });
    }

    /** Authenticate a request by a token a space or the universe signed, a session's bearer, or the fixture header. */
    static async #authenticate(
        request: Request,
        verifier: TokenVerifier,
        database: DatabaseConnection,
        audience: PackageId,
    ): Promise<Authentication | null> {
        // verify a token a space signed by the space's identity in the directory
        if (SpaceToken.accepts(request)) {
            return SpaceToken.verify(request, {
                directory: new DirectoryStore(database),
                audience,
            });
        }

        // verify a token the universe signed
        const authorization = request.headers.get("authorization") ?? "";
        if (/^Bearer \S+\.\S+\.\S+$/u.test(authorization)) {
            const authentication = await verifier.authenticate(request);
            await HostKey.requireAuthenticating(database, authentication, Date.now());

            return authentication;
        }

        // take the user of a session's bearer, refusing any other bearer
        const bearer = /^Bearer (session-\S+)$/u.exec(authorization);
        if (bearer !== null) {
            return AccountFixture.#sessionAuthentication(database, audience, bearer[1]);
        }

        // take the user from the fixture header
        const userId = request.headers.get("x-user");
        if (userId === null) {
            return null;
        }
        const subject = principal.user.reference(Scope.universe.id, userId);
        const now = Date.now();

        return new Authentication({
            credential: { kind: "fixture", id: userId },
            audience,
            subject,
            subjects: [subject],
            verifiedAt: now,
            expiresAt: now + FIXTURE_AUTHENTICATION_MILLISECONDS,
        });
    }

    /** Authenticate the user of an unexpired session at its last ceremony's assurance. */
    static async #sessionAuthentication(
        database: DatabaseConnection,
        audience: PackageId,
        token: string | undefined,
    ): Promise<AccountAuthentication> {
        // read the unexpired session
        const sessionId = schema.identifier("session").safeParse(token);
        const now = Date.now();
        const [active] = sessionId.success
            ? await database
                  .select({
                      userId: session.table.userId,
                      authenticatedAt: session.table.authenticatedAt,
                      authenticationMethod: session.table.authenticationMethod,
                  })
                  .from(session.table)
                  .where(
                      and(eq(session.table.id, sessionId.data), gt(session.table.expiresAt, now)),
                  )
            : [];
        if (!sessionId.success || active === undefined) {
            throw new ServiceError("UNAUTHORIZED", { message: "session is unknown or expired" });
        }

        // authenticate its user at the session's assurance
        const subject = principal.user.reference(Scope.universe.id, active.userId);
        const assurance = ActiveSession.rate(active);

        return new AccountAuthentication({
            credential: { kind: "session", id: sessionId.data, userId: active.userId },
            audience,
            subject,
            subjects: [subject],
            ...(assurance === undefined ? {} : { assurance }),
            verifiedAt: now,
            expiresAt: now + FIXTURE_AUTHENTICATION_MILLISECONDS,
        });
    }

    /** Stop the account service, then close the database. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.server.close();
        await this.storage.close();
    }
}

/** Refuse to keep, read or destroy a connection's credentials, which the fixture keeps none of. */
async function refuseCredentials(): Promise<never> {
    throw new TypeError("the fixture keeps no connection credentials");
}
