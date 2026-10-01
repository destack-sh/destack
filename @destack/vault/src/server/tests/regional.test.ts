import { TEST_DIALECTS } from "@destack/db/test";
import { Scope } from "@destack/sync";
import { accessRelationship, accessRole, accessRolePermission, principal } from "@destack/access";
import { AuditOutbox } from "@destack/audit/outbox";
import { PackageId } from "@destack/package";
import { Caller, TokenIssuer, TokenVerifier } from "@destack/service/authentication";
import { ServiceError } from "@destack/service/error";
import { RequestId } from "@destack/service/request";
import { resource, space } from "@destack/space/object";
import { expect, test } from "@destack/test";
import { exportJWK, generateKeyPair, SignJWT } from "jose";
import { vault } from "../../object/index.ts";
import { connect } from "../../secret/client.ts";
import { VAULT, VaultFixture } from "./fixture.ts";
import { VERSION_HEADER } from "@destack/service/request";
import { vaultService } from "../../service/index.ts";

test.each(TEST_DIALECTS)(
    "authorize several spaces through one regional vault on %s",
    async (dialect) => {
        await using first = await VaultFixture.open(dialect);
        await using second = await VaultFixture.open(dialect);

        // provision the second tenant in the same regional database
        const database = first.database;
        await database.insert(space.table).values(await second.database.select().from(space.table));
        await database
            .insert(resource.table)
            .values(await second.database.select().from(resource.table));
        await database.insert(vault.table).values(await second.database.select().from(vault.table));
        await database.insert(accessRole).values(await second.database.select().from(accessRole));
        await database
            .insert(accessRelationship)
            .values(await second.database.select().from(accessRelationship));
        await database
            .insert(Scope.table)
            .values(await second.database.select().from(Scope.table))
            .onConflictDoNothing();
        await database
            .insert(accessRolePermission)
            .values(await second.database.select().from(accessRolePermission));

        // serve the vault's procedures in another receiving package under signed tokens
        const audience = PackageId.parse("package-019f7480-0000-7000-8000-000000000099");
        const keys = await generateKeyPair("ES256");
        const issuer = new TokenIssuer({
            issuer: "https://account.test",
            authority: { kind: "universe" },
            sign: (payload) =>
                new SignJWT(payload)
                    .setProtectedHeader({ alg: "ES256", kid: "current" })
                    .sign(keys.privateKey),
        });
        const verifier = new TokenVerifier({
            issuer: "https://account.test",
            audience,
            authority: { kind: "universe" },
            keys: {
                keys: [{ ...(await exportJWK(keys.publicKey)), alg: "ES256", kid: "current" }],
            },
        });
        const server = await first.host(
            undefined,
            (request) => verifier.authenticate(request),
            audience,
        );
        const clients = [];
        for (const tenant of [first, second]) {
            const issued = await issuer.issue(
                new Caller({
                    ...tenant.caller.authentication,
                    audience,
                    scope: tenant.spaceId,
                    credential: { kind: "personal", id: tenant.userId },
                }),
            );
            clients.push(
                connect({
                    url: "https://vault.test",
                    headers: { authorization: `Bearer ${issued.accessToken}` },
                    fetch: (request) => server.fetch(request),
                }),
            );
        }

        // create a secret in each tenant's vault through the same server
        const created = [];
        for (const [index, tenant] of [first, second].entries()) {
            created.push(
                await clients[index]!.secret.create({
                    spaceId: tenant.spaceId,
                    parentId: tenant.vaultId,
                    name: "credential",
                    requestId: RequestId.create(),
                }),
            );
            expect(
                (await clients[index]!.vault.get({ spaceId: tenant.spaceId, id: tenant.vaultId }))
                    .id,
            ).toBe(tenant.vaultId);
        }

        // hide another tenant's secret, named in its own space or in the token's
        for (const [client, spaceId, id, message] of [
            [
                clients[0]!,
                second.spaceId,
                created[1]!.id,
                `scope ${second.spaceId} is outside the pinned scope`,
            ],
            [
                clients[1]!,
                first.spaceId,
                created[0]!.id,
                `scope ${first.spaceId} is outside the pinned scope`,
            ],
            [clients[1]!, second.spaceId, created[0]!.id, `no secret ${created[0]!.id}`],
        ] as const) {
            await expect(client.secret.get({ spaceId, id })).rejects.toEqual(
                new ServiceError("NOT_FOUND", { defined: true, message }),
            );
        }

        // refuse changes in a space other than the token's
        await expect(
            clients[0]!.secret.create({
                spaceId: second.spaceId,
                parentId: second.vaultId,
                name: "substituted",
                requestId: RequestId.create(),
            }),
        ).rejects.toEqual(
            new ServiceError("NOT_FOUND", {
                defined: true,
                message: `scope ${second.spaceId} is outside the pinned scope`,
            }),
        );

        // record each space's changes in its own history
        const events = await new AuditOutbox(database).read(1000);
        const changes = events.filter(
            (event) => event.action.name === "Secret.create" && event.result.stage === "result",
        );
        expect(
            changes.map((event) => ({
                package: event.context.package,
                scope: event.context.scope,
                actor: event.context.actor,
            })),
        ).toEqual(
            [first, second].map((tenant) => ({
                package: VAULT,
                scope: tenant.spaceId,
                actor: {
                    type: "subject",
                    subject: principal.user.reference("universe", tenant.userId),
                },
            })),
        );

        // prohibit caching of readiness, unknown routes and failures, and after shutdown
        for (const [path, status] of [
            ["/readyz", 200],
            ["/missing", 404],
            [`/spaces/${first.spaceId}/secrets`, 401],
        ] as const) {
            const isCreate = path.endsWith("/secrets");
            const response = await server.fetch(
                new Request(`https://vault.test${path}`, {
                    method: isCreate ? "POST" : "GET",
                    ...(isCreate
                        ? {
                              headers: {
                                  "Content-Type": "application/json",
                                  [VERSION_HEADER]: vaultService.package.version,
                              },
                              body: JSON.stringify({ parentId: first.vaultId, name: "anonymous" }),
                          }
                        : {}),
                }),
            );
            expect([response.status, response.headers.get("Cache-Control")]).toEqual([
                status,
                "no-store",
            ]);
            await response.text();
        }
        await server.close();
        const stopped = await server.fetch(new Request("https://vault.test/readyz"));
        expect([stopped.status, stopped.headers.get("Cache-Control")]).toEqual([503, "no-store"]);
    },
);
