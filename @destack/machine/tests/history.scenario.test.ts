import { Snapshot } from "@destack/db";
import { installation } from "@destack/object";
import { rollout } from "@destack/stack/object";
import { OWNER_EMAIL, TestComputer, TestDistribution } from "@destack/machine/computer/test";
import { expect, test } from "@destack/test";
import { present } from "@destack/schema";
import { entry } from "@destack/machine-journal-fixture/object";
import binderPackage from "@destack/machine-binder-fixture/package";

test("serve a tracked type that reads through its space in a team space, the space up beside it", async () => {
    // create a team space in the owner's account on the home computer
    const { home } = await TestComputer.home();
    const { world } = home;
    const login = await home.signIn(OWNER_EMAIL);
    const space = await home.space(login, world.owner.account, "team");

    // publish the journal as the platform's operator, and install it in the team space
    const build = await TestDistribution.fixture("journal");
    await home.publish(TestDistribution.lockOf("journal"));
    await home.install(login.id, space, build, "journal");
    await home.ready(space);

    // serve the journal's entries in the space's store, the space up beside them
    const served = await home.machine.space(space);
    const store = await served.store();
    expect({
        isServed: store.objects.some((type) => type.same(entry)),
        health: served.health.status,
    }).toEqual({ isServed: true, health: "up" });
});

test("refuse serving an installation whose tracked type reads through a type without history, the space up beside it", async () => {
    // create a team space in the owner's account on the home computer
    const { home } = await TestComputer.home();
    const { world } = home;
    const login = await home.signIn(OWNER_EMAIL);
    const space = await home.space(login, world.owner.account, "team");

    // install the binders, whose sheets read through binders keeping no history
    const build = await TestDistribution.fixture("binder");
    await home.publish(TestDistribution.lockOf("binder"));
    const installing: unknown = await home
        .install(login.id, space, build, "binder")
        .catch((error: unknown) => error);

    // read the rollout's refusal and the installation's failure
    const served = await home.machine.space(space);
    const snapshot = Snapshot.live(served.database);
    const installed = present(
        await snapshot
            .read(installation.table)
            .findFirst({ where: { scope: space, packageId: binderPackage.id } }),
        "the binders' installation",
    );
    const rolled = await snapshot
        .read(rollout.table)
        .findFirst({ where: { installationId: installed.id } });
    const reason = "object 'sheet' records history but reads through 'binder', which has no history";
    const failure = `@destack/stack SERVING_BLOCKED: cannot serve revision ${installed.revisionId}: ${reason}`;

    // refuse it alone, naming why, and keep the space up
    expect({
        installing,
        refusal: rolled?.refusal,
        synced: installed.conditions["Synced"]?.message,
        health: served.health.status,
        captured: world.telemetry.take().map((captured) => captured.message),
    }).toEqual({
        installing: new TypeError(`'binder' did not apply: '${failure}'`),
        refusal: reason,
        synced: failure,
        health: "up",
        captured: [`cannot serve revision ${installed.revisionId}: ${reason}`],
    });
});
