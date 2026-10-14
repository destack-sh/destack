import { Derivation } from "@destack/identity";
import { expect, test } from "@destack/test";
import { InstallationRoot } from "./root.ts";

test("derive each installation's keys from its space's identity, alike wherever the space is served", async () => {
    // derive two installations' roots from one space identity, the first twice as two hosts do
    const space = Derivation.of(await Derivation.root(crypto.getRandomValues(new Uint8Array(32))));
    const keys = async (installation: string) =>
        InstallationRoot.open(await InstallationRoot.derive(space, installation));
    const [notes, notesElsewhere, pages] = await Promise.all([
        keys("installation-notes"),
        keys("installation-notes"),
        keys("installation-pages"),
    ]);

    // keep one installation's keys alike across hosts and apart from another installation's
    const [secret, again, other] = await Promise.all([
        notes.derive("message"),
        notesElsewhere.derive("message"),
        pages.derive("message"),
    ]);
    expect([secret.toHex() === again.toHex(), secret.toHex() === other.toHex()]).toEqual([
        true,
        false,
    ]);
});
