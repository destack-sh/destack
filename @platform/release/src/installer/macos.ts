import { mkdir, mkdtemp, copyFile, symlink, rename, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { version } from "../distribution/index.ts";
import { MacSigning } from "../signing/apple.ts";
import { run } from "../distribution/command.ts";
import { ReleaseIdentity } from "../distribution/identity.ts";

/** Build a universal application in a drag-to-Applications disk image. */
export async function buildMacInstaller(): Promise<string> {
    if (process.platform !== "darwin") {
        throw new Error("build the macOS application on macOS");
    }
    const root = fileURLToPath(new URL("../../../../", import.meta.url));
    const output = join(root, "dist", version);
    const staging = await mkdtemp(join(tmpdir(), "destack-macos-"));
    const image = join(staging, "image");
    const title = new ReleaseIdentity(process.env.DESTACK_RELEASE_CHANNEL ?? "dev").title;
    const application = join(image, `${title}.app`);
    const signing = new MacSigning();
    try {
        await mkdir(image);

        // unpack both signed native applications without changing their compiled CLI payloads
        const targets = ["aarch64-apple-darwin", "x86_64-apple-darwin"];
        const architectures = ["arm64", "x86_64"];
        for (const target of targets) {
            await mkdir(join(staging, target));
            await run("tar", [
                "-xzf",
                join(output, `destack-${version}-${target}.tar.gz`),
                "-C",
                join(staging, target),
                "Destack.app",
            ]);
        }
        const applications = targets.map((target) => join(staging, target, "Destack.app"));
        await run("ditto", [applications[0], application]);
        const contents = join(application, "Contents");

        // merge the native Tauri desktop executable
        {
            const destination = join(contents, "MacOS", "Destack");
            await run("lipo", [
                "-create",
                ...applications.map((path) => join(path, "Contents/MacOS/Destack")),
                "-output",
                destination,
            ]);
            await run("lipo", [destination, "-verify_arch", ...architectures]);
        }

        // dispatch the universal command to a complete architecture-specific compiled CLI
        for (const name of [
            "destack",
            "destack-daemon",
            "destack-sandbox",
            "destack-desktop-host",
        ]) {
            const commands = [];
            for (const [index, architecture] of architectures.entries()) {
                const directory = join(contents, "Helpers", architecture);
                await mkdir(directory, { recursive: true });
                await copyFile(
                    join(applications[index], "Contents/Helpers", name),
                    join(directory, name),
                );
                const command = join(staging, `${name}-${architecture}`);
                await run("xcrun", [
                    "clang",
                    "-Os",
                    "-arch",
                    architecture,
                    "-mmacosx-version-min=11.0",
                    `-DDESTACK_COMMAND="${name}"`,
                    fileURLToPath(new URL("command.c", import.meta.url)),
                    "-o",
                    command,
                ]);
                commands.push(command);
            }
            const destination = join(contents, "Helpers", name);
            await run("lipo", ["-create", ...commands, "-output", destination]);
        }
        await signing.application(application);
        await signing.notarizeApplication(application);

        // let Finder provide the standard installation interaction
        await symlink("/Applications", join(image, "Applications"));
        const name = `destack-${version}-universal-apple-darwin.dmg`;
        const temporary = join(staging, name);
        await run("hdiutil", [
            "create",
            "-volname",
            title,
            "-srcfolder",
            image,
            "-format",
            "UDZO",
            temporary,
        ]);
        await run("hdiutil", ["verify", temporary]);
        await signing.image(temporary);
        await rename(temporary, join(output, name));

        return join(output, name);
    } finally {
        await rm(staging, { recursive: true, force: true });
    }
}
