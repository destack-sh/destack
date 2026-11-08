import { expect, test } from "@destack/test";
import { testHost } from "@destack/test/host";
import { Metadata, TargetFile, Targets } from "@tufjs/models";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { writeBootstrap } from "./bootstrap.ts";
import { COMMIT } from "@destack/update/test";

test("publish platform archives and the universal macOS installer in the download catalog", async () => {
    // describe the same macOS release in its two independently authenticated formats
    const { files } = testHost();
    const directory = await files.makeTemporaryDirectory("destack-catalog-");
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
        const feed = new URL("https://download.destack.sh/");
        await writeBootstrap(directory, targets, feed, files);
        expect(JSON.parse(await files.readTextFile(join(directory, "downloads.json")))).toEqual({
            version: "2026.9.1",
            downloads: {
                "aarch64-apple-darwin": {
                    archive: {
                        version: "2026.9.1",
                        sha256: archive,
                        size: 100,
                        url: `https://download.destack.sh/targets/${archive}.aarch64-apple-darwin.tar.gz`,
                    },
                    installers: {},
                },
                "universal-apple-darwin": {
                    installers: {
                        dmg: {
                            version: "2026.9.1",
                            sha256: installer,
                            size: 200,
                            url: `https://download.destack.sh/targets/${installer}.universal-apple-darwin.dmg`,
                        },
                    },
                },
            },
        });
        // write the archive's case into the installer in place of its marker
        const script = await files.readTextFile(join(directory, "install"));
        const shell = await files.readTextFile(
            fileURLToPath(new URL("../installer/install.sh", import.meta.url)),
        );
        const url = `https://download.destack.sh/targets/${archive}.aarch64-apple-darwin.tar.gz`;
        expect(script).toEqual(
            shell.replace(
                "# __DISTRIBUTIONS__",
                `    aarch64-apple-darwin/tar.gz) url='${url}'; sha256='${archive}' ;;`,
            ),
        );
    } finally {
        await files.remove(directory, { recursive: true, force: true });
    }
});
