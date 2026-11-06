import { CHECKOUT } from "../distribution/distribution.ts";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { COMMANDS, type Installer, releaseVersion } from "../distribution/index.ts";
import { MacSigning } from "../signing/apple.ts";
import { run } from "../distribution/command.ts";
import { selectedChannel } from "../distribution/identity.ts";
import type { Environment, HostCapability } from "@destack/environment";
import { Toolchain } from "@destack/check/toolchain";

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
export async function buildMacInstaller(
    installer: Installer,
    environment: Environment<HostCapability>,
): Promise<string> {
    // require macOS, and stage below a private temporary directory
    if (environment.host.os !== "darwin") {
        throw new Error("build the macOS application on macOS");
    }
    const { files } = environment;
    const version = await releaseVersion(CHECKOUT, environment);
    const output = join(CHECKOUT, "dist", version);
    const staging = await files.makeTemporaryDirectory("destack-macos-");
    const image = join(staging, "image");
    const title = selectedChannel(environment).title;
    const application = join(image, `${title}.app`);
    const signing = new MacSigning(environment);
    try {
        // start the universal application from the arm64 slice
        await files.mkdir(image);
        const slices = await unpackSlices(staging, output, version, environment);
        const [primary] = slices;
        await run("ditto", [primary.application, application], environment);
        const contents = join(application, "Contents");

        // merge the executables and helpers of both slices
        await mergeDesktop(contents, slices, environment);
        for (const name of [...COMMANDS, "destack-desktop-host"]) {
            await mergeCommand(name, contents, staging, slices, environment);
        }
        await mergeToolchains(contents, slices, environment);
        await signing.sign(application);
        await signing.verify(application);
        await signing.notarize(application);
        const name = `destack-${version}-${installer.name}.${installer.format}`;

        return await createImage(title, name, image, staging, output, signing);
    } finally {
        await files.remove(staging, { recursive: true, force: true });
    }
}

/** Unpack the arm64 and x86_64 applications from their distribution archives. */
async function unpackSlices(
    staging: string,
    output: string,
    version: string,
    environment: Environment<"files" | "process">,
): Promise<[Slice, Slice]> {
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
        await environment.files.mkdir(join(staging, target));
        await run(
            "tar",
            [
                "-xzf",
                join(output, `destack-${version}-${target}.tar.gz`),
                "-C",
                join(staging, target),
                "Destack.app",
            ],
            environment,
        );
    }

    return slices;
}

/** Merge the native Tauri desktop executable of each slice into one universal executable. */
async function mergeDesktop(
    contents: string,
    slices: Slice[],
    environment: Environment<"process">,
): Promise<void> {
    // merge and verify both architectures
    const destination = join(contents, "MacOS", "Destack");
    const executables = slices.map((slice) => join(slice.application, "Contents/MacOS/Destack"));
    const architectures = slices.map((slice) => slice.architecture);
    await run("lipo", ["-create", ...executables, "-output", destination], environment);
    await run("lipo", [destination, "-verify_arch", ...architectures], environment);
}

/** Dispatch the universal command to a complete architecture-specific compiled CLI. */
async function mergeCommand(
    name: string,
    contents: string,
    staging: string,
    slices: Slice[],
    environment: Environment<"files" | "process">,
): Promise<void> {
    // copy each architecture's CLI and compile its dispatcher
    const commands = [];
    for (const { architecture, application: source } of slices) {
        const directory = join(contents, "Helpers", architecture);
        await environment.files.mkdir(directory, { recursive: true });
        await environment.files.copyFile(
            join(source, "Contents/Helpers", name),
            join(directory, name),
        );
        commands.push(await compileDispatcher(name, architecture, staging, environment));
    }

    // merge the dispatchers into the universal command
    const destination = join(contents, "Helpers", name);
    await run("lipo", ["-create", ...commands, "-output", destination], environment);
}

/** Compile the dispatcher of one command for one architecture and return its path. */
async function compileDispatcher(
    name: string,
    architecture: string,
    staging: string,
    environment: Environment<"process">,
): Promise<string> {
    // build command.c with the command name defined
    const command = join(staging, `${name}-${architecture}`);
    await run(
        "xcrun",
        [
            "clang",
            "-Os",
            "-arch",
            architecture,
            "-mmacosx-version-min=11.0",
            `-DDESTACK_COMMAND="${name}"`,
            fileURLToPath(new URL("./command.c", import.meta.url)),
            "-o",
            command,
        ],
        environment,
    );

    return command;
}

/** Merge each architecture's toolchain into the application's, native packages side by side. */
async function mergeToolchains(
    contents: string,
    slices: Slice[],
    environment: Environment<"files">,
): Promise<void> {
    // copy each slice's files with their modes, refusing a shared file whose bytes differ between the slices
    const { files } = environment;
    const merged = Toolchain.of(join(contents, "Helpers/destack"));
    for (const { application } of slices) {
        const source = Toolchain.of(join(application, "Contents/Helpers/destack"));
        const entries = await files.readDirectory(source, { recursive: true });
        for (const entry of entries.filter((each) => each.kind === "file")) {
            const from = join(source, entry.name);
            const target = join(merged, entry.name);
            if (!(await files.exists(target))) {
                await files.mkdir(dirname(target), { recursive: true });
                await files.copyFile(from, target);
            } else {
                const [copied, kept] = [await files.readFile(from), await files.readFile(target)];
                if (!Buffer.from(copied).equals(Buffer.from(kept))) {
                    throw new Error(`the slices' toolchains differ at ${entry.name}`);
                }
            }
        }
    }
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
    const { environment } = signing;
    await environment.files.symlink("/Applications", join(image, "Applications"));
    const temporary = join(staging, name);
    await run(
        "hdiutil",
        ["create", "-volname", title, "-srcfolder", image, "-format", "UDZO", temporary],
        environment,
    );

    // verify and sign the image before moving it into the output
    await run("hdiutil", ["verify", temporary], environment);
    await signing.image(temporary);
    await environment.files.rename(temporary, join(output, name));

    return join(output, name);
}
