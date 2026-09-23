import { expect, test } from "@destack/test";
import { Caller, TokenIssuer, TokenVerifier } from "@destack/service/authentication";
import { createRequestId } from "@destack/service/request";
import { exportJWK, generateKeyPair, SignJWT } from "jose";
import { space, resource, vault, role, roleBinding, rolePermission } from "@destack/model/space";
import { AuditOutbox } from "@destack/audit/outbox";
import { connect } from "../../secret/client.ts";
import { vaultPackage } from "../../audit/index.ts";
import { VaultFixture } from "./fixture.ts";
import { PackageId } from "@destack/package";

/** Serve distinct tenants through one regional server while retaining exact token scopes. */
test("authorize multiple spaces through one regional vault", async () => {
    await using first = await VaultFixture.open();
    await using second = await VaultFixture.open();

    // provision the second tenant in the same regional database
    const database = first.database;
    await database.insert(space).values(await second.database.select().from(space));
    await database.insert(resource).values(await second.database.select().from(resource));
    await database.insert(vault).values(await second.database.select().from(vault));
    await database.insert(role).values(await second.database.select().from(role));
    await database.insert(roleBinding).values(await second.database.select().from(roleBinding));
    await database
        .insert(rolePermission)
        .values(await second.database.select().from(rolePermission));

    // embed vault procedures in a different receiving package, preserving vault permissions
    const audience = PackageId.parse("package-019f7480-0000-7000-8000-000000000099");
    // issue independent credentials for the same receiving service
    const keys = await generateKeyPair("ES256");
    const issuer = new TokenIssuer({
        issuer: "https://account.test",
        authority: { kind: "global" },
        sign: (payload) =>
            new SignJWT(payload)
                .setProtectedHeader({ alg: "ES256", kid: "current" })
                .sign(keys.privateKey),
    });
    const verifier = new TokenVerifier({
        issuer: "https://account.test",
        audience,
        authority: { kind: "global" },
        keys: { keys: [{ ...(await exportJWK(keys.publicKey)), alg: "ES256", kid: "current" }] },
    });
    const server = await first.host(
        first.vault,
        (request) => verifier.authenticate(request),
        audience,
    );
    const clients = [];
    for (const tenant of [first, second]) {
        const issued = await issuer.issue(
            new Caller({
                ...tenant.context.caller.authentication,
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

    // create metadata under each tenant without starting another server
    const created = [];
    for (const [index, tenant] of [first, second].entries()) {
        created.push(
            await clients[index]!.secret.create({
                spaceId: tenant.spaceId,
                vaultId: tenant.vaultId,
                name: "credential",
                requestId: createRequestId(),
            }),
        );
        expect(
            await clients[index]!.vault.get({
                spaceId: tenant.spaceId,
                vaultId: tenant.vaultId,
            }),
        ).toEqual({ spaceId: tenant.spaceId, resourceId: tenant.vaultId });
    }

    // reject token scope substitution before consulting another tenant's secret
    await expect(
        clients[0]!.secret.get({
            spaceId: second.spaceId,
            secretId: created[1]!.id,
        }),
    ).rejects.toMatchObject({ code: "UNAUTHORIZED" });
    await expect(
        clients[1]!.secret.get({
            spaceId: first.spaceId,
            secretId: created[0]!.id,
        }),
    ).rejects.toMatchObject({ code: "UNAUTHORIZED" });
    await expect(
        clients[1]!.secret.get({
            spaceId: second.spaceId,
            secretId: created[0]!.id,
        }),
    ).rejects.toMatchObject({ code: "NOT_FOUND" });

    // retain authoritative account ownership and correlate domain and procedure events
    const events = await new AuditOutbox(database).read(1000);
    const changes = events.filter((event) => event.action.name === "secret.create");
    expect(changes.map((event) => event.context.package)).toEqual([vaultPackage, vaultPackage]);
    expect(
        changes.map((event) => ({
            spaceId: event.context.spaceId,
            accountId: event.context.accountId,
            actor: event.context.actor,
            details: event.details,
        })),
    ).toEqual(
        [first, second].map((tenant) => ({
            spaceId: tenant.spaceId,
            accountId: tenant.context.caller.authentication.memberships![0]!.accountId,
            actor: { type: "user", id: tenant.userId },
            details: {},
        })),
    );
    for (const changed of changes) {
        expect(
            events
                .filter((event) => event.context.requestId === changed.context.requestId)
                .map((event) => event.result),
        ).toEqual([
            { stage: "attempt" },
            { stage: "result", outcome: "success" },
            { stage: "result", outcome: "success" },
        ]);
    }

    // prohibit caching for readiness, malformed requests, failures and shutdown
    for (const [path, status] of [
        ["/readyz", 200],
        ["/missing", 404],
        ["/secret/get", 401],
    ] as const) {
        const response = await server.fetch(
            new Request(`https://vault.test${path}`, {
                method: path === "/secret/get" ? "POST" : "GET",
                ...(path === "/secret/get"
                    ? {
                          headers: { "Content-Type": "application/json" },
                          body: JSON.stringify({
                              spaceId: first.spaceId,
                              secretId: created[0]!.id,
                          }),
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
    const stopped = await server.fetch(new Request("https://vault.test/vault/list"));
    expect([stopped.status, stopped.headers.get("Cache-Control")]).toEqual([503, "no-store"]);
});
