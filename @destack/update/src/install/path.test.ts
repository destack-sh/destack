import { join } from "node:path";
import { Simulation } from "@destack/environment/simulation";
import { Profile } from "../release/index.ts";
import { testSimulation } from "@destack/object/test";
import { expect, onTestFinished, test } from "@destack/test";
import { ShellPath } from "./path.ts";
import { testWorldOptions } from "@destack/test/world";

test("register the terminal command once in the user's shell file, keeping their configuration, and remove exactly it", async () => {
    // give a zsh user on macOS a home with a shell file of their own
    const { files } = testSimulation().environment("setup");
    const home = await files.makeTemporaryDirectory("destack-shell-");
    onTestFinished(() => files.remove(home, { recursive: true }));
    await using simulation = new Simulation({
        world: testWorldOptions(),
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
            script: "/usr/local/lib/destack/program/destack.js",
            runtime: { revision: "0".repeat(40), isStandalone: false },
        },
    });
    const host = simulation.environment("mac");
    const zshrc = join(home, ".zshrc");
    const original = "export EDITOR=vim\n";
    await files.writeTextFile(zshrc, original);

    // register the stable command twice, then remove it
    const profile = new Profile("release");
    const bin = join(home, ".destack/bin");
    await new ShellPath(bin, profile, host).add();
    await new ShellPath(bin, profile, host).add();
    const registered = await files.readTextFile(zshrc);
    await new ShellPath(bin, profile, host).remove();
    expect([registered, await files.readTextFile(zshrc)]).toEqual([
        `${original}\n# Destack\ncase ":$PATH:" in *":$HOME/.destack/bin:"*) ;; *) export PATH="$HOME/.destack/bin:$PATH" ;; esac\n`,
        original,
    ]);
});
