import { selectedChannel } from "./identity.ts";
import { Updater, Release } from "@destack/update";
import type { Target } from "@destack/update/release";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { isDeepStrictEqual } from "node:util";
import { schema } from "@destack/schema";
import { RepositoryConfiguration } from "../repository/index.ts";
import { print } from "../output/index.ts";

/** The running host `destack status --json` reports, by its identity. */
const Status = schema
    .object({ daemon: schema.object({ id: schema.string().min(1) }).strip() })
    .strip();

/** The staged executable and its daemon state, which cleanup reads after a failure. */
interface Installation {
    /** The staged executable, once staging finishes. */
    executable: string | undefined;
    /** Whether the installed daemon runs. */
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

/** Install the published native release and verify its persistent daemon and update selection. */
async function verifyInstallation(): Promise<void> {
    // verify inside one temporary directory and keep the first failure
    const configuration = new RepositoryConfiguration();
    const target = Release.target(process.platform, process.arch);
    const directory = await mkdtemp(join(tmpdir(), "destack-published-"));
    const state = join(directory, "state");
    const installation: Installation = { executable: undefined, isRunning: false };
    let failure: Error | undefined;
    try {
        await verifyRelease(configuration, target, directory, state, installation);
    } catch (error) {
        // preserve the original verification failure if cleanup also fails
        failure =
            error instanceof Error
                ? error
                : new Error("installation verification failed", { cause: error });
    }

    // complete cleanup before reporting either verification or cleanup failure
    try {
        await cleanInstallation(directory, state, installation, failure === undefined);
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
): Promise<void> {
    // authenticate downloaded executables before invoking the distribution's own installer
    const { version, archive, executable } = await stageRelease(configuration, directory, target);
    installation.executable = executable;

    // install twice through the shipped CLI and preserve one isolated user directory
    const environment = {
        ...process.env,
        DESTACK_DIRECTORY: state,
        DESTACK_UPDATE_URL: configuration.url.href,
    };
    await command(executable, ["self", "install", "--archive", archive], environment);
    await command(executable, ["self", "install", "--archive", archive], environment);
    await verifyDaemonRestart(executable, environment, installation);

    // require the installed executable to authenticate its current public update feed
    const selection: unknown = JSON.parse(
        await command(executable, ["self", "update", "--check", "--json"], environment),
    );
    if (!isDeepStrictEqual(selection, { current: version, latest: version, available: false })) {
        throw new Error("installed update selection differs from the published release");
    }
    print(`verified installation and restart of ${version} on ${target}`);
}

/** Download and stage the workflow's published release through a bootstrap updater. */
async function stageRelease(
    configuration: RepositoryConfiguration,
    directory: string,
    target: Target,
): Promise<Staging> {
    // open a bootstrap updater on the published repository
    await using updater = await Updater.open({
        directory: join(directory, "bootstrap"),
        repository: configuration.url,
        channel: configuration.channel,
        root: JSON.stringify((await configuration.root()).toJSON()),
        target,
        applicationIdentifier: selectedChannel().applicationIdentifier,
        ...(process.platform === "darwin" && {
            application: join(directory, "bootstrap.app"),
        }),
    });

    // require the workflow version as the installable release
    const update = await updater.check();
    if (!update) {
        throw new Error("published repository contains no installable release");
    }
    const version = update.release.version;
    if (version !== process.env["DESTACK_RELEASE_VERSION"]) {
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

/** Start, stop and start the installed daemon and require the same host identity across the restart. */
async function verifyDaemonRestart(
    executable: string,
    environment: NodeJS.ProcessEnv,
    installation: Installation,
): Promise<void> {
    // record the host of the first daemon
    await command(executable, ["daemon", "start", "--json"], environment);
    installation.isRunning = true;
    const before = Status.parse(
        JSON.parse(await command(executable, ["status", "--json"], environment)),
    );

    // restart the daemon
    await command(executable, ["daemon", "stop", "--json"], environment);
    installation.isRunning = false;
    await command(executable, ["daemon", "start", "--json"], environment);
    installation.isRunning = true;

    // compare the host of the restarted daemon
    const after = Status.parse(
        JSON.parse(await command(executable, ["status", "--json"], environment)),
    );
    if (before.daemon.id !== after.daemon.id) {
        throw new Error("installed daemon did not preserve its host across restart");
    }
}

/** Stop a running installed daemon and delete the directory of a verified installation. */
async function cleanInstallation(
    directory: string,
    state: string,
    installation: Installation,
    isVerified: boolean,
): Promise<void> {
    // stop verified installations before deleting their temporary executable files
    if (installation.isRunning && installation.executable !== undefined) {
        await command(installation.executable, ["daemon", "stop", "--json"], {
            ...process.env,
            DESTACK_DIRECTORY: state,
        });
    }

    // retain failed installations for runner diagnostics
    if (isVerified) {
        await rm(directory, { recursive: true, force: true });
    }
}

/** Execute a shipped command with bounded runtime and complete diagnostics. */
async function command(
    executable: string,
    arguments_: string[],
    environment: NodeJS.ProcessEnv,
): Promise<string> {
    // retain output for exact assertions without inheriting interactive input
    const child = Bun.spawn([executable, ...arguments_], {
        env: environment,
        stdin: "ignore",
        stdout: "pipe",
        stderr: "pipe",
        timeout: 120000,
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

await verifyInstallation();
