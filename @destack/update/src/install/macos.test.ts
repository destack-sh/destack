import { expect, test } from "@destack/test";
import { copyFile, mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { installApplication } from "./macos.ts";
import { Installer } from "./installer.ts";
import { Release } from "../release/release.ts";

test.runIf(process.platform === "darwin")(
    "recover macOS activation after bundle replacement",
    async () => {
        // construct an ad-hoc signed application entirely inside the temporary installation
        const directory = await mkdtemp(join(tmpdir(), "destack-macos-update-"));
        try {
            const installer = new Installer(join(directory, "installation"));
            const release = new Release("2026.9.1", Release.target());
            const staged = join(installer.directory, "versions", release.directory);
            const application = join(staged, "Destack.app");
            await writeApplication(application, release);

            // force staging cleanup to fail after the native bundle is replaced
            const sha256 = "a".repeat(64);
            await writeFile(join(staged, "receipt.json"), JSON.stringify({ sha256 }));
            const destination = join(directory, "Applications/Destack.app");
            await writeFile(
                join(installer.directory, "activate.json"),
                JSON.stringify({
                    version: release.version,
                    target: release.target,
                    sha256,
                    application: destination,
                }),
            );
            await mkdir(join(installer.directory, "staged.json"));
            await expect(installer.recover()).rejects.toMatchObject({ code: "ERR_FS_EISDIR" });
            expect(await readFile(join(destination, "Contents/Info.plist"), "utf8")).toBe(
                await readFile(join(application, "Contents/Info.plist"), "utf8"),
            );

            // replay the persisted activation through a real atomic bundle exchange
            await rm(join(installer.directory, "staged.json"), { recursive: true });
            await installer.recover();
            expect(await installer.current()).toEqual({ release, sha256, directory: staged });
            await expect(
                readFile(join(installer.directory, "activate.json")),
            ).rejects.toMatchObject({ code: "ENOENT" });
        } finally {
            await rm(directory, { recursive: true, force: true });
        }
    },
);

test.skipIf(process.platform !== "darwin")(
    "replace signed applications while keeping stable and nightly identities separate",
    async () => {
        // create genuine ad-hoc signed bundles without modifying installed applications
        const directory = await mkdtemp(join(tmpdir(), "destack-bundle-"));
        try {
            const source = join(directory, "source.app");
            const destination = join(directory, "renamed.app");
            for (const version of ["2026.9.1-nightly.1", "2026.9.1-nightly.2"]) {
                const release = new Release(version, Release.target());
                await writeApplication(source, release);
                await installApplication(source, destination, release.applicationIdentifier);
                expect(
                    await readFile(join(destination, "Contents/Resources/version"), "utf8"),
                ).toBe(version);
            }

            // reject another distribution identity without changing the installed bundle
            const stable = new Release("2026.9.2", Release.target());
            await writeApplication(source, stable);
            await expect(
                installApplication(source, destination, stable.applicationIdentifier),
            ).rejects.toThrow(
                `application identifier does not match sh.destack.desktop: ${destination}`,
            );
            expect(await readFile(join(destination, "Contents/Resources/version"), "utf8")).toBe(
                "2026.9.1-nightly.2",
            );
        } finally {
            await rm(directory, { recursive: true, force: true });
        }
    },
);

/** Write and sign a real bundle for installation and recovery scenarios. */
async function writeApplication(application: string, release: Release): Promise<void> {
    // write executable and versioned bundle contents before signing
    await mkdir(join(application, "Contents/MacOS"), { recursive: true });
    await mkdir(join(application, "Contents/Resources"), { recursive: true });
    await copyFile("/usr/bin/true", join(application, "Contents/MacOS/application"));
    await writeFile(join(application, "Contents/Resources/version"), release.version);
    await writeFile(
        join(application, "Contents/Info.plist"),
        `<?xml version="1.0"?><plist version="1.0"><dict>
<key>CFBundleIdentifier</key><string>${release.applicationIdentifier}</string>
<key>CFBundleExecutable</key><string>application</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleVersion</key><string>${release.version}</string>
</dict></plist>`,
    );

    // require a valid signature before exercising the installer
    const signing = Bun.spawn(["/usr/bin/codesign", "--force", "--sign", "-", application], {
        stdout: "ignore",
        stderr: "pipe",
        timeout: 3000,
    });
    const diagnostic = await new Response(signing.stderr).text();
    expect(await signing.exited, diagnostic).toBe(0);
}
