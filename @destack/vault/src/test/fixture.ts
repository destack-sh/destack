import { STACKS } from "@destack/account/test";
import {
    accessRelationship,
    accessRole,
    accessRolePermission,
    type Authentication,
    Caller,
    principal,
    Relationship,
} from "@destack/access";
import { AccessFixture } from "@destack/access/test";
import { account, space } from "@destack/account/object";
import { sudo } from "@destack/account/authentication";
import { and, type DatabaseConnection, defineDatabase, type Dialect, eq } from "@destack/db";
import * as sqlite from "@destack/db/bun";
import { TEST_CIPHER, TestDatabase } from "@destack/db/test";
import { defineServer, ObjectStore } from "@destack/object/server";
import { callTables, testEvents } from "@destack/object/test";
import type { EventStore } from "@destack/event";
import type { PackageId } from "@destack/package";
import { ResourceContext } from "@destack/resource/context";
import { Identifier, schema } from "@destack/schema";
import { createClient, type ClientOf } from "@destack/runtime/client";
import { CallError } from "@destack/runtime/error";
import { Health } from "@destack/runtime/health";
import { RequestId } from "@destack/runtime/request";
import { Server } from "@destack/runtime/server";
import { testCallKey } from "@destack/runtime/test";
import { binding, capture, deployment, installation } from "@destack/stack/object";
import { Scope, type Subject } from "@destack/sync";
import { TestClock } from "@destack/test";
import { secret, secretVersion, vault } from "../object/index.ts";
import { implementSecret, implementSecretVersion } from "../server/secret.ts";
import { implementVault } from "../server/vault.ts";
import vaultPackage from "../package.ts";
import { vaultTables } from "../stack/index.ts";

/** A machine's database keeping vaults beside its calls, replicating the spaces they live in. */
export const vaultDatabase = defineDatabase({
    name: "vault",
    tables: [...vaultTables, ...callTables],
    replicated: [space.table, installation.table, binding.table, capture.table, deployment.table],
});

/** How long the member's fixture credential lasts: one minute. */
const CREDENTIAL_LIFETIME_MILLISECONDS = 60_000;

/** How long a server drains on close: one second. */
const DRAIN_MILLISECONDS = 1000;

/** The name of the fixture's vault. */
const VAULT_NAME = "credentials";

/** The vault package: it declares the vaults and serves their secrets. */
export const VAULT = vault.package;

/** The machine a fixture secret is bound to as its consumer. */
export const MACHINE = schema
    .identifier("machine")
    .parse("machine-01996ab0-0000-7000-8000-0000000000e1");

/** A machine no fixture secret is bound to. */
export const OTHER_MACHINE = schema
    .identifier("machine")
    .parse("machine-01996ab0-0000-7000-8000-0000000000e2");

/** A migrated vault database, a provisioned vault, a member role and a client. */
export class VaultFixture implements AsyncDisposable {
    /** The space with the vault. */
    readonly spaceId: Identifier<"space">;
    /** The provisioned vault. */
    readonly vaultId: Identifier<"vault">;
    /** The member calling the vault. */
    readonly userId: Identifier<"user">;
    /** The account of the space with members for the role to bind. */
    readonly accountId: Identifier<"account">;
    /** The role granting every vault permission in the space. */
    readonly roleId: Identifier<"role">;
    /** The migrated vault database. */
    readonly database: DatabaseConnection;
    /** The clock the fixture's stores and servers read, which tests set. */
    readonly clock = new TestClock();
    /** The member's vault client, replaced by scenarios connecting to another server. */
    client: ClientOf<typeof vaultPackage>;
    /** The member the servers authenticate. */
    #authentication: Authentication;
    /** The event log keeping the stores' calls. */
    readonly #events: EventStore;
    /** The served servers, closed before the database. */
    readonly #servers: Server[] = [];
    /** Release the database. */
    readonly #close: () => Promise<void>;

    /** Keep the database, with the identities of a previous fixture over it or new ones. */
    private constructor(
        database: DatabaseConnection,
        close: () => Promise<void>,
        events: EventStore,
        previous?: VaultFixture,
    ) {
        // reuse the previous fixture's identities
        this.database = database;
        this.#close = close;
        this.#events = events;
        this.spaceId = previous?.spaceId ?? Identifier.create("space");
        this.vaultId = previous?.vaultId ?? Identifier.create("vault");
        this.userId = previous?.userId ?? Identifier.create("user");
        this.accountId = previous?.accountId ?? Identifier.create("account");
        this.roleId = previous?.roleId ?? Identifier.create("role");

        // serve the vault to the member
        this.#authentication = this.#member();
        this.client = this.connect(this.serve());
    }

