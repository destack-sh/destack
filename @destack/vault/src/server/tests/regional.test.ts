import { TEST_DIALECTS } from "@destack/db/test";
import { testCallKey } from "@destack/service/test";
import { Journal, AuditCaller } from "@destack/audit";
import { Scope } from "@destack/sync";
import { accessRelationship, accessRole, accessRolePermission, principal } from "@destack/access";
import { PackageId } from "@destack/package";
import { Authentication, TokenIssuer, TokenVerifier } from "@destack/service/authentication";
import { ServiceError } from "@destack/service/error";
import { RequestId } from "@destack/service/request";
import { space } from "@destack/space/object";
import { expect, test } from "@destack/test";
import { exportJWK, generateKeyPair, SignJWT } from "jose";
import { vault } from "../../object/index.ts";
import { SecretClient } from "../../object/index.ts";
import { spaceService } from "@destack/space/service";
import { SPACE, VaultFixture } from "./fixture.ts";

test.each(TEST_DIALECTS)(
    "authorize several spaces through one regional vault on %s",
    async (dialect) => {
        await using first = await VaultFixture.open(dialect);
        await using second = await VaultFixture.open(dialect);

        // provision the second tenant in the same regional database
        const database = first.database;
        await database.insert(space.table).values(await second.database.select().from(space.table));
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
                new Authentication({
                    ...tenant.caller.claims,
                    audience,
                    scope: tenant.spaceId,
                    credential: { kind: "personal", id: tenant.userId },
                }),
            );
            clients.push(
                new SecretClient(spaceService, {
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
        const calls = await new Journal(database, testCallKey).read({ limit: 1000 });
        const changes = calls.filter((call) => call.method === "secret.create");
        expect(
            changes.map((call) => ({
                package: call.execution!.context.package,
                scope: call.execution!.context.scope,
                actor: AuditCaller.actor(call.execution!.context.caller),
            })),
        ).toEqual(
            [first, second].map((tenant) => ({
                package: SPACE,
                scope: tenant.spaceId,
                actor: {
                    type: "subject",
                    subject: principal.user.reference("universe", tenant.userId),
                },
            })),
        );
    },
);
