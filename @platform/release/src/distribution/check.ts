import { dirname, join, resolve } from "node:path";
import { COMMANDS, Platform, RUNTIME } from "./index.ts";
import type { Environment, HostCapability } from "@destack/environment";
import { Device } from "@destack/machine/device";
import { schema } from "@destack/schema";
import type { Target } from "@destack/update/release";
import { SANDBOX_TOOLS, SandboxTool } from "@destack/sandbox";

/** How long one checked command may run, in milliseconds: a machine start with its keychain. */
const COMMAND_TIMEOUT_MILLISECONDS = 10_000;

/** Check a built distribution's commands and version, and that its machine keeps its host. */
export async function checkDistribution(
    directory: string,
    target: Target,
    version: string,
    environment: Environment<HostCapability>,
): Promise<void> {
    // require every packaged command before running any
    const { files } = environment;
    const executable = join(directory, "bin/destack");
    await checkExecutables(directory, target, environment);

    // run the commands over isolated device state, tracking whether cleanup stops the machine
    const state = await files.makeTemporaryDirectory("destack-distribution-check-");
    let isRunning = false;
    try {
        // require the release's version from the CLI and the machine it starts
        const identity = await run(
            executable,
            state,
            ["--version"],
            schema.object({ version: schema.string() }).strip(),
            environment,
        );
        if (identity.version !== version) {
            throw new Error(`the CLI reports ${identity.version}, not ${version}`);
        }
        await run(executable, state, ["self", "start"], schema.json(), environment);
        isRunning = true;
        const started = await Device.inspect(state, environment);
        if (started.kind !== "running" || started.computer.version !== version) {
            throw new Error(`the machine does not run ${version}`);
        }

        // restart the machine, requiring it to keep its host
        await run(executable, state, ["self", "stop"], schema.json(), environment);
        isRunning = false;
        await run(executable, state, ["self", "start"], schema.json(), environment);
        isRunning = true;
        const restarted = await Device.inspect(state, environment);
        if (restarted.kind !== "running" || restarted.computer.id !== started.computer.id) {
            throw new Error("the machine did not keep its host across a restart");
        }
    } finally {
        if (isRunning) {
            await run(executable, state, ["self", "stop"], schema.json(), environment);
        }
        await files.remove(state, { recursive: true });
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

/** Require every packaged command of a distribution, as a non-empty executable file. */
async function checkExecutables(
    root: string,
    target: Target,
    environment: Environment<"files">,
): Promise<void> {
    // select the packaged application and its platform-specific executable paths, with the Linux sandbox's tools
    const { bundle, system } = Platform.of(target);
    const application = join(root, bundle.application);
    const helpers = join(application, bundle.helpers);
    const tools =
        system === "linux"
            ? SANDBOX_TOOLS.map((tool) => join(helpers, SandboxTool.file(tool)))
            : [];
    const paths = [
        join(application, bundle.executable),
        join(helpers, RUNTIME),
        join(helpers, "destack-desktop-host"),
        ...tools,
    ];

    // require each command in the application, and its link from bin
    for (const name of COMMANDS) {
        paths.push(join(helpers, name));
        const link = join(root, "bin", name);
        const linked = await environment.files.readLink(link);
        if (resolve(dirname(link), linked) !== join(helpers, name)) {
            throw new Error(`${link} links to ${linked}, not the application's ${name}`);
        }
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
