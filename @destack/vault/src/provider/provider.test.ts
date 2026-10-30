import { eq, Snapshot } from "@destack/db";
import { binding, Rollout, rollout } from "@destack/stack/object";
import { ids } from "@destack/stack/test";
import { expect, refusal, single, test } from "@destack/test";
import { vault } from "../object/index.ts";
import { CONSUMER, CREDENTIAL, CREDENTIALS, TestVault } from "../test/index.ts";
import { defineVault, VAULT_PROVIDER } from "../declare/index.ts";
import { VAULT_REFERENCE, vaultProvider } from "./provider.ts";

test("provision the vault an installed build declares at the vault package's address", async () => {
    // install an application declaring its credentials vault
    const fixture = await TestVault.open();

    // record the vault provider and its address on the provisioned vault
    const provisioned = single(
        await fixture.served.database
            .select({ provider: vault.table.provider, reference: vault.table.reference })
            .from(vault.table),
    );
    expect(provisioned).toEqual({ provider: VAULT_PROVIDER, reference: VAULT_REFERENCE });
});

test("read a bound vault's secret in process as its installation, and refuse another installation's vault", async () => {
    // install the application with a secret in its vault, and another application with its own vault
    const fixture = await TestVault.open();
    await fixture.createSecret(fixture.installed(fixture.installation));
    const other = CONSUMER;
    const built = await TestVault.build(
        other,
        [defineVault({ name: CREDENTIALS, spec: {} }, { package: other })],
        [],
        fixture.environment.crypto,
    );
    const otherInstallation = await fixture.space.install(fixture.served, built);
    const bindingOf = async (installationId: typeof otherInstallation) =>
        single(
            await fixture.served.database
                .select()
                .from(binding.table)
                .where(eq(binding.table.installationId, installationId)),
        );
    const strayBinding = await bindingOf(otherInstallation);
    const strayId = strayBinding.target;

    // connect the application to its own vault and to the other application's, as its server in process
    const { database } = fixture.served;
    const store = Promise.resolve(fixture.served.store);
    const snapshot = Snapshot.live(database);
    const rows = await snapshot
        .read(rollout.table)
        .findMany({ where: { id: fixture.installation } });
    const row = single(await Rollout.read(snapshot, rows));
    const own = await vaultProvider.connect(store, row, await bindingOf(fixture.installation));
    const stray = await vaultProvider.connect(store, row, strayBinding);

    // read the secret of its own vault, and find no vault of the other's, which it may not see
    const read = await own.read("first");
    expect({
        reference: own.reference,
        read: [read.version, read.value],
        refused: await refusal(stray.read("first")),
    }).toEqual({
        reference: vault.reference(ids.space, await fixture.vault()),
        read: [1, CREDENTIAL],
        refused: ["NOT_FOUND", "NOT_FOUND", `vault ${strayId} not found`],
    });
});
