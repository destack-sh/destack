import { CHECKOUT } from "./distribution.ts";
import { Stage } from "@destack/update/release";
import { checkDistribution } from "./check.ts";
import { buildSandboxTools } from "./tools.ts";
import { DESKTOP_ENTRY, PlatformBuild } from "../deployment/index.ts";
import { basename, dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";
import { Release } from "@destack/update/release";
import { Version } from "@destack/schema";
import { FileSystemError } from "@destack/environment";
import type { Environment, HostCapability } from "@destack/environment";
import {
    buildExecutable,
    buildToolchain,
    COMMANDS,
    CONFIGURATION_FILE,
    PROGRAM_DIRECTORY,
    RUNTIME,
    toolchainOptions,
    writeToolchainManifest,
    type ExecutableOptions,
    Platform,
    readWorkspace,
    releaseVersion,
} from "./index.ts";
import { read, run } from "./command.ts";
import { print } from "./output.ts";
import { configureApplication } from "./application.ts";
import { Signer } from "../signing/signer.ts";
import { Distribution, DISTRIBUTION_DIRECTORY, TOOLCHAIN_DIRECTORY } from "@destack/update";

/** The directory beside a built distribution with its bundles' source maps, outside archives. */
const MAPS_DIRECTORY = "maps";

/** The Linux script that installs the extracted application's commands and desktop entry. */
const LINUX_INSTALLER =
    '#!/bin/sh\nset -eu\napplication=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)\n"$application/helpers/destack" self install\n"$application/helpers/destack" desktop\n';

/** Build the native window, TypeScript host, and CLI as one distribution. */
export async function buildDistribution(
    environment: Environment<HostCapability>,
    arguments_: readonly string[],
): Promise<void> {
    // compile distributions with the pinned runtime
    const { files } = environment;
    const version = await releaseVersion(CHECKOUT, environment);
    const { packageManager } = await readWorkspace(CHECKOUT, environment);
    const runtime = (await read(environment.process.executable, ["--version"], environment)).trim();
    if (`bun@${runtime}` !== packageManager) {
        throw new Error(`release builds require '${packageManager}'`);
    }

    // build each native desktop on its matching operating system
    const { os, architecture } = environment.host;
    const [selected] = arguments_;
    const release = new Release(version, selected ?? Release.target(os, architecture));
    const target = release.target;
    const platform = Platform.of(target);
    const stage = new Stage("production");
    if (os !== platform.system) {
        throw new Error("build desktop distributions on their target operating system");
    }

    // refuse a dev build's version, which the feed never publishes
    if (Version.isPrerelease(release.version)) {
        throw new Error(`a distribution requires a release version, not '${release.version}'`);
    }

    // stage the distribution while the previous build stays in place
    const staging = join(CHECKOUT, "dist/.build");
    await files.mkdir(staging, { recursive: true });
    const directory = await files.makeTemporaryDirectory(`${target}-`, { directory: staging });

    // compile the commands, bundle the desktop and place the commands inside it
    await compileExecutables(directory, platform, stage, version, environment);
    const application = await bundleDesktop(directory, platform, stage, version, environment);
    const helpers = join(application, platform.bundle.helpers);
    await placeCommands(directory, helpers, platform, environment);

    // build the sandbox's tools beside its launcher on Linux, where the sandbox runs them
    if (platform.system === "linux") {
        const scratch = await files.makeTemporaryDirectory(`${target}-tools-`, {
            directory: staging,
        });
        await buildSandboxTools(helpers, scratch, environment);
        await files.remove(scratch, { recursive: true });
    }

    // sign distributions of signed platforms before they replace the previous build
    const signer = Signer.of(platform, environment);
    if (signer !== undefined) {
        await Signer.certify(directory, platform, signer, environment);
    }
    // run the built distribution before it replaces the previous build
    await checkDistribution(directory, target, release.version, environment);
    const destination = join(CHECKOUT, "dist", release.version, target);
    await replace(directory, destination, environment);
    print(destination, environment);
}

/** Compile every executable into `bin` with the packages they load, the toolchain beside it. */
async function compileExecutables(
    directory: string,
    platform: Platform,
    stage: Stage,
    version: string,
    environment: Environment<HostCapability>,
): Promise<void> {
    // select the executables `bin` has, with the options each needs
    const { files } = environment;
    const executables: readonly {
        readonly name: string;
        readonly entrypoint: string;
        readonly options: Pick<ExecutableOptions, "external" | "plugins">;
    }[] = [
        { name: "destack", entrypoint: "@destack/cli/src/main.ts", options: {} },
        { name: "destack-machine", entrypoint: DESKTOP_ENTRY, options: toolchainOptions(files) },
        {
            name: "destack-sandbox",
            entrypoint: "@destack/sandbox/src/main.ts",
            options: {},
        },
        {
            name: "destack-build",
            entrypoint: "@destack/build/src/main.ts",
            options: toolchainOptions(files),
        },
        {
            name: "destack-desktop-host",
            entrypoint: "@destack/desktop/src/main.ts",
            options: {},
        },
    ];

    // compile each executable from the installed workspace graph
    const bin = join(directory, "bin");
    for (const executable of executables) {
        await buildExecutable(
            {
                root: CHECKOUT,
                entrypoint: join(CHECKOUT, executable.entrypoint),
                outfile: join(bin, executable.name),
                target: platform.target,
                runtime: platform.runtime,
                version,
                stage,
                ...executable.options,
            },
            environment,
        );
    }
    await buildToolchain(
        CHECKOUT,
        platform.target,
        { resources: bin, toolchain: join(directory, TOOLCHAIN_DIRECTORY) },
        environment,
    );

    // build the first-party releases from the checkout, reusing their store across builds
    await using releases = await PlatformBuild.build(CHECKOUT, environment);
    await releases.writeDistribution(join(bin, DISTRIBUTION_DIRECTORY));
}

/** Bundle the native desktop with Tauri and return its application directory. */
async function bundleDesktop(
    directory: string,
    platform: Platform,
    stage: Stage,
    version: string,
    environment: Environment<HostCapability>,
): Promise<string> {
    // build the native window with the stage's identity and icons
    const output = join(CHECKOUT, "dist/.native");
    const tauri = join(CHECKOUT, "@destack/desktop/node_modules/@tauri-apps/cli/tauri.js");
    const configuration = await configureApplication(CHECKOUT, output, stage, version, environment);
    const { target, bundle } = platform;
    const arguments_ = ["run", tauri, "build", "--target", target, "--config", configuration];
    arguments_.push(
        ...(bundle.tauri === undefined ? ["--no-bundle"] : ["--bundles", bundle.tauri]),
    );
    await run(
        environment.process.executable,
        arguments_,
        environment,
        join(CHECKOUT, "@destack/desktop/native"),
        { CARGO_TARGET_DIR: output, DESTACK_STAGE: stage.name },
    );
    const { files } = environment;

    // copy the Tauri application bundle
    const compiled = join(output, target, "release");
    const application = join(directory, bundle.application);
    if (bundle.tauri === "app") {
        await files.copy(join(compiled, `bundle/macos/${stage.title}.app`), application);

        return application;
    }

    // copy the bare executable into the application directory
    await files.mkdir(application);
    await files.copyFile(
        join(compiled, basename(bundle.executable)),
        join(application, bundle.executable),
    );

    // add the Linux menu artwork and installer
    if (platform.system === "linux") {
        await files.copyFile(
            fileURLToPath(
                import.meta.resolve(`@platform/brand/icon/icon${stage.suffix}-rounded.svg`),
            ),
            join(application, "icon.svg"),
        );
        await files.writeTextFile(join(application, "install.sh"), LINUX_INSTALLER, {
            mode: 0o755,
        });
    }

    return application;
}

/** Move the runtime, the launchers and their resources into the application, linked from `bin`. */
async function placeCommands(
    directory: string,
    helpers: string,
    platform: Platform,
    environment: Environment<"files">,
): Promise<void> {
    // create the platform's helper directory and its resources
    const { files } = environment;
    const bin = join(directory, "bin");
    const resources = Distribution.resources(join(helpers, "destack"));
    await files.mkdir(helpers, { recursive: true });
    await files.mkdir(resources, { recursive: true });

    // move the bundles' source maps beside the distribution, for observability
    const programs = join(bin, PROGRAM_DIRECTORY);
    await files.mkdir(join(directory, MAPS_DIRECTORY));
    for (const entry of await files.readDirectory(programs)) {
        if (entry.name.endsWith(".map")) {
            await files.rename(
                join(programs, entry.name),
                join(directory, MAPS_DIRECTORY, entry.name),
            );
        }
    }

    // move the bundles, their configuration, the packages they load and the first-party releases into the resources
    for (const name of [
        PROGRAM_DIRECTORY,
        CONFIGURATION_FILE,
        "node_modules",
        DISTRIBUTION_DIRECTORY,
    ]) {
        await files.rename(join(bin, name), join(resources, name));
    }

    // list the toolchain beside the application in its resources
    const toolchain = join(directory, TOOLCHAIN_DIRECTORY);
    await writeToolchainManifest(toolchain, resources, platform.architecture, environment);

    // move the runtime and the launchers the desktop starts into the helpers
    for (const name of [RUNTIME, "destack-desktop-host", ...COMMANDS]) {
        await files.rename(join(bin, name), join(helpers, name));
    }

    // link each command from bin, which resolves the launcher through the link
    for (const name of COMMANDS) {
        await files.symlink(relative(bin, join(helpers, name)), join(bin, name));
    }
}

/** Move a staged distribution to its destination, moving the previous build aside. */
async function replace(
    directory: string,
    destination: string,
    environment: Environment<"files">,
): Promise<void> {
    // move the previous build aside when one exists
    const { files } = environment;
    await files.mkdir(dirname(destination), { recursive: true });
    await files
        .rename(destination, `${directory}.previous`)
        .catch((error: unknown) => FileSystemError.absent(error));

    // move the staged build into place
    await files.rename(directory, destination);
}
