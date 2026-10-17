import { eq, Snapshot } from "@destack/db";
import type { MethodContext } from "@destack/object";
import { Package } from "@destack/package";
import { schema, Identifier } from "@destack/schema";
import { RequestId } from "@destack/runtime/request";
import { binding, installation } from "@destack/stack/object";
import type { ResourceNeed } from "@destack/stack/server";
import { expect, refusal, single, test } from "@destack/test";
import { SECRET_KIND, VAULT_PROVIDER } from "../declare/index.ts";
import { secret, SecretVersion, secretVersion, vault } from "../object/index.ts";
import vaultPackage from "../package.ts";
import { VaultFixture } from "../test/index.ts";
import { secretBindable } from "./secret.ts";
import { implementVault } from "./vault.ts";

/** The package whose installation binds the vault and the secrets. */
const notes = Package.parse({
    id: "package-01996ab0-0000-7000-8000-0000000000a1",
    name: "@example/notes",
    version: "2026.9.0",
});

/** A declaration of a secret the space generates in the package's vault named push. */
const generated: ResourceNeed = {
    name: "push-key",
    kind: SECRET_KIND,
    spec: { generated: { vault: "push", algorithm: "ES256" } },
    package: notes,
    state: {},
};

/** Run a function with a call of the fixture's vault, as the stack runs bindables in its calls. */
async function within<Result>(
    fixture: VaultFixture,
    run: (context: MethodContext) => Promise<Result>,
): Promise<Result> {
    // run the function inside an update of the vault
    const results: { readonly result: Result }[] = [];
    const vaults = implementVault().implement({
        methods: {
            update: async (call, next) => {
                results.push({ result: await run(call) });

                return next();
            },
        },
    });
    const store = fixture.objects(vaults);
    const row = single(
        await fixture.database
            .select()
            .from(vault.table)
            .where(eq(vault.table.id, fixture.vaultId)),
    );
    const now = Date.now();
    await store.mutate(store.serving(vault), {
        id: RequestId.create(now),
        calls: [vault.call("update", { ...vault.target(row), revision: row.revision })],
    });

    return single(results).result;
}

test("provision a generated secret in the vault the installation binds, generate its first version once the vault is ready, and capture it", async () => {
    // install notes, binding the fixture's vault as its push vault
    await using fixture = await VaultFixture.open("sqlite");
    const { database, spaceId, vaultId } = fixture;
    const now = Date.now();
    const installationId = Identifier.create("installation");
    await database.insert(installation.table).values({
        id: installationId,
        scope: spaceId,
        packageId: notes.id,
        role: "application",
        name: "notes",
        selection: { kind: "release", version: notes.version },
        createdAt: now,
        updatedAt: now,
    });
    const bind = async (name: string, target: string) => {
        const id = Identifier.create("binding");
        const row = { id, scope: spaceId, installationId, packageId: notes.id, name, target };
        await database
            .insert(binding.table)
            .values({ ...row, version: null, state: {}, createdAt: now, updatedAt: now });

        return single(await database.select().from(binding.table).where(eq(binding.table.id, id)));
    };
    const installed = single(await database.select().from(installation.table));
    const bindings = [await bind("push", vaultId)];

    // leave a secret a stack binds, refuse one in a vault the installation lacks, and provision the generated one
    const unbound = await within(fixture, (call) =>
        secretBindable.provision(call, installed, { ...generated, spec: {} }, bindings),
    );
    const elsewhere = { generated: { vault: "mail", algorithm: "ES256" } };
    const missing = await refusal(
        within(fixture, (call) =>
            secretBindable.provision(call, installed, { ...generated, spec: elsewhere }, bindings),
        ),
    );
    const provisioned = await within(fixture, (call) =>
        secretBindable.provision(call, installed, generated, bindings),
    );
    const id = schema.identifier("secret").parse(provisioned);

    // wait for the vault's provisioning, then generate the first version, then pass
    const bound = await bind("push-key", id);
    const ready = () => within(fixture, (call) => secretBindable.ready(call, bound, generated));
    const waiting = await ready();
    await database
        .update(vault.table)
        .set({ provider: VAULT_PROVIDER, reference: vaultPackage.name })
        .where(eq(vault.table.id, vaultId));
    const generating = await ready();
    const passed = await ready();

    // read the generated key and what a capture of each version uses
    const row = single(await database.select().from(secret.table));
    const version = single(await database.select().from(secretVersion.table));
    const value = SecretVersion.requireReadable(version, Date.now());
    const jwk = schema
        .looseObject({ kty: schema.string(), crv: schema.string(), alg: schema.string() })
        .parse(JSON.parse(value.value));
    const snapshot = Snapshot.live(database);
    const uses = (captured: number | null) =>
        secretBindable.uses({ scope: spaceId, target: id, version: captured }, snapshot);
    expect({
        unbound,
        missing,
        readiness: [waiting, generating, passed],
        secret: [row.id, row.parentId, row.name, row.currentVersion],
        key: [jwk.kty, jwk.crv, jwk.alg],
        versions: [secretBindable.version(row, null), secretBindable.version(row, 3)],
        uses: [await uses(null), await uses(1)],
    }).toEqual({
        unbound: undefined,
        missing: [
            "PRECONDITION_FAILED",
            "secret push-key lives in vault mail, which the installation binds nowhere",
        ],
        readiness: [false, false, true],
        secret: [id, vaultId, "push-key", 1],
        key: ["EC", "P-256", "ES256"],
        versions: [1, 3],
        uses: [
            [secret.reference(spaceId, id)],
            [secret.reference(spaceId, id), secretVersion.reference(spaceId, version.id)],
        ],
    });
});
