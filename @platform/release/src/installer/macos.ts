import { mkdir, mkdtemp, copyFile, symlink, rename, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { COMMANDS, type Installer, version } from "../distribution/index.ts";
import { MacSigning } from "../signing/apple.ts";
import { run } from "../distribution/command.ts";
import { selectedChannel } from "../distribution/identity.ts";

/** One architecture's signed application unpacked from its archive. */
interface Slice {
    /** The distribution target of the archive. */
    target: string;
    /** The Mach-O architecture name. */
    architecture: string;
    /** The unpacked application. */
    application: string;
}

/** Build a universal application in a drag-to-Applications disk image. */
export async function buildMacInstaller(installer: Installer): Promise<string> {
    if (process.platform !== "darwin") {
        throw new Error("build the macOS application on macOS");
    }
    const root = fileURLToPath(new URL("../../../../", import.meta.url));
    const output = join(root, "dist", version);
    const staging = await mkdtemp(join(tmpdir(), "destack-macos-"));
    const image = join(staging, "image");
    const title = selectedChannel().title;
    const application = join(image, `${title}.app`);
    const signing = new MacSigning();
    try {
        // start the universal application from the arm64 slice
        await mkdir(image);
        const slices = await unpackSlices(staging, output);
        const [primary] = slices;
        await run("ditto", [primary.application, application]);
        const contents = join(application, "Contents");

        // merge the executables and helpers of both slices
        await mergeDesktop(contents, slices);
        for (const name of [...COMMANDS, "destack-desktop-host"]) {
            await mergeCommand(name, contents, staging, slices);
        }
        await moveToolchains(contents, slices);
        await signing.sign(application);
        await signing.verify(application);
        await signing.notarize(application);
        const name = `destack-${version}-${installer.name}.${installer.format}`;

        return await createImage(title, name, image, staging, output, signing);
    } finally {
        await rm(staging, { recursive: true, force: true });
    }
}

/** Unpack the arm64 and x86_64 applications from their distribution archives. */
async function unpackSlices(staging: string, output: string): Promise<[Slice, Slice]> {
    // unpack both signed native applications without changing their compiled CLI payloads
    const slices: [Slice, Slice] = [
        {
            target: "aarch64-apple-darwin",
            architecture: "arm64",
            application: join(staging, "aarch64-apple-darwin", "Destack.app"),
        },
        {
            target: "x86_64-apple-darwin",
            architecture: "x86_64",
            application: join(staging, "x86_64-apple-darwin", "Destack.app"),
        },
    ];
    for (const { target } of slices) {
        await mkdir(join(staging, target));
        await run("tar", [
            "-xzf",
            join(output, `destack-${version}-${target}.tar.gz`),
            "-C",
            join(staging, target),
            "Destack.app",
        ]);
    }

    return slices;
}

/** Merge the native Tauri desktop executable of each slice into one universal executable. */
async function mergeDesktop(contents: string, slices: Slice[]): Promise<void> {
    // merge and verify both architectures
    const destination = join(contents, "MacOS", "Destack");
    await run("lipo", [
        "-create",
        ...slices.map((slice) => join(slice.application, "Contents/MacOS/Destack")),
        "-output",
        destination,
    ]);
    await run("lipo", [destination, "-verify_arch", ...slices.map((slice) => slice.architecture)]);
}

/** Dispatch the universal command to a complete architecture-specific compiled CLI. */
async function mergeCommand(
    name: string,
    contents: string,
    staging: string,
    slices: Slice[],
): Promise<void> {
    // copy each architecture's CLI and compile its dispatcher
    const commands = [];
    for (const { architecture, application: source } of slices) {
        const directory = join(contents, "Helpers", architecture);
        await mkdir(directory, { recursive: true });
        await copyFile(join(source, "Contents/Helpers", name), join(directory, name));
        commands.push(await compileDispatcher(name, architecture, staging));
    }

    // merge the dispatchers into the universal command
    const destination = join(contents, "Helpers", name);
    await run("lipo", ["-create", ...commands, "-output", destination]);
}

/** Compile the dispatcher of one command for one architecture and return its path. */
async function compileDispatcher(
    name: string,
    architecture: string,
    staging: string,
): Promise<string> {
    // build command.c with the command name defined
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

    return command;
}

/** Keep each architecture's compiler toolchain beside its compiler. */
async function moveToolchains(contents: string, slices: Slice[]): Promise<void> {
    // copy each slice's toolchain and remove the arm64 one at the shared path
    for (const { architecture, application: source } of slices) {
        await run("ditto", [
            join(source, "Contents/Helpers/toolchain"),
            join(contents, "Helpers", architecture, "toolchain"),
        ]);
    }
    await rm(join(contents, "Helpers/toolchain"), { recursive: true });
}

/** Create, verify and sign the disk image and move it into the output directory. */
async function createImage(
    title: string,
    name: string,
    image: string,
    staging: string,
    output: string,
    signing: MacSigning,
): Promise<string> {
    // let Finder provide the standard installation interaction
    await symlink("/Applications", join(image, "Applications"));
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

    // verify and sign the image before moving it into the output
    await run("hdiutil", ["verify", temporary]);
    await signing.image(temporary);
    await rename(temporary, join(output, name));

    return join(output, name);
}
