import { Derivation } from "@destack/identity";
import { testSimulation } from "../test/index.ts";
import { expect, test } from "@destack/test";
import { InstallationRoot } from "./root.ts";

test("derive each installation's keys from its space's identity, alike wherever the space is served", async () => {
    // derive two installations' roots from one space identity, the first twice as two hosts do
    const world = testSimulation().environment("test");
    const space = Derivation.of(
        await Derivation.root(world.random.bytes(32), world.crypto),
        world.crypto,
    );
    const keys = async (installation: string) =>
        InstallationRoot.open(await InstallationRoot.derive(space, installation), world.crypto);
    const [notes, notesElsewhere, pages] = await Promise.all([
        keys("installation-notes"),
        keys("installation-notes"),
        keys("installation-pages"),
    ]);

    // derive one installation's keys alike across hosts and apart from another installation's
    const [secret, again, other] = await Promise.all([
        notes.derive({ label: "message", context: "" }),
        notesElsewhere.derive({ label: "message", context: "" }),
        pages.derive({ label: "message", context: "" }),
    ]);
    expect([secret.toHex() === again.toHex(), secret.toHex() === other.toHex()]).toEqual([
        true,
        false,
    ]);
});
