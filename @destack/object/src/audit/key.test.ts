import { Derivation } from "@destack/identity";
import { testSimulation } from "../test/index.ts";
import { expect, test } from "@destack/test";
import { InstallationRoot } from "../workload/root.ts";
import { CallKey } from "./key.ts";

test("derive an installation's journal key alike on each of its hosts and apart from another installation's", async () => {
    // derive each installation's journal key from its root, twice for the first
    const world = testSimulation().environment("test");
    const space = Derivation.of(
        await Derivation.root(new Uint8Array(32).fill(7), world.crypto),
        world.crypto,
    );
    const fingerprint = async (installation: string) => {
        const keys = await InstallationRoot.open(
            await InstallationRoot.derive(space, installation),
            world.crypto,
        );
        const key = await CallKey.of(keys, world.crypto);
        const signed = await world.crypto.sign("HMAC", key, new TextEncoder().encode("input"));

        return new Uint8Array(signed).toHex();
    };
    const [first, again, other] = await Promise.all([
        fingerprint("installation-a"),
        fingerprint("installation-a"),
        fingerprint("installation-b"),
    ]);

    expect({ isRepeated: first === again, isApart: first !== other }).toEqual({
        isRepeated: true,
        isApart: true,
    });
});
