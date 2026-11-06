import { Bookmark } from "@destack/object/client";
import { Loopback } from "@destack/machine/computer";
import { selectedChannel } from "./identity.ts";
import type { Environment, HostCapability } from "@destack/environment";
import { Updater, Release } from "@destack/update";
import type { Target } from "@destack/update/release";
import { join } from "node:path";
import { isDeepStrictEqual } from "node:util";
import { RepositoryConfiguration } from "../repository/index.ts";
import { print } from "./output.ts";

/** How long one shipped command may run, in milliseconds. */
const COMMAND_TIMEOUT_MILLISECONDS = 120_000;

/** The staged executable and its machine state, which cleanup reads after a failure. */
interface Installation {
    /** The staged executable, once staging finishes. */
    executable: string | undefined;
    /** Whether the installed machine runs. */
    isRunning: boolean;
}

/** A published release staged from the repository. */
interface Staging {
    /** The published version. */
    version: string;
    /** The downloaded update archive. */
    archive: string;
    /** The staged `destack` executable. */
    executable: string;
}

/** Install the published native release and verify its persistent machine and update selection. */
export async function verifyInstallation(environment: Environment<HostCapability>): Promise<void> {
    // verify inside one temporary directory and keep the first failure
    const configuration = new RepositoryConfiguration(environment);
    const target = Release.target(environment.host.os, environment.host.architecture);
    const directory = await environment.files.makeTemporaryDirectory("destack-published-");
    const state = join(directory, "state");
    const installation: Installation = { executable: undefined, isRunning: false };
    let failure: Error | undefined;
    try {
        await verifyRelease(configuration, target, directory, state, installation, environment);
    } catch (error) {
        // preserve the original verification failure if cleanup also fails
        failure =
            error instanceof Error
                ? error
                : new Error("installation verification failed", { cause: error });
    }

    // complete cleanup before reporting either verification or cleanup failure
    try {
        await cleanInstallation(directory, state, installation, failure === undefined, environment);
    } catch (error) {
        if (failure !== undefined) {
            throw new AggregateError(
                [failure, error],
                "installation verification and cleanup failed",
                { cause: error },
            );
        }
        throw error;
    }
    if (failure !== undefined) {
        throw failure;
    }
}

/** Install and restart the published release and check its update selection. */
async function verifyRelease(
    configuration: RepositoryConfiguration,
    target: Target,
    directory: string,
    state: string,
    installation: Installation,
    environment: Environment<HostCapability>,
): Promise<void> {
    // authenticate downloaded executables before invoking the distribution's own installer
    const staging = await stageRelease(configuration, directory, target, environment);
    const { version, archive, executable } = staging;
    installation.executable = executable;

    // install twice through the shipped CLI and preserve one isolated user directory
    const variables = {
        ...environment.process.variables,
        DESTACK_DIRECTORY: state,
        DESTACK_UPDATE_URL: configuration.url.href,
    };
    const install = ["self", "install", "--archive", archive];
    await command(executable, install, variables, environment);
    await command(executable, install, variables, environment);
    await verifyMachineRestart(executable, state, variables, installation, environment);

    // require the installed executable to authenticate its current public update feed
    const selection: unknown = JSON.parse(
        await command(executable, ["self", "update", "--stage", "--json"], variables, environment),
    );
    if (!isDeepStrictEqual(selection, { installed: version, staged: null })) {
        throw new Error("installed update selection differs from the published release");
    }
    print(`verified installation and restart of ${version} on ${target}`, environment);
}

/** Download and stage the workflow's published release through a bootstrap updater. */
async function stageRelease(
    configuration: RepositoryConfiguration,
    directory: string,
    target: Target,
    environment: Environment<HostCapability>,
): Promise<Staging> {
    // open a bootstrap updater on the published repository
    await using updater = await Updater.open({
        directory: join(directory, "bootstrap"),
        repository: configuration.url,
        root: JSON.stringify(configuration.root().toJSON()),
        target,
        applicationIdentifier: selectedChannel(environment).applicationIdentifier,
        environment,
        ...(environment.host.os === "darwin" && {
            application: join(directory, "bootstrap.app"),
        }),
    });

    // require the workflow version as the installable release
    const update = await updater.check();
    if (!update) {
        throw new Error("published repository contains no installable release");
    }
    const version = update.release.version;
    if (version !== environment.process.variables["DESTACK_RELEASE_VERSION"]) {
        throw new Error("published release differs from the workflow version");
    }

    // download and stage its archive
    const download = await update.download();
    const staged = await updater.stage(download);

    return {
        version,
        archive: download.archive,
        executable: join(staged.directory, "bin", "destack"),
    };
}

/** Start, stop and start the installed machine, requiring one host identity across the restart. */
async function verifyMachineRestart(
    executable: string,
    state: string,
    variables: Readonly<Record<string, string>>,
    installation: Installation,
    environment: Environment<HostCapability>,
): Promise<void> {
    // record the host of the first machine
    await command(executable, ["self", "start", "--json"], variables, environment);
    installation.isRunning = true;
    const before = await new Loopback(state, environment, new Bookmark()).computer();

    // restart the machine
    await command(executable, ["self", "stop", "--json"], variables, environment);
    installation.isRunning = false;
    await command(executable, ["self", "start", "--json"], variables, environment);
    installation.isRunning = true;

    // compare the host of the restarted machine
    const after = await new Loopback(state, environment, new Bookmark()).computer();
    if (before.id !== after.id) {
        throw new Error("installed machine did not preserve its host across restart");
    }
}

/** Stop a running installed machine and delete the directory of a verified installation. */
async function cleanInstallation(
    directory: string,
    state: string,
    installation: Installation,
    isVerified: boolean,
    environment: Environment<HostCapability>,
): Promise<void> {
    // stop verified installations before deleting their temporary executable files
    if (installation.isRunning && installation.executable !== undefined) {
        const variables = { ...environment.process.variables, DESTACK_DIRECTORY: state };
        await command(installation.executable, ["self", "stop", "--json"], variables, environment);
    }

    // retain failed installations for runner diagnostics
    if (isVerified) {
        await environment.files.remove(directory, { recursive: true, force: true });
    }
}

/** Execute a shipped command with bounded runtime and complete diagnostics. */
async function command(
    executable: string,
    arguments_: string[],
    variables: Readonly<Record<string, string>>,
    environment: Environment<"process">,
): Promise<string> {
    // retain output for exact assertions without inheriting interactive input
    const child = environment.process.spawn([executable, ...arguments_], {
        variables,
        stdin: "ignore",
        stdout: "pipe",
        stderr: "pipe",
        signal: environment.clock.timeout(COMMAND_TIMEOUT_MILLISECONDS),
    });
    const [code, output, diagnostic] = await Promise.all([
        child.exited,
        new Response(child.stdout).text(),
        new Response(child.stderr).text(),
    ]);
    if (code !== 0) {
        throw new Error(
            `installed command ${arguments_.join(" ")} failed (${code}): ${diagnostic}`,
        );
    }

    return output;
}
