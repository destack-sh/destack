import { join } from "node:path";
import { Simulation } from "@destack/environment/simulation";
import { ReleaseChannel } from "../release/index.ts";
import { testSimulation } from "@destack/object/test";
import { expect, onTestFinished, test } from "@destack/test";
import { ShellPath } from "./path.ts";

test("register the terminal command once in the user's shell file, keeping their configuration, and remove exactly it", async () => {
    // give a zsh user on macOS a home with a shell file of their own
    const { files } = testSimulation().environment("setup");
    const home = await files.makeTemporaryDirectory("destack-shell-");
    onTestFinished(() => files.remove(home, { recursive: true }));
    const simulation = new Simulation({
        seed: 1,
        start: 0,
        host: { os: "darwin", architecture: "arm64", version: "24.1.0", hostname: "mac", home },
        process: {
            arguments: [],
            variables: { SHELL: "/bin/zsh" },
            workingDirectory: home,
            id: 1,
            userId: 1000,
            executable: "/usr/local/bin/destack",
            runtime: { revision: "0".repeat(40), isStandalone: false },
        },
    });
    const host = simulation.environment("mac");
    const profile = join(home, ".zshrc");
    const original = "export EDITOR=vim\n";
    await files.writeTextFile(profile, original);

    // register the stable command twice, then remove it
    const channel = new ReleaseChannel("stable");
    const bin = join(home, ".destack/bin");
    await new ShellPath(bin, channel, host).add();
    await new ShellPath(bin, channel, host).add();
    const registered = await files.readTextFile(profile);
    await new ShellPath(bin, channel, host).remove();
    expect([registered, await files.readTextFile(profile)]).toEqual([
        `${original}\n# Destack\ncase ":$PATH:" in *":$HOME/.destack/bin:"*) ;; *) export PATH="$HOME/.destack/bin:$PATH" ;; esac\n`,
        original,
    ]);
});
