import { join } from "node:path";
import { Simulation } from "@destack/environment/simulation";
import { Release, ReleaseChannel } from "../release/index.ts";
import { testSimulation } from "@destack/object/test";
import { expect, onTestFinished, test } from "@destack/test";
import { Distribution, type DistributionEnvironment } from "./distribution.ts";

test("link and unlink independent Linux desktop entries and link handlers for stable and dev archives", async () => {
    // give a Linux person a home with an activated archive release
    const { files } = testSimulation().environment("setup");
    const directory = await files.makeTemporaryDirectory("destack-menu-");
    onTestFinished(() => files.remove(directory, { recursive: true, force: true }));
    const data = join(directory, "share");
    const simulation = new Simulation({
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
            runtime: { revision: "0".repeat(40), isStandalone: false },
        },
    });
    const environment = simulation.environment("linux");
    const release = join(directory, "release");
    await files.mkdir(join(release, "Destack"), { recursive: true });
    await files.mkdir(join(release, "bin"), { recursive: true });
    await files.writeTextFile(join(release, "bin", "destack"), "#!/bin/sh\n");
    await files.writeTextFile(
        join(release, "Destack", "icon.svg"),
        '<svg xmlns="http://www.w3.org/2000/svg"/>',
    );

    // link the stable and dev channels, stable twice, then unlink stable twice
    const record = { method: "archive" as const, application: join(release, "Destack") };
    const stable = distributionAt(join(directory, ".destack"), "stable", environment);
    const dev = distributionAt(join(directory, ".destack-dev"), "dev", environment);
    const machine = await stable.link(record, { isShell: true });
    await dev.link(record, { isShell: true });
    await stable.link(record, { isShell: true });
    const entry = await files.readTextFile(join(data, "applications/app.destack.desktop.desktop"));
    await stable.unlink({ isShell: true });
    await stable.unlink({ isShell: true });

    // keep dev's entry and icon, and answer the archive's machine
    const names = async (path: string) =>
        (await files.readDirectory(join(data, path))).map((found) => found.name);
    expect({
        machine,
        entry,
        applications: await names("applications"),
        icons: await names("icons/hicolor/scalable/apps"),
    }).toEqual({
        machine: join(release, "bin", "destack-machine"),
        entry: `[Desktop Entry]\nType=Application\nName=Destack\nExec="${directory}/.destack/bin/destack" desktop %u\nIcon=app.destack.desktop\nTerminal=false\nCategories=Utility;Development;\nMimeType=x-scheme-handler/destack;\n`,
        applications: ["app.destack.desktop.dev.desktop"],
        icons: ["app.destack.desktop.dev.svg"],
    });
});

/** Describe a channel's Linux distribution in a home. */
function distributionAt(
    home: string,
    channel: "stable" | "dev",
    environment: DistributionEnvironment,
): Distribution {
    return new Distribution({
        home,
        repository: new URL("https://download.destack.test/stable/"),
        root: Distribution.root(),
        running: new Release("2026.9.0", "x86_64-unknown-linux-gnu"),
        executable: join(home, "bin", "destack"),
        applications: join(home, "Applications"),
        channel: new ReleaseChannel(channel),
        environment,
    });
}
