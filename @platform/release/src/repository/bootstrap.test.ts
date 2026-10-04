import { expect, test } from "@destack/test";
import { Metadata, TargetFile, Targets } from "@tufjs/models";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { writeBootstrap } from "./bootstrap.ts";
import { COMMIT } from "@destack/update/test";

test("publish platform archives and the universal macOS installer in the download catalog", async () => {
    // describe the same macOS release in its two independently authenticated formats
    const directory = await mkdtemp(join(tmpdir(), "destack-catalog-"));
    const targets = new Metadata(
        new Targets({ version: 1, specVersion: "1.0.31", expires: "2099-01-01T00:00:00Z" }),
    );
    const archive = "a".repeat(64);
    const installer = "b".repeat(64);
    targets.signed.addTarget(
        new TargetFile({
            path: "aarch64-apple-darwin.tar.gz",
            length: 100,
            hashes: { sha256: archive },
            unrecognizedFields: { custom: { version: "2026.9.1", commit: COMMIT } },
        }),
    );
    targets.signed.addTarget(
        new TargetFile({
            path: "universal-apple-darwin.dmg",
            length: 200,
            hashes: { sha256: installer },
            unrecognizedFields: { custom: { version: "2026.9.1", commit: COMMIT } },
        }),
    );
    try {
        // preserve both file identities without letting insertion order replace either format
        await writeBootstrap(directory, targets, new URL("https://download.destack.sh/stable/"));
        expect(JSON.parse(await readFile(join(directory, "downloads.json"), "utf8"))).toEqual({
            version: "2026.9.1",
            channel: "stable",
            downloads: {
                "aarch64-apple-darwin": {
                    archive: {
                        version: "2026.9.1",
                        sha256: archive,
                        size: 100,
                        url: `https://download.destack.sh/stable/targets/${archive}.aarch64-apple-darwin.tar.gz`,
                    },
                    installers: {},
                },
                "universal-apple-darwin": {
                    installers: {
                        dmg: {
                            version: "2026.9.1",
                            sha256: installer,
                            size: 200,
                            url: `https://download.destack.sh/stable/targets/${installer}.universal-apple-darwin.dmg`,
                        },
                    },
                },
            },
        });
        // write the archive's case into the installer in place of its marker
        const script = await readFile(join(directory, "install"), "utf8");
        const shell = await readFile(new URL("../installer/install.sh", import.meta.url), "utf8");
        const url = `https://download.destack.sh/stable/targets/${archive}.aarch64-apple-darwin.tar.gz`;
        expect(script).toEqual(
            shell.replace(
                "# __DISTRIBUTIONS__",
                `    aarch64-apple-darwin/tar.gz) url='${url}'; sha256='${archive}' ;;`,
            ),
        );
    } finally {
        await rm(directory, { recursive: true, force: true });
    }
});
