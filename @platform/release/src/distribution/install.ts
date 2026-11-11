import { Bookmark } from "@destack/object/client";
import { Loopback } from "@destack/machine/computer";
import { Stage } from "@destack/update/release";
import type { Environment, HostCapability } from "@destack/environment";
import { present } from "@destack/schema";
import { join } from "node:path";
import { isDeepStrictEqual } from "node:util";
import { RepositoryConfiguration } from "../repository/index.ts";
import { print } from "./output.ts";

/** How long one shipped command may run, in milliseconds. */
const COMMAND_TIMEOUT_MILLISECONDS = 120_000;

/** The installation under check: its directory, its command, and whether its machine runs. */
interface Installation {
    /** The installation's `DESTACK_DIRECTORY`. */
    readonly directory: string;
    /** The installed command, in the installation's `bin` directory. */
    readonly executable: string;
    /** Whether the installed machine runs. */
    isRunning: boolean;
}

/** Install the published release as people do, and verify its machine and update selection. */
export async function verifyInstallation(environment: Environment<HostCapability>): Promise<void> {
    // check inside one temporary directory and keep the first failure
    const configuration = new RepositoryConfiguration(environment);
    const directory = await environment.files.makeTemporaryDirectory("destack-installation-");
    const state = join(directory, "state");
    const executable = join(state, "bin", new Stage("production").command);
    const installation: Installation = { directory: state, executable, isRunning: false };
    let failure: Error | undefined;
    try {
        await verifyInstall(configuration, installation, environment);
    } catch (error) {
        // preserve the original verification failure if cleanup also fails
        failure =
            error instanceof Error
                ? error
                : new Error("installation verification failed", { cause: error });
    }

    // complete cleanup before reporting either verification or cleanup failure
    try {
        await cleanInstallation(directory, installation, failure === undefined, environment);
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

/** Install the feed's release through its install script, then reinstall and restart it. */
async function verifyInstall(
    configuration: RepositoryConfiguration,
    installation: Installation,
    environment: Environment<HostCapability>,
): Promise<void> {
    // pipe the feed's install script into the shell, as the site's install command does
    const { executable } = installation;
    const version = present(
        environment.process.variables["DESTACK_RELEASE_VERSION"],
        "the release version",
    );
    const variables = {
        ...environment.process.variables,
        DESTACK_DIRECTORY: installation.directory,
        DESTACK_UPDATE_URL: configuration.url.href,
    };
    const script = new URL("install", configuration.url).href;
    await command("sh", ["-c", 'curl -fsSL "$1" | sh', "sh", script], variables, environment);
    installation.isRunning = true;

    // install again through the shipped command, then restart its machine
    await command(executable, ["self", "install"], variables, environment);
    await verifyMachineRestart(installation, variables, environment);

    // require the installed release to be the feed's latest
    const selection: unknown = JSON.parse(
        await command(executable, ["self", "update", "--stage", "--json"], variables, environment),
    );
    if (!isDeepStrictEqual(selection, { installed: version, staged: null })) {
        throw new Error(`installed release differs from '${version}'`);
    }
    print(`verified installation and restart of ${version} from ${configuration.url}`, environment);
}

/** Start, stop and start the installed machine, requiring one host identity across the restart. */
async function verifyMachineRestart(
    installation: Installation,
    variables: Readonly<Record<string, string>>,
    environment: Environment<HostCapability>,
): Promise<void> {
    // record the host of the running machine
    const { directory, executable } = installation;
    await command(executable, ["self", "start", "--json"], variables, environment);
    const before = await new Loopback(directory, environment, new Bookmark()).computer();

    // restart the machine
    await command(executable, ["self", "stop", "--json"], variables, environment);
    installation.isRunning = false;
    await command(executable, ["self", "start", "--json"], variables, environment);
    installation.isRunning = true;

    // compare the host of the restarted machine
    const after = await new Loopback(directory, environment, new Bookmark()).computer();
    if (before.id !== after.id) {
        throw new Error("installed machine did not preserve its host across restart");
    }
}

/** Stop a running installed machine and delete the directory of a verified installation. */
async function cleanInstallation(
    directory: string,
    installation: Installation,
    isVerified: boolean,
    environment: Environment<HostCapability>,
): Promise<void> {
    // stop a running machine before deleting its executable files
    if (installation.isRunning) {
        const variables = {
            ...environment.process.variables,
            DESTACK_DIRECTORY: installation.directory,
        };
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
            `installed command '${arguments_.join(" ")}' failed ('${code}'): '${diagnostic}'`,
        );
    }

    return output;
}
