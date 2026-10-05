import {
    accessRelationship,
    accessRole,
    accessRolePermission,
    principal,
    Relationship,
} from "@destack/access";
import { and, eq, inArray, type DatabaseConnection, type Dialect } from "@destack/db";
import { Scope } from "@destack/sync";
import * as accountObject from "@destack/account/object";
import { TestDatabase } from "@destack/db/test";
import { DirectoryStore, directoryTables } from "@destack/directory";
import * as sqlite from "@destack/db/bun";
import type { PackageId } from "@destack/package";
import { ResourceContext } from "@destack/resource/context";
import { schema, type Identifier } from "@destack/schema";
import { Authentication } from "@destack/service/authentication";
import { ServiceError } from "@destack/service/error";
import { Health } from "@destack/service/health";
import { RequestId } from "@destack/service/request";
import { Server } from "@destack/service/server";

import { ObjectServer } from "@destack/object/server";
import * as spaceObject from "@destack/space/object";
import { space } from "@destack/space/object";
import { v7 } from "uuid";
import { type Keyring, LocalKeyring } from "@destack/host/keychain";
import { VaultKey } from "../encryption/index.ts";
import { secret, secretVersion, vault } from "../object/index.ts";
import { SecretClient } from "../object/index.ts";
import { spaceService } from "@destack/space/service";
import * as served from "../server/secret.ts";
import { spaceDatabase } from "@destack/space/stack";

import { testCallKey } from "@destack/service/test";

/** The vault package, declaring the vaults and their permissions. */
export const VAULT = vault.package;

/** The space package serving vaults' secrets, the audience of their callers. */
export const SPACE = spaceService.package;

/** The storage location the fixture's values authenticate. */
export const LOCATION = "eu";

/** How long the fixture's host keeps deleted secrets restorable. */
const RECOVERY = { days: 30 };

