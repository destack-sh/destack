import { expect, test } from "@destack/test";
import { Metadata, TargetFile, Targets } from "@tufjs/models";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { writeCatalog } from "./catalog.ts";

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
            unrecognizedFields: { custom: { version: "2026.9.1" } },
        }),
    );
    targets.signed.addTarget(
        new TargetFile({
            path: "universal-apple-darwin.dmg",
            length: 200,
            hashes: { sha256: installer },
            unrecognizedFields: { custom: { version: "2026.9.1" } },
        }),
    );
    try {
        // preserve both file identities without letting insertion order replace either format
        await writeCatalog(directory, targets, new URL("https://download.destack.sh/stable/"));
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
        const script = await readFile(join(directory, "install"), "utf8");
        expect(script).toContain(
            `https://download.destack.sh/stable/targets/${archive}.aarch64-apple-darwin.tar.gz`,
        );
        expect(script).not.toContain("# __DISTRIBUTIONS__");
    } finally {
        await rm(directory, { recursive: true, force: true });
    }
});
