import { Package } from "@destack/package";
import { Authentication, Caller, principal } from "@destack/access";
import { AccessFixture } from "@destack/access/test";
import * as accountObject from "@destack/account/object";
import { AccountFixture, ids } from "@destack/account/test";
import { implementConnection, implementIntegration, TokenSource } from "@destack/connection/server";
import { authorizationRequest } from "@destack/connection/object";
import connectionPackage from "@destack/connection/package";
import { connectionDatabase } from "@destack/connection/test";
import type { DatabaseConnection } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { implementPackage, ObjectStore, Server, Health } from "@destack/object/server";
import { testEvents, testCallKey } from "@destack/object/test";
import { ResourceContext } from "@destack/resource/context";
import { Identifier, present } from "@destack/schema";
import { SPACE_KEY } from "@destack/access/token";
import { createClient } from "@destack/object/client";
import { PackageMount, SpaceMount } from "@destack/object";
import { type ObjectReference, Scope, type Subject } from "@destack/sync";
import vaultServer from "@destack/vault/server";
import { vault } from "@destack/vault/object";
import { Clock } from "@destack/environment";

/** The header with the user a fixture request comes from. */
const USER_HEADER = "x-user";

/** The residency the fixture's accounts store their data in. */
const RESIDENCY = "eu";

/** An account's home space storing its connections, with the account's chain replicated in. */
export class ConnectionSpace implements AsyncDisposable {
    /** The account the space is the home of. */
    readonly accountId: Identifier<"account">;
    /** The space. */
    readonly id: Identifier<"space">;
    /** The space's connection service. */
    readonly server: Server;
    /** The vault reference connections store their credentials in. */
    readonly vault: ObjectReference;
    /** The space's connection installation, which calls the forge's issuer. */
    readonly consumer: Subject;
    /** The space's database and server, closed with the space. */
    readonly #disposal: AsyncDisposableStack;

    /** Take a served home space. */
    private constructor(
        accountId: Identifier<"account">,
        id: Identifier<"space">,
        server: Server,
        credentials: ObjectReference,
        consumer: Subject,
        disposal: AsyncDisposableStack,
    ) {
        this.accountId = accountId;
        this.id = id;
        this.server = server;
        this.vault = credentials;
        this.consumer = consumer;
        this.#disposal = disposal;
    }

    /** Serve an account's home space with its owner and its residency's installation. */
    static async open(
        account: { readonly id: Identifier<"account">; readonly owner: Identifier<"user"> },
        id: Identifier<"space">,
        installation: Subject,
        issuer: {
            readonly url: string;
            readonly issue: (request: Request, from: Subject) => Promise<Response>;
        },
    ): Promise<ConnectionSpace> {
        // store the space's database with the account's chain, its owner and the installation serving its residency
        const disposal = new AsyncDisposableStack();
        const storage = await TestDatabase.create("sqlite", connectionDatabase, {
            isMigrated: true,
        });
        disposal.defer(() => storage.close());
        const { database } = storage;
        await ConnectionSpace.#keepChain(database, account, id, installation);

        // serve the space's vaults and connections, issuing no installation tokens itself
        const consumer = principal.installation.reference(id, Identifier.create("installation"));
        const tokens = new TokenSource({
            consumer,
            fetch: async () => {
                throw new TypeError("the space calls no authorization server");
            },
            // reach the forge's issuer of installation tokens as the space's connections
            at: () => ({ url: issuer.url, fetch: (request) => issuer.issue(request, consumer) }),
        });
        const connections = implementPackage(connectionPackage, {
            objects: [
                implementConnection(tokens),
                implementIntegration(tokens),
                authorizationRequest,
            ],
        });
        const objects = new ObjectStore({
            servers: [vaultServer, connections],
            chain: async () => undefined,
            clock: Clock.system,
            principal: principal.space.reference(Scope.universe.id, id),
            database,
            callKey: testCallKey,
            events: await testEvents(database),
            origin: { package: Package.of(connectionPackage) },
            policies: [accountObject.account.policy, accountObject.residency.policy],
            machine: ids.machine,
        });
        const server = Server.start({
            ...objects.package(connectionPackage),
            clock: Clock.system,
            resources: new ResourceContext(),
            health: new Health("connection"),
            drainTimeout: 1000,
            authorizeMachine: async () => {},
            authenticate: async (request) => ConnectionSpace.#authenticate(request, installation),
        });
        disposal.defer(() => server.close());

        // store a vault in the account for the connections' credentials
        const now = Date.now();
        const vaultId = vault.generateId();
        await database.insert(vault.table).values({
            id: vaultId,
            scope: account.id,
            name: "credentials",
            definitionPackageId: vault.package.id,
            definitionVersion: vault.package.version,
            definitionName: "credentials",
            spec: {},
            createdAt: now,
            updatedAt: now,
        });

        return new ConnectionSpace(
            account.id,
            id,
            server,
            vault.reference(account.id, vaultId),
            consumer,
            disposal,
        );
    }

