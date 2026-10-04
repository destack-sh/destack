import { lstat, mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { COMMANDS, selectPlatform, version } from "./index.ts";
import { schema } from "@destack/schema";
import { Release } from "@destack/update/release";
import { print } from "../output/index.ts";

/** Target executable selected for this verification host. */
const target = new Release(
    version,
    process.argv[2] ?? Release.target(process.platform, process.arch),
).target;

/** Isolated device state used to exercise the compiled host. */
const directory = await mkdtemp(join(tmpdir(), "destack-release-check-"));
/** Native CLI built for this runner. */
const executable = fileURLToPath(
    new URL(`../../../../dist/${version}/${target}/bin/destack`, import.meta.url),
);

/** A checkout the CLI reports. */
const Checkout = schema.object({ id: schema.string() }).strip();

/** The CLI's report of the daemon's connection and version. */
const Status = schema
    .object({
        status: schema.string(),
        daemon: schema.object({ version: schema.string() }).strip().exactOptional(),
    })
    .strip();

/** Run a compiled command with a bounded runtime and read its JSON response. */
async function run<Output>(arguments_: string[], response: schema.Schema<Output>): Promise<Output> {
    // run against isolated device state with a bounded lifetime
    const child = Bun.spawn([executable, ...arguments_, "--json"], {
        env: { ...process.env, DESTACK_DIRECTORY: directory },
        stdout: "pipe",
        stderr: "pipe",
        timeout: 10000,
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

/** Verify every executable before starting the distribution. */
async function verifyDistribution(): Promise<void> {
    // select the packaged application and its platform-specific executable paths
    const root = dirname(dirname(executable));
    const { bundle } = selectPlatform(target);
    const application = join(root, bundle.application);
    const helpers = join(application, bundle.helpers);
    const files = [join(application, bundle.executable), join(helpers, "destack-desktop-host")];

    // require the standalone commands and the copies used by native desktop startup
    for (const name of COMMANDS) {
        files.push(join(root, "bin", name));
        files.push(join(helpers, name));
    }

    // reject empty files, links and missing executable permissions before runtime checks
    for (const path of files) {
        const file = await lstat(path);
        if (!file.isFile() || file.size === 0) {
            throw new Error(`missing packaged file: ${path}`);
        }
        if ((file.mode & 0o111) === 0) {
            throw new Error(`packaged command is not executable: ${path}`);
        }
    }
}

/** Whether cleanup must stop the verification daemon. */
let isRunning = false;
try {
    // reject incomplete application archives before testing the executable service lifecycle
    await verifyDistribution();

    // start the compiled daemon and require the release's version
    const identity = await run(["version"], schema.object({ version: schema.string() }).strip());
    if (identity.version !== version) {
        throw new Error("CLI version does not match");
    }
    await run(["daemon", "start"], schema.json());
    isRunning = true;
    const status = await run(["status"], Status);
    if (status.status !== "connected" || status.daemon?.version !== version) {
        throw new Error("daemon version does not match");
    }

    // register a Git working directory as a checkout
    const repository = join(directory, "repository");
    if (Bun.spawnSync(["git", "init", "--quiet", repository]).exitCode !== 0) {
        throw new Error("git could not create the verification repository");
    }
    const created = await run(["checkout", "create", repository], Checkout);

    // restart the daemon and read the registered checkout back
    await run(["daemon", "stop"], schema.json());
    isRunning = false;
    await run(["daemon", "start"], schema.json());
    isRunning = true;
    const checkouts = await run(["checkout", "list"], schema.array(Checkout));
    const [checkout] = checkouts;
    if (checkouts.length !== 1 || checkout?.id !== created.id) {
        throw new Error("checkout did not survive restart");
    }
    print(
        `Verified ${version} for ${target} on ${Release.target(process.platform, process.arch)}.`,
    );
} finally {
    if (isRunning) {
        await run(["daemon", "stop"], schema.json());
    }
    await rm(directory, { recursive: true });
}
