import {
    accessRelationship,
    accessRole,
    accessRolePermission,
    type Caller,
    principal,
    Relationship,
} from "@destack/access";
import { AccessFixture } from "@destack/access/test";
import { account, space } from "@destack/account/object";
import { journal } from "@destack/audit/stack";
import {
    and,
    type DatabaseConnection,
    defineDatabase,
    type Dialect,
    eq,
    inArray,
} from "@destack/db";
import * as sqlite from "@destack/db/bun";
import { TestDatabase } from "@destack/db/test";
import { type Keyring, LocalKeyring } from "@destack/identity";
import { ObjectStore } from "@destack/object/server";
import type { PackageId } from "@destack/package";
import { ResourceContext } from "@destack/resource/context";
import { type Identifier, schema } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import { Health } from "@destack/service/health";
import { RequestId } from "@destack/service/request";
import { Server } from "@destack/service/server";
import { testCallKey } from "@destack/service/test";
import { binding, capture, deployment, installation } from "@destack/stack/object";
import { Scope, type Subject } from "@destack/sync";
import { TestClock } from "@destack/test";
import { v7 } from "uuid";
import { VaultClient } from "../connector/index.ts";
import { VaultKeystore } from "../key/index.ts";
import { SECRET_RECOVERY, secret, secretVersion, vault } from "../object/index.ts";
import { serveKeys, serveSecrets, serveVaults } from "../server/secret.ts";
import { CREDENTIALS_VAULT } from "../server/server.ts";
import { vaultService } from "../service/index.ts";
import { vaultTables } from "../stack/index.ts";
import { TestVault } from "./vault.ts";

/** A machine's database keeping vaults beside its journal, copying the spaces they live in. */
export const vaultDatabase = defineDatabase({
    name: "vault",
    tables: [...vaultTables, journal],
    copies: [space.table, installation.table, binding.table, capture.table, deployment.table],
});

/** The bytes of a root key, an AES-256 key. */
const ROOT_KEY_BYTES = 32;

/** How long the member's fixture credential lasts: one minute. */
const CREDENTIAL_LIFETIME_MILLISECONDS = 60_000;

/** How long a server drains on close: one second. */
const DRAIN_MILLISECONDS = 1000;

/** The object types whose plaintext reads the role grants. */
const READ_TYPES = ["secret", "version"];

/** The vault package: it declares the vaults and serves their secrets. */
export const VAULT = vault.package;

/** The machine the fixture's vault keys belong to. */
export const MACHINE = schema
    .identifier("machine")
    .parse("machine-01996ab0-0000-7000-8000-0000000000e1");

/** Another machine a space moves to. */
export const OTHER_MACHINE = schema
    .identifier("machine")
    .parse("machine-01996ab0-0000-7000-8000-0000000000e2");

