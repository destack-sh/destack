import { ReleaseChannel } from "@destack/daemon/process";
import { cp, mkdir, mkdtemp, rename, rm, writeFile } from "node:fs/promises";
import { basename, dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { Release } from "@destack/update/release";
import workspace from "../../../../package.json" with { type: "json" };
import solidPlugin from "@opentui/solid/bun-plugin";
import {
    buildExecutable,
    buildToolchain,
    COMMANDS,
    COMPILER,
    type ExecutableOptions,
    type Platform,
    selectPlatform,
    version,
} from "./index.ts";
import { run } from "./command.ts";
import { configureApplication } from "./application.ts";
import { selectedChannel } from "./identity.ts";
import { createSigner, signDistribution } from "../signing/signer.ts";
import { print } from "../output/index.ts";

/** Repository containing all distribution entrypoints. */
const ROOT = fileURLToPath(new URL("../../../../", import.meta.url));

/** The executables each distribution compiles into `bin`, with the options each needs. */
const EXECUTABLES: readonly {
    readonly name: string;
    readonly entrypoint: string;
    readonly options: Pick<ExecutableOptions, "external" | "isLoadingSources" | "plugins">;
}[] = [
    {
        name: "destack",
        entrypoint: "@destack/cli/src/main.ts",
        options: { plugins: [solidPlugin] },
    },
    { name: "destack-daemon", entrypoint: "@destack/daemon/src/main.ts", options: {} },
    { name: "destack-sandbox", entrypoint: "@destack/sandbox/src/main.ts", options: {} },
    { name: "destack-build", entrypoint: "@destack/build/src/main.ts", options: COMPILER },
    { name: "destack-desktop-host", entrypoint: "@destack/desktop/src/main.ts", options: {} },
];

/** The Linux script that installs the extracted application's commands and desktop entry. */
const LINUX_INSTALLER =
    '#!/bin/sh\nset -eu\napplication=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)\n"$application/helpers/destack" self install\n"$application/helpers/destack" desktop\n';

/** Build the native window, TypeScript host, and CLI as one distribution. */
async function build(): Promise<void> {
    // compile distributions with the pinned runtime
    if (`bun@${Bun.version}` !== workspace.packageManager) {
        throw new Error(`release builds require ${workspace.packageManager}`);
    }

    // build each native desktop on its matching operating system
    const release = new Release(
        version,
        process.argv[2] ?? Release.target(process.platform, process.arch),
    );
    const target = release.target;
    const platform = selectPlatform(target);
    const channel = selectedChannel();
    if (process.platform !== platform.system) {
        throw new Error("build desktop distributions on their target operating system");
    }

    // refuse a published channel's identity on another channel's version
    if (channel.name !== "dev" && channel.name !== release.channel) {
        throw new Error(`a ${channel.name} distribution requires a ${channel.name} version`);
    }

    // stage the distribution while the previous build stays in place
    const staging = join(ROOT, "dist/.build");
    await mkdir(staging, { recursive: true });
    const directory = await mkdtemp(join(staging, `${target}-`));

    // compile the commands, bundle the desktop and place the commands inside it
    await compileExecutables(directory, platform, channel);
    const application = await bundleDesktop(directory, platform, channel);
    await placeCommands(directory, join(application, platform.bundle.helpers));

    // sign distributions of signed platforms before they replace the previous build
    const signer = createSigner(platform);
    if (signer !== undefined) {
        await signDistribution(directory, platform, signer);
    }
    const destination = join(ROOT, "dist", release.version, target);
    await replace(directory, destination);
    print(destination);
}

/** Compile every executable into `bin` and lay out the compiler's toolchain beside them. */
async function compileExecutables(
    directory: string,
    platform: Platform,
    channel: ReleaseChannel,
): Promise<void> {
    // compile each executable from the installed workspace graph
    const bin = join(directory, "bin");
    for (const executable of EXECUTABLES) {
        await buildExecutable({
            root: ROOT,
            entrypoint: join(ROOT, executable.entrypoint),
            outfile: join(bin, executable.name),
            target: platform.target,
            runtime: platform.runtime,
            version,
            channel,
            ...executable.options,
        });
    }
    await buildToolchain(ROOT, platform.target, join(bin, "toolchain"));
}

/** Bundle the native desktop with Tauri and return its application directory. */
async function bundleDesktop(
    directory: string,
    platform: Platform,
    channel: ReleaseChannel,
): Promise<string> {
    // build the native window with the channel's identity and icons
    const output = join(ROOT, "dist/.native");
    const tauri = join(ROOT, "@destack/desktop/node_modules/@tauri-apps/cli/tauri.js");
    const configuration = await configureApplication(ROOT, directory, channel, version);
    const { target, bundle } = platform;
    const arguments_ = ["run", tauri, "build", "--target", target, "--config", configuration];
    arguments_.push(
        ...(bundle.tauri === undefined ? ["--no-bundle"] : ["--bundles", bundle.tauri]),
    );
    await run(process.execPath, arguments_, join(ROOT, "@destack/desktop/native"), {
        CARGO_TARGET_DIR: output,
        DESTACK_RELEASE_CHANNEL: channel.name,
    });
    await rm(join(directory, "icons"), { recursive: true });

    // copy the Tauri application bundle
    const compiled = join(output, target, "release");
    const application = join(directory, bundle.application);
    if (bundle.tauri === "app") {
        await cp(join(compiled, `bundle/macos/${channel.title}.app`), application, {
            recursive: true,
        });

        return application;
    }

    // copy the bare executable into the application directory
    await mkdir(application);
    await cp(join(compiled, basename(bundle.executable)), join(application, bundle.executable));

    // add the Linux menu artwork and installer
    if (platform.system === "linux") {
        await cp(
            join(ROOT, `@platform/brand/icon/icon${channel.suffix}-rounded.svg`),
            join(application, "icon.svg"),
        );
        await writeFile(join(application, "install.sh"), LINUX_INSTALLER, { mode: 0o755 });
    }

    return application;
}

/** Copy the commands and toolchain into the application's helpers and move the desktop host there. */
async function placeCommands(directory: string, helpers: string): Promise<void> {
    // create the platform's helper directory
    const bin = join(directory, "bin");
    await mkdir(helpers, { recursive: true });

    // copy the standalone commands the native desktop starts
    for (const name of COMMANDS) {
        await cp(join(bin, name), join(helpers, name));
    }
    await cp(join(bin, "toolchain"), join(helpers, "toolchain"), { recursive: true });
    await rename(join(bin, "destack-desktop-host"), join(helpers, "destack-desktop-host"));
}

/** Move a staged distribution to its destination, moving the previous build aside. */
async function replace(directory: string, destination: string): Promise<void> {
    // move the previous build aside when one exists
    await mkdir(dirname(destination), { recursive: true });
    try {
        await rename(destination, `${directory}.previous`);
    } catch (error) {
        if (!(error instanceof Error && "code" in error && error.code === "ENOENT")) {
            throw error;
        }
    }

    await rename(directory, destination);
}

await build();
