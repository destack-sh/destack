import { join } from "node:path";
import { expect, onTestFinished, test } from "@destack/test";
import { testHost } from "@destack/test/host";
import { TerminalFixture } from "../test/terminal.ts";

test("register this program as git's credential helper at every machine repositories answer at", async () => {
    // set git up in a terminal whose global git configuration is a file of its own
    const { files } = testHost();
    const directory = await files.makeTemporaryDirectory("destack-git-");
    onTestFinished(() => files.remove(directory, { recursive: true }));
    const configuration = join(directory, "gitconfig");
    await files.writeTextFile(configuration, "");
    const terminal = await TerminalFixture.open({
        NO_COLOR: "1",
        GIT_CONFIG_GLOBAL: configuration,
    });
    const set = await terminal.printed(["git", "setup"]);

    // read each origin's helpers and whether git sends it the path
    const { process } = testHost();
    const listed = await process.run(["git", "config", "--file", configuration, "--list"]);
    const entries = new TextDecoder()
        .decode(listed.stdout)
        .trim()
        .split("\n")
        .map((line) => line.split("=", 1)[0] ?? line);
    const machines = terminal.computer.world.universe.control.definition.platform.machines;

    // expect each residency's machine and this computer's, each with the helper alone and the path
    const origins = [
        ...new Set([
            ...machines.map((machine) => new URL(machine.origin).origin),
            "http://127.0.0.1:7470",
        ]),
    ];
    expect({ set, entries: entries.toSorted() }).toEqual({
        set: {
            code: 0,
            output: "git now pushes and pulls Destack repositories as you\n",
            error: "",
        },
        entries: origins
            .flatMap((origin) => [
                `credential.${origin}.helper`,
                `credential.${origin}.helper`,
                `credential.${origin}.usehttppath`,
            ])
            .toSorted(),
    });
});
