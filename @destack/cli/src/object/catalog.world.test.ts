import { expect, test } from "@destack/test";
import { TerminalFixture } from "../test/terminal.ts";
import { Catalog } from "./catalog.ts";

test("serve each noun at the innermost store serving its type", async () => {
    // describe the stores of the device, the machine, the universe and the home
    const terminal = await TerminalFixture.open();
    const catalog = await terminal.wait(Catalog.of(terminal.context("home")));
    const sourceOf = (word: string) => {
        const found = catalog.find(word);

        return found === undefined ? undefined : `${found.store}/${found.qualifier}`;
    };

    // take the account's types from the universe, the space's from its base packages, the device's and machine's from loopback
    expect(
        ["spaces", "users", "installations", "rollouts", "settings", "computers", "checkouts"].map(
            sourceOf,
        ),
    ).toEqual([
        "universe/account",
        "universe/account",
        "space/object",
        "space/stack",
        "space/setting",
        "device/machine",
        "machine/machine",
    ]);
});
