import * as turso from "@destack/db/turso";
import * as postgres from "@destack/db/postgres";
import { orderSchemas, sql, eq, and, type DatabaseConnection } from "@destack/db";
import { migrate } from "@destack/db/migration";
import { space, resource, vault, role, roleBinding, rolePermission } from "@destack/model/regional";
import { vaultService } from "../../service/index.ts";
import { identifier } from "@destack/schema";
import { AuditRecorder } from "@destack/audit";
import { AuditOutbox } from "@destack/audit/outbox";
import { Server } from "@destack/service/server";
import { ResourceContext } from "@destack/resource/context";
import { Health } from "@destack/service/health";
import { ServiceError } from "@destack/service/error";
import { createRequestId } from "@destack/service/request";
import { Caller, TokenIssuer, TokenVerifier } from "@destack/service/authentication";
import { generateKeyPair, exportJWK, SignJWT } from "jose";
import { v7 } from "uuid";
import { vaultSchema } from "../../stack/index.ts";
import { implementService } from "../../server/index.ts";
import { Vault, type VaultContext } from "../../vault/index.ts";
import { LocalKeyring, EnvelopeEncryption } from "../../encryption/index.ts";
import { vaultPackage } from "../../audit/index.ts";
import { connect } from "../../secret/client.ts";
import type { PackageId } from "@destack/package";

/** A migrated database, provisioned vault and authenticated HTTP client. */
export class VaultFixture implements AsyncDisposable {
    /** Region-qualified tenant identity. */
    readonly spaceId = identifier("space").parse(`space-${v7()}`);
    /** Provisioned vault resource. */
    readonly vaultId = identifier("resource").parse(`resource-${v7()}`);
    /** Verified user identity. */
    readonly userId = identifier("user").parse(`user-${v7()}`);
    /** Old key retained across root-key rotation. */
    readonly root = crypto.getRandomValues(new Uint8Array(32));
    /** Database opened by this fixture. */
    readonly database: DatabaseConnection;
    /** Production encrypted storage. */
    vault!: Vault;
    /** Standard service hosting lifecycle. */
    server!: Server;
    /** Hosted servers closed before their shared database. */
    readonly servers: Server[] = [];
    /** Typed HTTP client. */
    client!: ReturnType<typeof connect>;
    /** Host-authenticated request context. */
    context!: VaultContext;
    /** Persisted role used by the authenticated member. */
    readonly roleId = identifier("role").parse(`role-${v7()}`);
    /** Verified account membership. */
    readonly membershipId = identifier("account-membership").parse(`account-membership-${v7()}`);
    /** Physical database cleanup. */
    readonly #closeDatabase: () => Promise<void>;

    /** Close the isolated database after a scenario. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.close();
    }

    /** Drain service requests before releasing the database. */
    async close(): Promise<void> {
        for (const server of this.servers) {
            await server.close();
        }

        await this.#closeDatabase();
    }

    /** Retain the isolated database before provisioning. */
    private constructor(database: DatabaseConnection, close: () => Promise<void>) {
        this.database = database;
        this.#closeDatabase = close;
    }

    /** Revoke or restore the member's persisted role binding. */
    async allow(allowed: boolean): Promise<void> {
        await this.database
            .update(roleBinding)
            .set({ revokedAt: allowed ? null : Date.now() })
            .where(eq(roleBinding.roleId, this.roleId));
    }

    /** Remove or restore the separate plaintext permission. */
    async allowRead(allowed: boolean): Promise<void> {
        if (allowed) {
            await this.database.insert(rolePermission).values({
                id: identifier("role-permission").parse(`role-permission-${v7()}`),
                roleId: this.roleId,
                packageId: vaultPackage.id,
                type: "version",
                name: "read",
            });
        } else {
            await this.database
                .delete(rolePermission)
                .where(
                    and(
                        eq(rolePermission.roleId, this.roleId),
                        eq(rolePermission.type, "version"),
                        eq(rolePermission.name, "read"),
                    ),
                );
        }
    }

    /** Authenticate a captured host identity through real signed tokens and the production verifier. */
    async signCaller(spaceId?: string): Promise<void> {
        // retain the supplied trusted host identity while replacing HTTP authentication
        const keys = await generateKeyPair("ES256");
        const publicKey = { ...(await exportJWK(keys.publicKey)), kid: "fixture", alg: "ES256" };
        const issuer = new TokenIssuer({
            issuer: "https://authority.test",
            authority: spaceId ? { kind: "space", spaceId } : { kind: "global" },
            sign: (payload) =>
                new SignJWT(payload)
                    .setProtectedHeader({ alg: "ES256", kid: "fixture" })
                    .sign(keys.privateKey),
        });
        const current = this.context.caller.authentication;
        const issued = await issuer.issue(
            new Caller({
                ...current,
                credential: { kind: "fixture", id: "fixture-credential" },
                scope: this.spaceId,
            }),
        );
        const verifier = new TokenVerifier({
            issuer: "https://authority.test",
            audience: vaultPackage.id,
            authority: spaceId ? { kind: "space", spaceId } : { kind: "global" },
            keys: { keys: [publicKey] },
        });

        // keep the same signed credential through subsequent database revocation checks
        const server = await this.host(this.vault, (request) => verifier.authenticate(request));
        this.client = connect({
            url: "https://vault.test",
            headers: { authorization: `Bearer ${issued.accessToken}` },
            fetch: (request) => server.fetch(request),
        });
    }