    /** The space's principal, which its servers' own work runs as. */
    get spacePrincipal(): Subject {
        return principal.space.reference(Scope.universe.id, this.spaceId);
    }

    /** Close the servers, then the database. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.close();
    }

    /** Drain the servers' requests before releasing the database. */
    async close(): Promise<void> {
        for (const server of this.#servers) {
            await server.close();
        }
        await this.#close();
    }

    /** Serve the vault's objects to the space, as a machine's controllers call them. */
    objects(vaults: ReturnType<typeof implementVault> = implementVault()): ObjectStore {
        return new ObjectStore({
            chain: async () => undefined,
            principal: this.spacePrincipal,
            servers: [
                defineServer(vaultPackage, {
                    objects: [vaults, implementSecret(), implementSecretVersion()],
                }),
            ],
            policies: [space],
            database: this.database,
            callKey: testCallKey,
            events: this.#events,
            origin: { package: VAULT },
            clock: this.clock,
        });
    }

    /** Serve the vault with bearer authentication of the member. */
    serve(): Server {
        // serve the objects to the member
        const objects = this.objects();
        const server = Server.start({
            ...objects.package(vaultPackage),
            resources: new ResourceContext(),
            health: new Health("vault"),
            authenticate: async (request) => this.#authenticate(request),
            authorizeMachine: async () => {},
            drainTimeout: DRAIN_MILLISECONDS,
            clock: this.clock,
        });
        this.#servers.push(server);

        return server;
    }

    /** Connect to a served vault as the fixture's member. */
    connect(server: Server): ClientOf<typeof vaultPackage> {
        return createClient(vaultPackage, {
            url: "http://vault.test",
            headers: { Authorization: `Bearer ${this.userId}` },
            fetch: (request) => server.fetch(request),
        });
    }

    /** Authenticate a request with the member's identifier as the member. */
    #authenticate(request: Request): Authentication {
        if (request.headers.get("Authorization") !== `Bearer ${this.userId}`) {
            throw new CallError("UNAUTHORIZED", { message: "invalid bearer credential" });
        }

