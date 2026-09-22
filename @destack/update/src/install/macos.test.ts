import { expect, test } from "@destack/test";
import { copyFile, mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
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
            await mkdir(join(application, "Contents/MacOS"), { recursive: true });
            await copyFile("/bin/echo", join(application, "Contents/MacOS/Destack"));
            await writeFile(
                join(application, "Contents/Info.plist"),
                `<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0"><dict>
<key>CFBundleIdentifier</key><string>sh.destack.desktop</string>
<key>CFBundleExecutable</key><string>Destack</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleVersion</key><string>2026.9.1</string>
</dict></plist>
`,
            );
            const signed = Bun.spawn(["/usr/bin/codesign", "--force", "--sign", "-", application], {
                stdout: "ignore",
                stderr: "pipe",
                timeout: 3000,
            });
            const [code, error] = await Promise.all([
                signed.exited,
                new Response(signed.stderr).text(),
            ]);
            if (code !== 0) {
                throw new Error(error);
            }

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
