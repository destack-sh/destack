import { CHECKOUT } from "./distribution.ts";
import { dirname, join } from "node:path";
import { COMMANDS, Platform, releaseVersion } from "./index.ts";
import type { Environment, HostCapability } from "@destack/environment";
import { Device } from "@destack/machine/device";
import { schema } from "@destack/schema";
import { Release, type Target } from "@destack/update/release";
import { print } from "./output.ts";

/** How long one verified command may run, in milliseconds: a machine start with its keychain. */
const COMMAND_TIMEOUT_MILLISECONDS = 10_000;

/** A checkout the CLI reports. */
const Checkout = schema.object({ id: schema.string() }).strip();

/** Verify a compiled distribution's files, then its machine over isolated device state. */
export async function verifyDistribution(
    environment: Environment<HostCapability>,
    arguments_: readonly string[],
): Promise<void> {
    // select the target the command line requests, this host's by default
    const { files } = environment;
    const { os, architecture } = environment.host;
    const version = await releaseVersion(CHECKOUT, environment);
    const [selected] = arguments_;
    const target = new Release(version, selected ?? Release.target(os, architecture)).target;

    // create isolated device state and select the native CLI built for this runner
    const directory = await files.makeTemporaryDirectory("destack-release-check-");
    const executable = join(CHECKOUT, "dist", version, target, "bin/destack");

    // track whether cleanup must stop the verification machine
    let isRunning = false;
    try {
        // reject incomplete application archives before testing the executable service lifecycle
        await verifyExecutables(executable, target, environment);

        // start the compiled machine and require the release's version
        const identity = await run(
            executable,
            directory,
            ["--version"],
            schema.object({ version: schema.string() }).strip(),
            environment,
        );
        if (identity.version !== version) {
            throw new Error("CLI version does not match");
        }
        await run(executable, directory, ["self", "start"], schema.json(), environment);
        isRunning = true;
        const status = await Device.inspect(directory, environment);
        if (status.kind !== "running" || status.computer.version !== version) {
            throw new Error("machine version does not match");
        }

        // register a Git working directory as a checkout
        const repository = join(directory, "repository");
        await createRepository(repository, environment);
        const created = await run(
            executable,
            directory,
            ["checkout", "create", "--directory", repository],
            Checkout,
            environment,
        );

        // restart the machine and read the registered checkout back
        await run(executable, directory, ["self", "stop"], schema.json(), environment);
        isRunning = false;
        await run(executable, directory, ["self", "start"], schema.json(), environment);
        isRunning = true;
        const checkouts = await run(
            executable,
            directory,
            ["checkouts"],
            schema.array(Checkout),
            environment,
        );
        const [checkout] = checkouts;
        if (checkouts.length !== 1 || checkout?.id !== created.id) {
            throw new Error("checkout did not survive restart");
        }
        print(
            `Verified ${version} for ${target} on ${Release.target(os, architecture)}.`,
            environment,
        );
    } finally {
        if (isRunning) {
            await run(executable, directory, ["self", "stop"], schema.json(), environment);
        }
        await files.remove(directory, { recursive: true });
    }
}

/** Run a compiled command over a device directory and read its JSON response. */
async function run<Output>(
    executable: string,
    directory: string,
    words: string[],
    response: schema.Schema<Output>,
    environment: Environment<"process">,
): Promise<Output> {
    // run against isolated device state with a bounded lifetime
    const child = environment.process.spawn([executable, ...words, "--json"], {
        variables: { ...environment.process.variables, DESTACK_DIRECTORY: directory },
        stdout: "pipe",
        stderr: "pipe",
        signal: environment.clock.timeout(COMMAND_TIMEOUT_MILLISECONDS),
    });
    const [code, output, error] = await Promise.all([
        child.exited,
        new Response(child.stdout).text(),
        new Response(child.stderr).text(),
    ]);
    if (code !== 0) {
        throw new Error(error);
    }

    return response.parse(JSON.parse(output));
}

/** Verify every executable of a target's distribution before starting it. */
async function verifyExecutables(
    executable: string,
    target: Target,
    environment: Environment<"files">,
): Promise<void> {
    // select the packaged application and its platform-specific executable paths
    const root = dirname(dirname(executable));
    const { bundle } = Platform.of(target);
    const application = join(root, bundle.application);
    const helpers = join(application, bundle.helpers);
    const paths = [join(application, bundle.executable), join(helpers, "destack-desktop-host")];

    // require the standalone commands and the copies used by native desktop startup
    for (const name of COMMANDS) {
        paths.push(join(root, "bin", name));
        paths.push(join(helpers, name));
    }

    // reject empty files, links and missing executable permissions before runtime checks
    for (const path of paths) {
        const file = await environment.files.lstat(path);
        if (file.kind !== "file" || file.size === 0) {
            throw new Error(`missing packaged file: ${path}`);
        }
        if ((file.mode & 0o111) === 0) {
            throw new Error(`packaged command is not executable: ${path}`);
        }
    }
}

/** Create a Git working directory, refusing a failed creation. */
async function createRepository(
    repository: string,
    environment: Environment<"process">,
): Promise<void> {
    const child = environment.process.spawn(["git", "init", "--quiet", repository], {
        stdout: "ignore",
        stderr: "ignore",
    });
    if ((await child.exited) !== 0) {
        throw new Error("git could not create the verification repository");
    }
}
