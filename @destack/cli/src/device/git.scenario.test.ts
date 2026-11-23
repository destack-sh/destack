import { join } from "node:path";
import { present } from "@destack/schema";
import { expect, onTestFinished, test } from "@destack/test";
import { testHost } from "@destack/test/host";
import { TerminalFixture } from "../test/terminal.ts";

test("register this program as git's credential helper at the universe's origin, which serves every repository", async () => {
    // set git up in a terminal whose global git configuration is a file of its own
    const { files, process: host } = testHost();
    const directory = await files.makeTemporaryDirectory("destack-git-");
    onTestFinished(() => files.remove(directory, { recursive: true }));
    const configuration = join(directory, "gitconfig");
    await files.writeTextFile(configuration, "");
    const terminal = await TerminalFixture.open({
        NO_COLOR: "1",
        GIT_CONFIG_GLOBAL: configuration,
        PATH: present(host.variables["PATH"], "the host's PATH"),
    });
    const set = await terminal.printed(["git", "setup"]);

    // read every key and value git setup wrote
    const listed = await host.run(["git", "config", "--file", configuration, "--list"]);

    // expect the universe's origin alone, its other helpers reset, then this program from its directory
    expect({ set, listed: new TextDecoder().decode(listed.stdout) }).toEqual({
        set: {
            code: 0,
            output: "git now pushes and pulls Destack repositories as you\n",
            error: "",
        },
        listed: [
            "credential.https://universe.test.helper=",
            "credential.https://universe.test.helper=!'destack' '--cwd' '/' 'destack.js' git credential",
            "",
        ].join("\n"),
    });
});
