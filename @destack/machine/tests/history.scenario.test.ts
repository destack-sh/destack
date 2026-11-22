import { installation } from "@destack/stack/object";
import type { DatabaseConnection } from "@destack/db/database";
import { OWNER_EMAIL, TestComputer, TestDistribution } from "@destack/machine/computer/test";
import { expect, test } from "@destack/test";
import { type Identifier, present } from "@destack/schema";
import { entry } from "@destack/machine-journal-fixture/object";
import journalPackage from "@destack/machine-journal-fixture/package";
import binderPackage from "@destack/machine-binder-fixture/package";

test("serve a tracked type reading through its space, keeping the space up", async () => {
    // create a team space in the owner's account on the home computer
    const { home } = await TestComputer.home();
    const { world } = home;
    const login = await home.signIn(OWNER_EMAIL);
    const space = await home.space(login.id, world.owner.account, "team");

    // publish the journal as the platform's operator, and install it in the team space
    const build = await TestDistribution.fixture("journal");
    await home.publish(TestDistribution.imageOf("journal"));
    await home.install(login.id, space, build, "journal");
    await home.ready(space);

    // serve the journal's entries in the space's store, its installation reading why, the space up beside them
    const served = await home.machine.space(space);
    const store = await served.store();
    const installed = present(
        await served.database
            .snapshot()
            .read(installation.table)
            .findFirst({ where: { scope: space, packageId: journalPackage.id } }),
        "the journal's installation",
    );
    const serving = installed.conditions["Serving"];
    expect({
        isServed: store.objects.some((type) => type.same(entry)),
        serving: [serving?.status, serving?.reason, serving?.message],
        health: served.health.status,
    }).toEqual({
        isServed: true,
        serving: [
            "true",
            "Served",
            `serving revision ${await targeted(served.database, installed.id)}`,
        ],
        health: "up",
    });
});

test("refuse serving a type reading through a type without history", async () => {
    // create a team space in the owner's account on the home computer
    const { home } = await TestComputer.home();
    const { world } = home;
    const login = await home.signIn(OWNER_EMAIL);
    const space = await home.space(login.id, world.owner.account, "team");

    // install the binders, whose sheets read through binders keeping no history
    const build = await TestDistribution.fixture("binder");
    await home.publish(TestDistribution.imageOf("binder"));
    const installing: unknown = await home
        .install(login.id, space, build, "binder")
        .catch((error: unknown) => error);

    // read the installation's Serving and its failure
    const served = await home.machine.space(space);
    const snapshot = served.database.snapshot();
    const installed = present(
        await snapshot
            .read(installation.table)
            .findFirst({ where: { scope: space, packageId: binderPackage.id } }),
        "the binders' installation",
    );
    const reason =
        "object 'sheet' records history but reads through 'binder', which has no history";
    const failure = `@destack/stack SERVING_BLOCKED: cannot serve revision '${await targeted(served.database, installed.id)}': ${reason}`;

    // refuse it alone, naming why, and keep the space up
    expect({
        installing,
        serving: [
            installed.conditions["Serving"]?.status,
            installed.conditions["Serving"]?.reason,
            installed.conditions["Serving"]?.message,
        ],
        synced: installed.conditions["Synced"]?.message,
        health: served.health.status,
        captured: world.telemetry.take().map((captured) => captured.message),
    }).toEqual({
        installing: new TypeError(`'binder' did not apply: '${failure}'`),
        serving: ["false", "Refused", reason],
        synced: failure,
        health: "up",
        captured: [
            `cannot serve revision '${await targeted(served.database, installed.id)}': ${reason}`,
        ],
    });
});

/** Read the revision an installation's rollout targets. */
async function targeted(database: DatabaseConnection, id: Identifier<"installation">) {
    const row = await database
        .snapshot()
        .read(installation.table)
        .findFirst({ where: { id: id } });

    return present(row, `the rollout of ${id}`).targetRevisionId;
}