    /** Host production procedures under the standard authentication and shutdown lifecycle. */
    async host(
        vault: Vault,
        authenticate: (request: Request) => Promise<Caller | null> = async (request) => {
            if (request.headers.get("Authorization") !== `Bearer ${this.userId}`) {
                throw new ServiceError("UNAUTHORIZED");
            }

            return this.context.caller;
        },
        audience: PackageId = vaultPackage.id,
    ): Promise<Server> {
        const server = Server.start({
            ...implementService(vault),
            audience,
            scope: "space-00000000-0000-4000-8000-000000000001",
            resources: new ResourceContext(),
            health: new Health("vault"),
            authenticate,
            authorizeHost: async ({ access }) => {
                if (access.authentication === "host") {
                    throw new ServiceError("FORBIDDEN");
                }
            },
            drainTimeout: 1000,
        });
        this.servers.push(server);

        return server;
    }

    /** Create one credential through the service for scenarios requiring an existing value. */
    async createSecret() {
        const first = await this.client.secret.create({
            spaceId: this.spaceId,
            vaultId: this.vaultId,
            name: "first",
            requestId: createRequestId(),
        });
        const key = { spaceId: this.spaceId, secretId: first.id };
        const request = {
            ...key,
            requestId: createRequestId(),
            revision: first.revision,
            value: { encoding: "text" as const, value: "credential" },
        };
        const written = await this.client.version.write(request);

        return { first, key, request, written };
    }

    /** Migrate and provision the same schemas used by the host. */
    static async open(file?: string): Promise<VaultFixture> {
        const address = file ? undefined : process.env.DESTACK_TEST_POSTGRES;
        let database: DatabaseConnection;
        let close: () => Promise<void>;
        if (address) {
            const administration = await postgres.connect(address);
            const name = `vault_${crypto.randomUUID().replaceAll("-", "")}`;
            await administration.execute(sql`CREATE DATABASE ${sql.identifier(name)}`);
            const url = new URL(address);
            url.pathname = `/${name}`;
            const connection = await postgres.connect(url.href, vaultSchema);
            database = connection;
            close = async () => {
                await connection.close();
                await administration.execute(sql`DROP DATABASE ${sql.identifier(name)}`);
                await administration.close();
            };
        } else {
            const connection = await turso.connect(file ?? ":memory:", vaultSchema);
            database = connection;
            close = () => connection.close();
        }

        const fixture = new VaultFixture(database, close);
        try {
            for (const schema of orderSchemas([vaultSchema])) {
                await migrate(database, schema);
            }

            // provision resource metadata as the space controller would
            const now = Date.now();
            const accountId = identifier("account").parse(`account-${v7()}`);
            await database.insert(space).values({
                id: fixture.spaceId,
                accountId,
                name: "vault fixture",
                authorityRegionId: identifier("region").parse(`region-${v7()}`),
                authorityEpoch: 1,
                createdAt: now,
                updatedAt: now,
            });
            await database.insert(resource).values({
                id: fixture.vaultId,
                spaceId: fixture.spaceId,
                name: "credentials",
                kind: "vault",
                definitionPackageId: vaultPackage.id,
                definitionVersion: vaultPackage.version,
                definitionName: "credentials",
                spec: {},
                createdAt: now,
                updatedAt: now,
            });
            await database
                .insert(vault)
                .values({ resourceId: fixture.vaultId, spaceId: fixture.spaceId });

            // grant each declared operation explicitly through persisted regional RBAC
            await database.insert(role).values({
                id: fixture.roleId,
                spaceId: fixture.spaceId,
                name: "vault-owner",
                description: "manage the fixture vault",
                createdAt: now,
                updatedAt: now,
            });
            await database.insert(roleBinding).values({
                id: identifier("role-binding").parse(`role-binding-${v7()}`),
                accountId,
                roleId: fixture.roleId,
                spaceId: fixture.spaceId,
                accountMembershipId: fixture.membershipId,
                createdAt: now,
                updatedAt: now,
            });
            for (const [type, operations] of Object.entries(vaultService.router)) {
                for (const name of Object.keys(operations)) {
                    await database.insert(rolePermission).values({
                        id: identifier("role-permission").parse(`role-permission-${v7()}`),
                        roleId: fixture.roleId,
                        packageId: vaultPackage.id,
                        type,
                        name,
                    });
                }
            }
            const keys = await LocalKeyring.import("one", new Map([["one", fixture.root]]));
            fixture.vault = new Vault(database, new EnvelopeEncryption(keys), "eu");

            // bind verified identity and transaction-aware authority as the host does
            fixture.context = {
                audience: vaultPackage.id,
                caller: new Caller({
                    credential: { kind: "fixture" },
                    audience: vaultPackage.id,
                    verifiedAt: Date.now(),
                    expiresAt: Date.now() + 60000,
                    subject: { kind: "user", authority: "global", id: fixture.userId },
                    subjects: [{ kind: "user", authority: "global", id: fixture.userId }],
                    memberships: [
                        {
                            subject: { kind: "user", authority: "global", id: fixture.userId },
                            accountId,
                            id: fixture.membershipId,
                        },
                    ],
                }),
                audit: new AuditRecorder(
                    {
                        actor: { type: "user", authority: "global", id: fixture.userId },
                        delegation: [],
                        package: vaultPackage,
                        service: "vault",
                        spaceId: fixture.spaceId,
                        accountId,
                    },
                    new AuditOutbox(database),
                ),
            };
            fixture.server = await fixture.host(fixture.vault);
            fixture.client = connect({
                url: "http://vault.test",
                headers: { Authorization: `Bearer ${fixture.userId}` },
                fetch: (request) => fixture.server.fetch(request),
            });

            return fixture;
        } catch (error) {
            await close();
            throw error;
        }
    }
}