    /** Close the space's server and database. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.#disposal.disposeAsync();
    }

    /** Replicate the account's chain, owner, home and residency installation into a space. */
    static async #keepChain(
        database: DatabaseConnection,
        account: { readonly id: Identifier<"account">; readonly owner: Identifier<"user"> },
        id: Identifier<"space">,
        installation: Subject,
    ): Promise<void> {
        // replicate the owner's and the account's scopes, the account owned by the owner and homed in the space
        const access = new AccessFixture(database);
        const owner = principal.user.reference(Scope.universe.id, account.owner);
        const scope = accountObject.account.reference(account.owner, account.id);
        await access.replicateScope(owner);
        await access.replicateScope(scope);
        await access.bindOwner(scope, owner);
        const home = principal.space.reference(Scope.universe.id, id);
        await AccountFixture.relate(database, account.id, scope, "home", home);

        // relate the residency's servers and installations to the account, and the installation to the residency
        const residency = accountObject.residency.reference(Scope.universe.id, RESIDENCY);
        const servers = accountObject.Residency.servers(RESIDENCY);
        const claimants = accountObject.Residency.claimants(RESIDENCY);
        await AccountFixture.relate(database, account.id, scope, "resident", servers);
        await AccountFixture.relate(database, account.id, scope, "claimant", claimants);
        await AccountFixture.relate(
            database,
            Scope.universe.id,
            residency,
            "installation",
            installation,
        );
    }

    /** Connect to the space's connection service as a user. */
    connect(userId: string) {
        return createClient(connectionPackage, {
            url: "https://space.test",
            headers: { [USER_HEADER]: userId },
            fetch: (request) => this.server.fetch(request),
        });
    }

    /** Route an installation's request to the connection service of its path's space. */
    static fetch(spaces: readonly ConnectionSpace[]): (request: Request) => Promise<Response> {
        return async (request) => {
            // find the space and strip its prefix and the service's mount
            const url = new URL(request.url);
            const { spaceId: id, path } = present(SpaceMount.parse(url.pathname), "a space path");
            const space = present(
                spaces.find((each) => each.id === id),
                `the space ${id}`,
            );
            const mount = PackageMount.path(connectionPackage.id);
            url.pathname = path.slice(mount.length) || "/";

            return space.server.fetch(new Request(url, request));
        };
    }

    /** Authenticate the user of a fixture request, else the installation the space serves. */
    static #authenticate(request: Request, installation: Subject): Authentication {
        const userId = request.headers.get(USER_HEADER);
        const subject =
            userId !== null ? principal.user.reference(Scope.universe.id, userId) : installation;
        const now = Date.now();

        return {
            caller: Caller.of(subject),
            credential: {
                kind: userId === null ? SPACE_KEY : "session",
                id: subject.id,
                audience: connectionPackage.id,
                verifiedAt: now,
                expiresAt: now + 60_000,
            },
        };
    }
}