/** A migrated database, a provisioned vault, a member role on its space and an authenticated client. */
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
    /** The first root key, kept across key rotations. */
    readonly root: Uint8Array<ArrayBuffer>;
    /** The migrated database. */
    readonly database: DatabaseConnection;
    /** The hosted servers, closed before the database. */
    readonly servers: Server[] = [];
    /** The authenticated caller, replaced by scenarios acting as others. */
    caller!: Authentication;
    /** The hosted vault. */
    server!: Server;
    /** The member's vault client. */
    client!: SecretClient;
    /** Release the database. */
    readonly #close: () => Promise<void>;
    /** The directory databases of the cells served, closed with the fixture. */
    readonly #directories: TestDatabase[] = [];

    /** Retain the database, with the identities and root key of a previous fixture over it or new ones. */
    private constructor(
        database: DatabaseConnection,
        close: () => Promise<void>,
        previous?: VaultFixture,
    ) {
        // keep the database, and reuse the previous fixture's identities and root key
        this.database = database;
        this.#close = close;
        this.spaceId = previous?.spaceId ?? schema.identifier("space").parse(`space-${v7()}`);
        this.vaultId = previous?.vaultId ?? schema.identifier("vault").parse(`vault-${v7()}`);
        this.userId = previous?.userId ?? schema.identifier("user").parse(`user-${v7()}`);
        this.accountId =
            previous?.accountId ?? schema.identifier("account").parse(`account-${v7()}`);
        this.roleId = previous?.roleId ?? schema.identifier("role").parse(`role-${v7()}`);
        this.root = previous?.root ?? crypto.getRandomValues(new Uint8Array(32));
    }

    /** Close the servers, then the database. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.close();
    }

    /** Drain the servers' requests before releasing the database. */
    async close(): Promise<void> {
        for (const server of this.servers) {
            await server.close();
        }
        for (const directory of this.#directories) {
            await directory.close();
        }
        await this.#close();
    }

    /** Import the given root keys, the fixture's first one by default. */
    async keyring(
        active = "one",
        keys: ReadonlyMap<string, Uint8Array<ArrayBuffer>> = new Map([["one", this.root]]),
    ): Promise<LocalKeyring> {
        return LocalKeyring.import(active, new Map(keys));
    }

    /** Host the vault under bearer authentication of the fixture's caller. */
    async host(
        keyring?: Keyring,
        authenticate: (request: Request) => Promise<Authentication | null> = async (request) => {
            if (request.headers.get("Authorization") !== `Bearer ${this.userId}`) {
                throw new ServiceError("UNAUTHORIZED", { message: "invalid bearer credential" });
            }

            return this.caller;
        },
        audience: PackageId = SPACE.id,
    ): Promise<Server> {
        // serve the secrets and versions as a cell does
        const objects = new ObjectServer({
            objects: served.serveSecrets(keyring ?? (await this.keyring()), LOCATION, RECOVERY),
            policies: [space, vault],
            database: this.database,
            callKey: testCallKey,
            origin: { package: SPACE, service: spaceService.name },
        });
        const server = Server.start({
            ...objects.implement(spaceService),
            audience,
            resources: new ResourceContext(),
            health: new Health("space"),
            authenticate,
            authorizeHost: async () => {},
            drainTimeout: 1000,
        });
        this.servers.push(server);

        return server;
    }

    /** Serve the vault's objects to the system, as a host's controllers call them. */
    system(keyring: Keyring) {
        return new ObjectServer({
            objects: served.serveSecrets(keyring, LOCATION, RECOVERY),
            policies: [space, vault],
            database: this.database,
            callKey: testCallKey,
            origin: { package: SPACE, service: spaceService.name },
        });
    }

    /** Serve the vault and its secrets to the system with a directory, as a cell does. */
    async cell(keyring: Keyring) {
        const directory = await TestDatabase.create("sqlite", directoryTables, {
            isMigrated: true,
        });
        this.#directories.push(directory);

        return new ObjectServer({
            objects: { vault: served.vault, ...served.serveSecrets(keyring, LOCATION, RECOVERY) },
            policies: [space],
            directory: new DirectoryStore(directory.database),
            database: this.database,
            callKey: testCallKey,
            origin: { package: SPACE, service: spaceService.name },
        });
    }

    /** Connect to a hosted vault as the fixture's member. */
    connect(server: Server): SecretClient {
        return new SecretClient(spaceService, {
            url: "http://vault.test",
            headers: { Authorization: `Bearer ${this.userId}` },
            fetch: (request) => server.fetch(request),
        });
    }

    /** Authenticate the member of the space's account. */
    member(): Authentication {
        return new Authentication({
            credential: { kind: "fixture", id: "fixture-1" },
            audience: SPACE.id,
            verifiedAt: Date.now(),
            expiresAt: Date.now() + 60_000,
            subject: principal.user.reference("universe", this.userId),
            subjects: [
                principal.user.reference("universe", this.userId),
                {
                    ...accountObject.account.reference(this.userId, this.accountId),
                    relation: "member",
                },
            ],
        });
    }

    /** Grant or withdraw the role's plaintext reads of secrets and versions. */
    async allowRead(isAllowed: boolean): Promise<void> {
        // withdraw both reads
        if (!isAllowed) {
            await this.database
                .delete(accessRolePermission)
                .where(
                    and(
                        eq(accessRolePermission.roleId, this.roleId),
                        eq(accessRolePermission.name, "read"),
                        inArray(accessRolePermission.type, ["secret", "version"]),
                    ),
                );
        }
        // grant both reads
        else {
            await this.#grant(["secret", "version"].map((type) => ({ type, name: "read" })));
        }
    }

    /** Create a secret with a first version of a text value. */
    async createSecret() {
        // create the secret, then write its first version
        const first = await this.client.secret.create({
            spaceId: this.spaceId,
            parentId: this.vaultId,
            name: "first",
            requestId: RequestId.create(),
        });
        const request = {
            spaceId: this.spaceId,
            parentId: first.id,
            requestId: RequestId.create(),
            value: { encoding: "text" as const, value: "credential" },
        };
        const written = await this.client.version.create(request);

        return { first, key: { spaceId: this.spaceId, id: first.id }, request, written };
    }

    /** Claim an isolated migrated database of a dialect, provision the space, its vault and a member role, and host the vault. */
    static async open(dialect: Dialect): Promise<VaultFixture> {
        const test = await TestDatabase.create(dialect, spaceDatabase, { isMigrated: true });

        return VaultFixture.#start(test.database, () => test.close());
    }

    /** Open a SQLite file, migrate it, provision it unless a previous fixture did, and host the vault. */
    static async openFile(file: string, previous?: VaultFixture): Promise<VaultFixture> {
        const connection = await sqlite.connect(file, spaceDatabase);
        await connection.migrate(spaceDatabase.tables).catch(async (error: unknown) => {
            await connection.close();
            throw error;
        });

        return VaultFixture.#start(connection, () => connection.close(), previous);
    }

    /** Provision the space, its vault and a member role in a migrated database unless a previous fixture did, and host the vault. */
    static async #start(
        database: DatabaseConnection,
        close: () => Promise<void>,
        previous?: VaultFixture,
    ): Promise<VaultFixture> {
        const fixture = new VaultFixture(database, close, previous);
        try {
            if (previous === undefined) {
                await fixture.#provision();
            }
            fixture.caller = fixture.member();
            fixture.server = await fixture.host();
            fixture.client = fixture.connect(fixture.server);

            return fixture;
        } catch (error) {
            await close();
            throw error;
        }
    }

    /** Provision the space, its vault, and a role granting every vault permission to the account's members. */
    async #provision(): Promise<void> {
        // register the space and its vault
        const now = Date.now();
        await this.#register(now);

        // bind a role to the account's members on the space
        await this.#bindRole(now);

        // keep the vault's key
        const keyring = await this.keyring();
        await VaultKey.provision(this.database, keyring, LOCATION, {
            id: this.vaultId,
            scope: this.spaceId,
        });

        // grant the role every vault permission
        await this.#grant([
            ...[vault, secret, secretVersion].flatMap((object) =>
                object.permissions.map((name) => ({ type: object.name, name })),
            ),
            { type: space.name, name: "read", packageId: space.policy.definition.packageId },
        ]);
    }

    /** Register the space and its vault. */
    async #register(now: number): Promise<void> {
        // register the space
        await this.database.insert(space.table).values({
            id: this.spaceId,
            scope: this.accountId,
            name: "vault-fixture",
            createdAt: now,
            updatedAt: now,
        });

        // register its vault
        await this.database.insert(vault.table).values({
            id: this.vaultId,
            scope: this.spaceId,
            name: "credentials",
            definitionPackageId: VAULT.id,
            definitionVersion: VAULT.version,
            definitionName: "credentials",
            spec: {},
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
            packageId: spaceObject.space.policy.definition.packageId,
            type: spaceObject.space.name,
            ancestors: [this.accountId],
        });

        // bind the role to the account's members on the space
        await this.database.insert(accessRelationship).values(
            Relationship.encode(
                {
                    id: `relationship-${v7()}`,
                    object: spaceObject.space.reference(this.accountId, this.spaceId),
                    role: this.roleId,
                    subject: {
                        ...accountObject.account.reference(this.userId, this.accountId),
                        relation: "member",
                    },
                    createdAt: now,
                    expiresAt: null,
                },
                this.spaceId,
            ),
        );
    }

    /** Grant the role permissions in the space, vault permissions unless another package's, in one statement. */
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
