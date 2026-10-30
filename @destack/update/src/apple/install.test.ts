import { testHost } from "@destack/test/host";
import { expect, test, refusal } from "@destack/test";
import { join } from "node:path";
import { installApplication } from "./install.ts";
import { Installer } from "../install/installer.ts";
import { Release } from "../release/release.ts";

/** The Mac these tests sign, copy and exchange real application bundles on. */
const HOST = testHost();

/** The longest an ad-hoc signature may take, in milliseconds. */
const SIGN_MILLISECONDS = 3000;

/** The release target of the host. */
const TARGET = Release.target(HOST.host.os, HOST.host.architecture);

test("recover macOS activation after bundle replacement", async () => {
    // construct an ad-hoc signed application entirely inside the temporary installation
    const world = requireMac();
    const directory = await world.files.makeTemporaryDirectory("destack-macos-update-");
    try {
        const installer = new Installer(join(directory, "installation"), world);
        const release = new Release("2026.9.1", TARGET);
        const staged = join(installer.directory, "versions", release.directory);
        const application = join(staged, "Destack.app");
        await writeApplication(application, release, "org.example.editor");

        // force staging cleanup to fail after the native bundle is replaced
        const sha256 = "a".repeat(64);
        await HOST.files.writeTextFile(join(staged, "receipt.json"), JSON.stringify({ sha256 }));
        const destination = join(directory, "Applications/Destack.app");
        await HOST.files.writeTextFile(
            join(installer.directory, "activate.json"),
            JSON.stringify({
                version: release.version,
                target: release.target,
                sha256,
                application: destination,
                applicationIdentifier: "org.example.editor",
            }),
        );
        const staging = join(installer.directory, "staged.json");
        await HOST.files.mkdir(staging);
        await HOST.files.writeTextFile(join(staging, "blocking"), "");
        expect(await refusal(installer.recover())).toEqual([
            "CONFLICT",
            "ALREADY_EXISTS",
            `cannot remove ${staging}: ENOTEMPTY`,
        ]);
        expect(await HOST.files.readTextFile(join(destination, "Contents/Info.plist"))).toBe(
            await HOST.files.readTextFile(join(application, "Contents/Info.plist")),
        );

        // replay the persisted activation through a real atomic bundle exchange
        await HOST.files.remove(join(installer.directory, "staged.json"), { recursive: true });
        await installer.recover();
        expect(await installer.current()).toEqual({ release, sha256, directory: staged });
        expect(await HOST.files.exists(join(installer.directory, "activate.json"))).toBe(false);
    } finally {
        await world.files.remove(directory, { recursive: true });
    }
});

test("replace signed applications while keeping stable and dev identities separate", async () => {
    // create genuine ad-hoc signed bundles without modifying installed applications
    const world = requireMac();
    const directory = await world.files.makeTemporaryDirectory("destack-bundle-");
    try {
        const source = join(directory, "source.app");
        const destination = join(directory, "renamed.app");
        for (const version of ["2026.9.1-dev.1", "2026.9.1-dev.2"]) {
            const release = new Release(version, TARGET);
            await writeApplication(source, release, "org.example.editor.dev");
            await installApplication(source, destination, "org.example.editor.dev", world);
            expect(
                await HOST.files.readTextFile(join(destination, "Contents/Resources/version")),
            ).toBe(version);
        }

        // reject another distribution identity without changing the installed bundle
        const stable = new Release("2026.9.2", TARGET);
        await writeApplication(source, stable, "org.example.editor");
        expect(
            await refusal(installApplication(source, destination, "org.example.editor", world)),
        ).toEqual([
            "BAD_GATEWAY",
            "VERIFICATION_FAILED",
            `application identifier does not match org.example.editor: ${destination}`,
        ]);
        expect(await HOST.files.readTextFile(join(destination, "Contents/Resources/version"))).toBe(
            "2026.9.1-dev.2",
        );
    } finally {
        await world.files.remove(directory, { recursive: true });
    }
});

/** Write and sign a real bundle for installation and recovery scenarios. */
async function writeApplication(
    application: string,
    release: Release,
    identifier: string,
): Promise<void> {
    // write executable and versioned bundle contents before signing
    await HOST.files.mkdir(join(application, "Contents/MacOS"), { recursive: true });
    await HOST.files.mkdir(join(application, "Contents/Resources"), { recursive: true });
    await HOST.files.copyFile("/usr/bin/true", join(application, "Contents/MacOS/application"));
    await HOST.files.writeTextFile(
        join(application, "Contents/Resources/version"),
        release.version,
    );
    await HOST.files.writeTextFile(
        join(application, "Contents/Info.plist"),
        `<?xml version="1.0"?><plist version="1.0"><dict>
<key>CFBundleIdentifier</key><string>${identifier}</string>
<key>CFBundleExecutable</key><string>application</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleVersion</key><string>${release.version}</string>
</dict></plist>`,
    );

    // require a valid signature before exercising the installer
    const signing = HOST.process.spawn(
        ["/usr/bin/codesign", "--force", "--sign", "-", application],
        {
            stdout: "ignore",
            stderr: "pipe",
            signal: HOST.clock.timeout(SIGN_MILLISECONDS),
        },
    );
    const diagnostic = await new Response(signing.stderr).text();
    expect(await signing.exited, diagnostic).toBe(0);
}

/** Require the host to be a Mac, failing loudly elsewhere, and answer it. */
function requireMac(): typeof HOST {
    if (HOST.host.os !== "darwin") {
        throw new Error(
            "the macOS installer tests run on a Mac: run the integration project there",
        );
    }

    return HOST;
}
