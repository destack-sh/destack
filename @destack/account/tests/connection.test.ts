import { eq } from "@destack/db";
import { ResourceId } from "@destack/resource";
import { RequestId } from "@destack/service/request";
import { expect, test } from "@destack/test";
import { v7 } from "uuid";
import { connection } from "../src/object/connection.ts";
import { Connections } from "../src/server/index.ts";
import { Digest } from "../src/object/digest.ts";
import { AccountFixture, type Person } from "./fixture.ts";

/** Authorize an OAuth account through the provider's page, keep its credential in the vault, and revoke it. */
test("authorize, complete and revoke oauth connections", async () => {
    await using fixture = await AccountFixture.open();
    const owner = await fixture.signIn("owner@example.com");
    const { accountId, spaceId } = await fixture.createSpace(owner);
    const vaultId = ResourceId.parse(`resource-${v7()}`);
    const github = fixture.providers.get("github")!;

    // start an authorization with its state and PKCE challenge on the provider page
    const authorize = () =>
        owner.client.connection.authorize({
            accountId,
            requestId: RequestId.create(),
            provider: "github",
            scopes: ["repo"],
            secretSpaceId: spaceId,
            vaultId,
        });
    const pending = await authorize();
    const page = new URL(pending.authorizationUrl!);
    const [held] = await fixture.opened.database
        .select({ verifier: connection.table.verifier })
        .from(connection.table)
        .where(eq(connection.table.id, pending.id));
    expect({
        userId: pending.userId,
        kind: pending.kind,
        issuer: pending.issuer,
        subject: pending.subject,
        page: `${page.origin}${page.pathname}`,
        state: page.searchParams.get("state"),
        challenge: page.searchParams.get("code_challenge"),
        authorizedAt: pending.authorizedAt,
        verifier: Object.hasOwn(pending, "verifier"),
    }).toEqual({
        userId: owner.id,
        kind: "oauth",
        issuer: "https://github.test",
        subject: null,
        page: "https://github.test/authorize",
        state: pending.state,
        challenge: await Digest.base64url(held!.verifier!),
        authorizedAt: null,
        verifier: false,
    });

    // refuse a callback carrying another state, and another user completing it
    const complete = (person: Person, parameters: Record<string, string>, id = pending.id) =>
        person.client.connection.complete({
            accountId,
            id,
            requestId: RequestId.create(),
            state: parameters.state!,
            parameters,
        });
    const callback = github.approve(pending.authorizationUrl!, "octocat");
    await expect(complete(owner, { ...callback, state: "forged" })).rejects.toMatchObject({
        code: "BAD_REQUEST",
        message: "connection state does not match",
    });
    const stranger = await fixture.signIn("stranger@example.com");
    await expect(complete(stranger, callback)).rejects.toMatchObject({
        code: "NOT_FOUND",
        message: `no scope ${accountId}`,
    });

    // complete it with the provider's callback and keep the credential in the chosen vault
    const active = await complete(owner, callback);
    const [secretId] = [...fixture.vault.secrets.keys()];
    expect(active).toEqual({
        ...pending,
        revision: 2,
        updatedAt: active.updatedAt,
        subject: "octocat",
        secretId,
        authorizationUrl: null,
        state: null,
        authorizedAt: active.updatedAt,
    });
    expect(fixture.vault.secrets.get(secretId!)).toEqual({
        spaceId,
        vaultId,
        name: pending.id,
        value: "credential-of-octocat",
        subject: owner.subject,
    });
    await expect(complete(owner, callback)).rejects.toMatchObject({
        code: "CONFLICT",
        message: "connection is authorized",
    });

    // cancel an abandoned authorization, and refuse completing one past its ten minutes
    const abandoned = await authorize();
    expect(
        await owner.client.connection.cancel({
            accountId,
            id: abandoned.id,
            requestId: RequestId.create(),
        }),
    ).toEqual({});
    const late = await authorize();
    await fixture.opened.database
        .update(connection.table)
        .set({ createdAt: Date.now() - 11 * 60 * 1000 })
        .where(eq(connection.table.id, late.id));
    await expect(
        complete(owner, github.approve(late.authorizationUrl!, "octocat"), late.id),
    ).rejects.toMatchObject({ code: "GONE", message: "connection authorization expired" });

    // revoke it at the provider, destroying its credential
    const revoked = await owner.client.connection.revoke({
        accountId,
        id: active.id,
        requestId: RequestId.create(),
    });
    expect(revoked.revokedAt).toBe(revoked.updatedAt);
    expect(github.revoked).toEqual([{ subject: "octocat", credential: "credential-of-octocat" }]);
    expect(fixture.vault.secrets.has(secretId!)).toBe(false);
    expect(
        (await owner.client.connection.list({ accountId })).items.map((item) => item.id).sort(),
    ).toEqual([active.id, late.id].sort());
});

/** Install a provider's application without a vault credential. */
test("connect installations without vaulted credentials", async () => {
    await using fixture = await AccountFixture.open();
    const owner = await fixture.signIn("owner@example.com");
    const { accountId, spaceId } = await fixture.createSpace(owner);
    const app = fixture.providers.get("github-app")!;

    // refuse a vault for an installation, and an unknown provider
    const refusals = await Promise.all(
        [
            { provider: "github-app", secretSpaceId: spaceId, vaultId: `resource-${v7()}` },
            { provider: "gitlab" },
        ].map((input) =>
            owner.client.connection
                .authorize({ accountId, requestId: RequestId.create(), scopes: [], ...input })
                .then(
                    () => "authorized",
                    (error: { code: string; message: string }) => [error.code, error.message],
                ),
        ),
    );
    expect(refusals).toEqual([
        ["BAD_REQUEST", "an installation keeps no credential in a vault"],
        ["BAD_REQUEST", "unknown provider gitlab"],
    ]);

    // install the application and record its installation
    const pending = await owner.client.connection.authorize({
        accountId,
        requestId: RequestId.create(),
        provider: "github-app",
        scopes: [],
    });
    const callback = app.approve(pending.authorizationUrl!, "acme");
    const installed = await owner.client.connection.complete({
        accountId,
        id: pending.id,
        requestId: RequestId.create(),
        state: callback.state!,
        parameters: callback,
    });
    expect({
        kind: installed.kind,
        subject: installed.subject,
        installationId: installed.installationId,
        permissions: installed.permissions,
        secretId: installed.secretId,
    }).toEqual({
        kind: "installation",
        subject: "acme",
        installationId: callback.installation_id,
        permissions: { contents: "read" },
        secretId: null,
    });
    expect(fixture.vault.secrets.size).toBe(0);

    // find the live installation of the application, narrowed by installation, until it is revoked
    const installations = (installationId?: string) =>
        Connections.installations(fixture.opened.database, {
            provider: "github-app",
            applicationId: app.applicationId,
            ...(installationId === undefined ? {} : { installationId }),
        });
    const live = [{ scope: accountId, id: installed.id, installationId: callback.installation_id }];
    expect([await installations(), await installations("installation-elsewhere")]).toEqual([
        live,
        [],
    ]);
    await owner.client.connection.revoke({
        accountId,
        id: installed.id,
        requestId: RequestId.create(),
    });
    expect(await installations()).toEqual([]);
});
