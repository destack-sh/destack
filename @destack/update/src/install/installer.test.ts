import { testSimulation } from "@destack/object/test";
import { expect, test, refusal } from "@destack/test";
import { join } from "node:path";
import type { Environment } from "@destack/environment";
import { Installer } from "./installer.ts";
import { Manifest } from "../manifest/manifest.ts";
import type { Download } from "../repository/repository.ts";
import { Release } from "../release/release.ts";
import { Digest } from "@destack/schema";

test("recover interrupted activation and retain both distributions", async () => {
    const world = testSimulation().environment("test");
    const directory = await world.files.makeTemporaryDirectory("destack-activation-");
    try {
        // lay out two distributions
        const source = join(directory, "source");
        await world.files.mkdir(source);
        const installer = new Installer(join(directory, "installation"), world);
        const installed = [];
        for (const version of ["2026.9.0", "2026.9.1"]) {
            const files = join(source, version);
            await world.files.mkdir(files);
            await world.files.writeTextFile(join(files, "version"), version);
            const release = new Release(version, "x86_64-unknown-linux-gnu");
            installed.push(
                await installer.stage(await downloaded(release, files, world), async (path) => {
                    expect(await world.files.readTextFile(join(path, "version"))).toBe(version);
                }),
            );
        }

        // interrupt the second activation after persisting its intent
        const [first, second] = installed;
        if (first === undefined || second === undefined) {
            throw new Error("expected two staged distributions");
        }
        await installer.activate(first);
        await world.files.writeTextFile(
            join(installer.directory, "activate.json"),
            JSON.stringify({
                version: second.release.version,
                target: second.release.target,
                sha256: second.sha256,
            }),
        );
        expect(await installer.current()).toEqual(first);
        await installer.recover();

        // require a complete selection and preserve the previous distribution
        expect(await installer.current()).toEqual(second);
        expect(
            (await world.files.readDirectory(join(installer.directory, "versions"))).map(
                (entry) => entry.name,
            ),
        ).toEqual([first.release.directory, second.release.directory]);
        expect(await world.files.exists(join(installer.directory, "activate.json"))).toBe(false);
        await installer.recover();
        expect(await installer.current()).toEqual(second);

        // reject an obsolete journal without changing the selected release
        await world.files.writeTextFile(
            join(installer.directory, "activate.json"),
            JSON.stringify({
                version: first.release.version,
                target: first.release.target,
                sha256: first.sha256,
            }),
        );
        expect(await refusal(installer.recover())).toEqual([
            "PRECONDITION_FAILED",
            "RELEASE_BLOCKED",
            "cannot activate the release: it is older or incompatible",
        ]);
        expect(await installer.current()).toEqual(second);
    } finally {
        await world.files.remove(directory, { recursive: true });
    }
});

test("refuse a Mac activation without its application's absolute destination and identifier", async () => {
    // stage a Mac release in a world on a Mac
    const world = testSimulation().environment("test");
    const directory = await world.files.makeTemporaryDirectory("destack-mac-activation-");
    try {
        const source = join(directory, "source");
        await world.files.mkdir(join(source, "Destack.app"), { recursive: true });
        await world.files.writeTextFile(join(source, "Destack.app", "Info.plist"), "plist");
        const installer = new Installer(join(directory, "installation"), world);
        const release = new Release("2026.9.1", "aarch64-apple-darwin");
        const download = await downloaded(release, source, world);
        const { sha256 } = download;
        const staged = await installer.stage(download, async () => {});

        // refuse activating it without a destination or an identifier
        const refused = [
            await refusal(installer.activate(staged)),
            await refusal(installer.activate(staged, "/Applications/Destack.app")),
        ];

        // refuse recovering a recorded activation with a relative destination
        const record = { version: release.version, target: release.target, sha256 };
        await world.files.writeTextFile(
            join(installer.directory, "activate.json"),
            JSON.stringify({ ...record, application: "Destack.app", applicationIdentifier: "app" }),
        );
        refused.push(await refusal(installer.recover()));
        expect(refused).toEqual([
            [
                "BAD_REQUEST",
                "OPTIONS_INVALID",
                "macOS release option set is invalid: it must have an application destination and identifier",
            ],
            [
                "BAD_REQUEST",
                "OPTIONS_INVALID",
                "macOS release option set is invalid: it must have an application destination and identifier",
            ],
            [
                "INTERNAL_SERVER_ERROR",
                "RECORD_BROKEN",
                "missing absolute application destination or identifier",
            ],
        ]);
    } finally {
        await world.files.remove(directory, { recursive: true });
    }
});

/** Describe a directory's files as a release's download, named by its manifest's digest. */
async function downloaded(
    release: Release,
    directory: string,
    environment: Environment<"files">,
): Promise<Download> {
    const manifest = await Manifest.read(directory, environment);
    const sha256 = await Digest.of(manifest.encode(), environment.crypto);

    return { release, manifest, blobs: new URL("http://127.0.0.1/blobs/"), directory, sha256 };
}
