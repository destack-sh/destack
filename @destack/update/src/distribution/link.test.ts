import { join } from "node:path";
import { Simulation } from "@destack/environment/simulation";
import { Release, Stage } from "../release/index.ts";
import { testSimulation } from "@destack/object/test";
import { expect, onTestFinished, test } from "@destack/test";
import { Distribution, type DistributionEnvironment } from "./distribution.ts";
import { testWorldOptions } from "@destack/test/world";

test("link and unlink independent Linux desktop entries and link handlers for production and local archives", async () => {
    // give a Linux person a home with an activated archive release
    const { files } = testSimulation().environment("setup");
    const directory = await files.makeTemporaryDirectory("destack-menu-");
    onTestFinished(() => files.remove(directory, { recursive: true, force: true }));
    const data = join(directory, "share");
    await using simulation = new Simulation({
        world: testWorldOptions(),
        seed: 1,
        start: 0,
        host: {
            os: "linux",
            architecture: "x64",
            version: "6.8",
            hostname: "linux",
            home: directory,
        },
        process: {
            arguments: [],
            variables: { XDG_DATA_HOME: data, SHELL: "/bin/bash" },
            workingDirectory: directory,
            id: 1,
            userId: 1000,
            executable: join(directory, "destack"),
            script: join(directory, "destack.js"),
            runtime: { revision: "0".repeat(40), isStandalone: false },
        },
    });
    const environment = simulation.environment("linux");
    const release = join(directory, "release");
    await files.mkdir(join(release, "Destack"), { recursive: true });
    await files.mkdir(join(release, "bin"), { recursive: true });
    await files.mkdir(join(release, "Destack", "helpers"));
    await files.writeTextFile(join(release, "Destack", "helpers", "destack"), "launcher");
    await files.symlink("../Destack/helpers/destack", join(release, "bin", "destack"));
    await files.writeTextFile(
        join(release, "Destack", "icon.svg"),
        '<svg xmlns="http://www.w3.org/2000/svg"/>',
    );

    // link the production and local stages, production twice, then unlink production twice
    const record = { method: "archive" as const, application: join(release, "Destack") };
    const production = distributionAt(join(directory, ".destack"), "production", environment);
    const local = distributionAt(join(directory, ".destack-local"), "local", environment);
    const machine = await production.link(record, { isShell: true });
    await local.link(record, { isShell: true });
    await production.link(record, { isShell: true });
    const entry = await files.readTextFile(join(data, "applications/app.destack.desktop.desktop"));
    const command = join(directory, ".destack", "bin", "destack");
    const linked = {
        target: await files.readLink(command),
        launcher: await files.readTextFile(command),
    };
    await production.unlink({ isShell: true });
    await production.unlink({ isShell: true });

    // keep local's entry and icon, answer the archive's machine, and reach the launcher through the command
    const names = async (path: string) =>
        (await files.readDirectory(join(data, path))).map((found) => found.name);
    expect({
        machine,
        linked,
        entry,
        applications: await names("applications"),
        icons: await names("icons/hicolor/scalable/apps"),
    }).toEqual({
        machine: join(release, "bin", "destack-machine"),
        linked: { target: join(release, "bin", "destack"), launcher: "launcher" },
        entry: `[Desktop Entry]\nType=Application\nName=Destack\nExec="${directory}/.destack/bin/destack" desktop %u\nIcon=app.destack.desktop\nTerminal=false\nCategories=Utility;Development;\nMimeType=x-scheme-handler/destack;\n`,
        applications: ["app.destack.desktop.local.desktop"],
        icons: ["app.destack.desktop.local.svg"],
    });
});

/** Describe a stage's Linux distribution in a home. */
function distributionAt(
    home: string,
    stage: "production" | "local",
    environment: DistributionEnvironment,
): Distribution {
    return new Distribution({
        home,
        repository: new URL("https://download.destack.test/"),
        root: Distribution.root(),
        running: new Release("2026.9.0", "x86_64-unknown-linux-gnu"),
        executable: join(home, "bin", "destack"),
        applications: join(home, "Applications"),
        stage: new Stage(stage),
        environment,
    });
}