/** A migrated vault service database, a provisioned vault, a member role and a client. */
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
    /** The first root key, which outlives key rotations. */
    readonly root: Uint8Array<ArrayBuffer>;
    /** The keyring of the first root key. */
    readonly keyring: Keyring;
    /** The migrated vault service database. */
    readonly database: DatabaseConnection;
    /** The clock the fixture's stores and servers read, which tests set. */
    readonly clock = new TestClock();
    /** The member's vault client, replaced by scenarios connecting to another server. */
    client: VaultClient;
    /** The member the servers authenticate. */
    readonly #caller: Caller;
    /** The served servers, closed before the database. */
    readonly #servers: Server[] = [];
    /** Release the database. */
    readonly #close: () => Promise<void>;

    /** Keep the database, with the identities of a previous fixture over it or new ones. */
    private constructor(
        database: DatabaseConnection,
        close: () => Promise<void>,
        root: { readonly key: Uint8Array<ArrayBuffer>; readonly keyring: Keyring },
        previous?: VaultFixture,
    ) {
        // reuse the previous fixture's identities
        this.database = database;
        this.#close = close;
        this.root = root.key;
        this.keyring = root.keyring;
        this.spaceId = previous?.spaceId ?? schema.identifier("space").parse(`space-${v7()}`);
        this.vaultId = previous?.vaultId ?? schema.identifier("vault").parse(`vault-${v7()}`);
        this.userId = previous?.userId ?? schema.identifier("user").parse(`user-${v7()}`);
        this.accountId =
            previous?.accountId ?? schema.identifier("account").parse(`account-${v7()}`);
        this.roleId = previous?.roleId ?? schema.identifier("role").parse(`role-${v7()}`);

        // serve the vault under the root key to the member
        this.#caller = this.#member();
        this.client = this.connect(this.serve(this.keyring));
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

    /** Open the vault keys under a keyring, the fixture's first root key by default. */
    keystore(keyring: Keyring = this.keyring): VaultKeystore {
        return new VaultKeystore(this.database, keyring, MACHINE);
    }

    /** Serve the vault's objects to the space, as a machine's controllers call them. */
    objects(
        keyring: Keyring = this.keyring,
        vaults: ReturnType<typeof serveVaults> = serveVaults(this.keystore(keyring)),
    ): ObjectStore {
        const keystore = this.keystore(keyring);

        return new ObjectStore({
            principal: this.spacePrincipal,
            objects: {
                vault: vaults,
                vaultKey: serveKeys(keystore),
                ...serveSecrets(keystore, SECRET_RECOVERY),
            },
            policies: [space],
            database: this.database,
            callKey: testCallKey,
            origin: { package: VAULT, service: vaultService.name },
            clock: this.clock,
        });
    }

    /** Serve the vault under a keyring with bearer authentication of the member. */
    serve(keyring: Keyring): Server {
        // serve the objects to the member
        const objects = this.objects(keyring);
        const server = Server.start({
            ...objects.implement(vaultService, objects.routed, objects.controllers(), {}),
            audience: VAULT.id,
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
    connect(server: Server): VaultClient {
        return new VaultClient(vaultService, {
            url: "http://vault.test",
            headers: { Authorization: `Bearer ${this.userId}` },
            fetch: (request) => server.fetch(request),
        });
    }

    /** Authenticate a request with the member's identifier as the member. */
    #authenticate(request: Request): Caller {
        if (request.headers.get("Authorization") !== `Bearer ${this.userId}`) {
            throw new ServiceError("UNAUTHORIZED", { message: "invalid bearer credential" });
        }

        return this.#caller;
    }

    /** Describe the member of the space's account. */
    #member(): Caller {
        const now = Date.now();

        return {
            subject: principal.user.reference("universe", this.userId),
            subjects: [
                principal.user.reference("universe", this.userId),
                {
                    ...account.reference(this.userId, this.accountId),
                    relation: "member",
                },
            ],
            credential: {
                kind: "fixture",
                id: "fixture-1",
                audience: VAULT.id,
                verifiedAt: now,
                expiresAt: now + CREDENTIAL_LIFETIME_MILLISECONDS,
            },
        };
    }

    /** Grant the role's plaintext reads of secrets and versions. */
    async grantRead(): Promise<void> {
        await this.#grant(READ_TYPES.map((type) => ({ type, name: "read" })));
    }

    /** Withdraw the role's plaintext reads of secrets and versions. */
    async withdrawRead(): Promise<void> {
        await this.database
            .delete(accessRolePermission)
            .where(
                and(
                    eq(accessRolePermission.roleId, this.roleId),
                    eq(accessRolePermission.name, "read"),
                    inArray(accessRolePermission.type, READ_TYPES),
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
        // import the previous fixture's root key or a new one
        const key = previous?.root ?? crypto.getRandomValues(new Uint8Array(ROOT_KEY_BYTES));
        const keyring = await LocalKeyring.import(1, new Map([[1, key]]));
        const fixture = new VaultFixture(database, close, { key, keyring }, previous);

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

    /** Provision the space, its vault with its first key, and the member role. */
    async #provision(): Promise<void> {
        // register the space and its vault with its first key
        const now = Date.now();
        await this.#register(now);
        await this.registerVault(this.vaultId, CREDENTIALS_VAULT, now);

        // bind the role and own the space as itself, as the space's creation does
        await this.#bindRole(now);
        await new AccessFixture(this.database).copyOwner(
            space.reference(this.accountId, this.spaceId),
            this.spacePrincipal,
            now,
        );

        // grant the role every vault permission
        await this.#grant([
            ...[vault, secret, secretVersion].flatMap((object) =>
                object.permissions.map((name) => ({ type: object.name, name })),
            ),
            { type: space.name, name: "read", packageId: space.policy.definition.packageId },
        ]);
    }

    /** Register a vault of the space with its first key, as the vault's creation does. */
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

        // start its key at its first version
        await TestVault.startKey(this.database, this.keystore(), { id, scope: this.spaceId }, now);
    }

    /** Register the space. */
    async #register(now: number): Promise<void> {
        await this.database.insert(space.table).values({
            id: this.spaceId,
            scope: this.accountId,
            name: "vault-fixture",
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
                    id: `relationship-${v7()}`,
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
                id: schema.identifier("role-permission").parse(`role-permission-${v7()}`),
                roleId: this.roleId,
                scope: this.spaceId,
                packageId: packageId ?? VAULT.id,
                type,
                name,
            })),
        );
    }
}
