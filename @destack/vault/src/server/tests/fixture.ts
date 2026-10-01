import {
    accessRelationship,
    accessRole,
    accessRolePermission,
    principal,
    Relationship,
} from "@destack/access";
import type { Table } from "@destack/db";
import { Scope } from "@destack/sync";
import * as accountObject from "@destack/account/object";
import { and, eq, inArray, type DatabaseConnection, type Dialect } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import * as sqlite from "@destack/db/bun";
import type { PackageId } from "@destack/package";
import { ResourceContext } from "@destack/resource/context";
import { identifier, type Identifier } from "@destack/schema";
import { Caller } from "@destack/service/authentication";
import { ServiceError } from "@destack/service/error";
import { Health } from "@destack/service/health";
import { RequestId } from "@destack/service/request";
import { Server } from "@destack/service/server";
import { Journal } from "@destack/service/database";
import { AuditRecorder } from "@destack/audit";
import { AuditOutbox } from "@destack/audit/outbox";
import { ObjectServer } from "@destack/object/server";
import * as spaceObject from "@destack/space/object";
import { resource, space } from "@destack/space/object";
import { v7 } from "uuid";
import { type Keyring, LocalKeyring, VaultKey } from "../../encryption/index.ts";
import { secret, secretVersion, vault } from "../../object/index.ts";
import { connect } from "../../secret/client.ts";
import { vaultJournal, vaultTables } from "../../stack/index.ts";
import { spaceTables } from "@destack/space/stack";
import { servedObjects } from "../secret.ts";
import { implementService } from "../server.ts";

import { testJournalKey } from "@destack/service/test";

/** The tables of a test cell's regional database: the vaults, the spaces and every object server's own. */
export const cellTables: readonly Table[] = [...vaultTables, ...spaceTables];

/** The vault package, the audience of its callers. */
export const VAULT = vault.package;

/** The storage location the fixture's values authenticate. */
export const LOCATION = "eu";

/** How long the fixture's host keeps deleted secrets restorable. */
const RECOVERY = { days: 30 };

/** A migrated database, a provisioned vault, a member role on its space and an authenticated client. */
export class VaultFixture implements AsyncDisposable {
    /** The space with the vault. */
    readonly spaceId: Identifier<"space">;
    /** The provisioned vault resource. */
    readonly vaultId: Identifier<"resource">;
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
    caller!: Caller;
    /** The hosted vault. */
    server!: Server;
    /** The member's vault client. */
    client!: ReturnType<typeof connect>;
    /** Release the database. */
    readonly #close: () => Promise<void>;

    /** Retain the database, with the identities and root key of a previous fixture over it or new ones. */
    private constructor(
        database: DatabaseConnection,
        close: () => Promise<void>,
        previous?: VaultFixture,
    ) {
        this.database = database;
        this.#close = close;
        this.spaceId = previous?.spaceId ?? identifier("space").parse(`space-${v7()}`);
        this.vaultId = previous?.vaultId ?? identifier("resource").parse(`resource-${v7()}`);
        this.userId = previous?.userId ?? identifier("user").parse(`user-${v7()}`);
        this.accountId = previous?.accountId ?? identifier("account").parse(`account-${v7()}`);
        this.roleId = previous?.roleId ?? identifier("role").parse(`role-${v7()}`);
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
        authenticate: (request: Request) => Promise<Caller | null> = async (request) => {
            if (request.headers.get("Authorization") !== `Bearer ${this.userId}`) {
                throw new ServiceError("UNAUTHORIZED", { message: "invalid bearer credential" });
            }

            return this.caller;
        },
        audience: PackageId = VAULT.id,
    ): Promise<Server> {
        const server = Server.start({
            ...implementService({
                journalKey: testJournalKey,
                database: this.database,
                keyring: keyring ?? (await this.keyring()),
                location: LOCATION,
                recovery: RECOVERY,
            }),
            audience,
            resources: new ResourceContext(),
            health: new Health("vault"),
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
            objects: servedObjects(keyring, LOCATION, RECOVERY),
            policies: [space],
            database: this.database,
            journal: new Journal(vaultJournal, testJournalKey),
            audit: AuditRecorder.service(new AuditOutbox(this.database), {
                package: VAULT,
                service: "vault",
            }),
        });
    }

    /** Connect to a hosted vault as the fixture's member. */
    connect(server: Server): ReturnType<typeof connect> {
        return connect({
            url: "http://vault.test",
            headers: { Authorization: `Bearer ${this.userId}` },
            fetch: (request) => server.fetch(request),
        });
    }

    /** Authenticate the member of the space's account. */
    member(): Caller {
        return new Caller({
            credential: { kind: "fixture", id: "fixture-1" },
            audience: VAULT.id,
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
        const test = await TestDatabase.create(dialect, cellTables, { isMigrated: true });

        return VaultFixture.#start(test.database, () => test.close());
    }

    /** Open a SQLite file, migrate it, provision it unless a previous fixture did, and host the vault. */
    static async openFile(file: string, previous?: VaultFixture): Promise<VaultFixture> {
        const connection = await sqlite.connect(file, cellTables);
        await connection.migrate(cellTables).catch(async (error: unknown) => {
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

    /** Provision the space, its vault resource and facet, and a role granting every vault permission to the account's members. */
    async #provision(): Promise<void> {
        // register the space and its vault resource
        const now = Date.now();
        await this.database.insert(space.table).values({
            id: this.spaceId,
            scope: this.accountId,
            name: "vault-fixture",
            createdAt: now,
            updatedAt: now,
        });
        await this.database.insert(resource.table).values({
            id: this.vaultId,
            scope: this.spaceId,
            name: "credentials",
            kind: "vault",
            definitionPackageId: VAULT.id,
            definitionVersion: VAULT.version,
            definitionName: "credentials",
            spec: {},
            createdAt: now,
            updatedAt: now,
        });

        // bind a role granting every vault permission to the account's members on the space
        await this.database.insert(accessRole).values({
            id: this.roleId,
            scope: this.spaceId,
            name: "vault-owner",
            description: "manage the fixture vault",
            createdAt: now,
            updatedAt: now,
        });
        await this.database.insert(Scope.table).values({
            scope: this.spaceId,
            parent: this.accountId,
            packageId: spaceObject.space.policy.definition.packageId,
            type: spaceObject.space.name,
            ancestors: [this.accountId],
        });
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

        // keep the vault's key and create its facet, as the provider and the resource controller do
        const keyring = await this.keyring();
        await VaultKey.provision(this.database, keyring, LOCATION, this.vaultId);
        await this.database.transaction((transaction) =>
            this.system(keyring).invoke(
                transaction,
                this.spaceId,
                vault,
                "create",
                { id: this.vaultId },
                now,
            ),
        );

        // grant the role every vault permission
        await this.#grant([
            ...[vault, secret, secretVersion].flatMap((object) =>
                object.permissions.map((name) => ({ type: object.name, name })),
            ),
            { type: space.name, name: "read", packageId: space.policy.definition.packageId },
        ]);
    }

    /** Grant the role permissions in the space, vault permissions unless another package's, in one statement. */
    async #grant(
        permissions: readonly { type: string; name: string; packageId?: PackageId }[],
    ): Promise<void> {
        await this.database.insert(accessRolePermission).values(
            permissions.map(({ type, name, packageId }) => ({
                id: identifier("role-permission").parse(`role-permission-${v7()}`),
                roleId: this.roleId,
                scope: this.spaceId,
                packageId: packageId ?? VAULT.id,
                type,
                name,
            })),
        );
    }
}