        return this.#authentication;
    }

    /** Describe the member of the space's account. */
    #member(): Authentication {
        // sign the member in with a fixture credential, inside the account
        const now = Date.now();
        const subject = principal.user.reference("universe", this.userId);
        const membership = {
            ...account.reference(this.userId, this.accountId),
            relation: "member",
        };

        return {
            caller: { ...Caller.of(subject), groups: [membership] },
            credential: {
                kind: "fixture",
                id: "fixture-1",
                audience: VAULT.id,
                verifiedAt: now,
                expiresAt: now + CREDENTIAL_LIFETIME_MILLISECONDS,
            },
        };
    }

    /** Authenticate the member again with a second factor just now, as a step-up does. */
    stepUp(): void {
        const now = Date.now();
        const { caller, credential } = this.#authentication;
        this.#authentication = {
            caller,
            credential: {
                ...credential,
                assurance: { level: sudo.assurance, authenticatedAt: now },
            },
        };
    }

    /** Grant the role the reveal of secrets. */
    async grantReveal(): Promise<void> {
        await this.#grant([{ type: secret.name, name: "reveal" }]);
    }

    /** Withdraw the role's reveal of secrets. */
    async withdrawReveal(): Promise<void> {
        await this.database
            .delete(accessRolePermission)
            .where(
                and(
                    eq(accessRolePermission.roleId, this.roleId),
                    eq(accessRolePermission.type, secret.name),
                    eq(accessRolePermission.name, "reveal"),
                ),
            );
    }

    /** Create a secret with a first version of a text value. */
    async createSecret() {
        // create the secret with its first version
        const first = await this.client.secret.create({
            scope: this.spaceId,
            parentId: this.vaultId,
            name: "first",
            requestId: RequestId.create(Date.now()),
        });
        const request = {
            scope: this.spaceId,
            parentId: first.id,
            requestId: RequestId.create(Date.now()),
            value: { encoding: "text" as const, value: "credential" },
        };
        const written = await this.client.version.create(request);

        return { first, key: { scope: this.spaceId, id: first.id }, request, written };
    }

    /** Claim an isolated migrated database of a dialect with vault contents. */
    static async open(dialect: Dialect): Promise<VaultFixture> {
        // claim the database and the contents' directory together
        const test = await TestDatabase.create(dialect, vaultDatabase, { isMigrated: true });

        return VaultFixture.#start(test.database, () => test.close());
    }

    /** Open a SQLite file, migrate it, and serve the vault. */
    static async openFile(file: string, previous?: VaultFixture): Promise<VaultFixture> {
        // migrate the file
        const connection = await sqlite.connectBunSqlite(file, vaultDatabase);
        connection.key(TEST_CIPHER);
        await connection.migrate(vaultDatabase.tables).catch(async (error: unknown) => {
            await connection.close();
            throw error;
        });

        return VaultFixture.#start(connection, () => connection.close(), previous);
    }

    /** Provision the space, its vault and a member role in a migrated database. */
    static async #start(
        database: DatabaseConnection,
        close: () => Promise<void>,
        previous?: VaultFixture,
    ): Promise<VaultFixture> {
        // keep the events beside the database
        const events = await testEvents(database);
        const fixture = new VaultFixture(database, close, events, previous);

        // provision the space unless the previous fixture did
        try {
            if (previous === undefined) {
                await fixture.#provision();
            }

            return fixture;
        } catch (error) {
            await fixture.close();
            throw error;
        }
    }

    /** Provision the space, its vault and the member role. */
    async #provision(): Promise<void> {
        // register the space and its vault
        const now = Date.now();
        await this.#register(now);
        await this.registerVault(this.vaultId, VAULT_NAME, now);

        // bind the role and own the space as itself, as the space's creation does
        await this.#bindRole(now);
        await new AccessFixture(this.database).bindOwner(
            space.reference(this.accountId, this.spaceId),
            this.spacePrincipal,
            now,
        );

        // grant the role every vault permission
        await this.#grant([
            ...[vault, secret, secretVersion].flatMap((object) =>
                object.permissions
                    .filter((name) => !(object.policy.definition.reserved ?? []).includes(name))
                    .map((name) => ({ type: object.name, name })),
            ),
            { type: space.name, name: "read", packageId: space.policy.definition.packageId },
        ]);
    }

    /** Register a vault of the space, as the vault's creation does. */
    async registerVault(
        id: Identifier<"vault">,
        name: string,
        now: number = Date.now(),
    ): Promise<void> {
        // register the vault
        await this.database.insert(vault.table).values({
            id,
            scope: this.spaceId,
            name,
            definitionPackageId: VAULT.id,
            definitionVersion: VAULT.version,
            definitionName: name,
            spec: {},
            createdAt: now,
            updatedAt: now,
        });
    }

    /** Register the space. */
    async #register(now: number): Promise<void> {
        await this.database.insert(space.table).values({
            id: this.spaceId,
            scope: this.accountId,
            name: "vault-fixture",
            stack: STACKS.space,
            createdAt: now,
            updatedAt: now,
        });
    }

    /** Bind the member role to the account's members on the space. */
    async #bindRole(now: number): Promise<void> {
        // create the role
        await this.database.insert(accessRole).values({
            id: this.roleId,
            scope: this.spaceId,
            name: "vault-owner",
            description: "manage the fixture vault",
            createdAt: now,
            updatedAt: now,
        });

        // record the space under the account
        await this.database.insert(Scope.table).values({
            scope: this.spaceId,
            parent: this.accountId,
            partition: this.spaceId,
            packageId: space.policy.definition.packageId,
            type: space.name,
            ancestors: [this.accountId],
        });

        // bind the role to the account's members on the space
        await this.database.insert(accessRelationship).values(
            Relationship.encode(
                {
                    id: Identifier.create("relationship"),
                    object: space.reference(this.accountId, this.spaceId),
                    role: this.roleId,
                    subject: {
                        ...account.reference(this.userId, this.accountId),
                        relation: "member",
                    },
                    createdAt: now,
                    expiresAt: null,
                },
                this.spaceId,
            ),
        );
    }

    /** Grant the role permissions in the space in one statement. */
    async #grant(
        permissions: readonly { type: string; name: string; packageId?: PackageId }[],
    ): Promise<void> {
        await this.database.insert(accessRolePermission).values(
            permissions.map(({ type, name, packageId }) => ({
                id: Identifier.create("role-permission"),
                roleId: this.roleId,
                scope: this.spaceId,
                packageId: packageId ?? VAULT.id,
                type,
                name,
            })),
        );
    }
}
