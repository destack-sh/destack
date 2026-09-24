import { lstat, mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { version } from "./index.ts";
import { Release } from "@destack/update/release";

/** Target executable selected for this verification host. */
const target = new Release(version, process.argv[2] ?? Release.target()).target;

/** Isolated device state used to exercise the compiled host. */
const directory = await mkdtemp(join(tmpdir(), "destack-release-check-"));
/** Native CLI built for this runner. */
const executable = fileURLToPath(
    new URL(
        `../../../../dist/${version}/${target}/bin/${
            process.platform === "win32" ? "destack.exe" : "destack"
        }`,
        import.meta.url,
    ),
);

/** Run a compiled command with a bounded runtime and return its JSON response. */
async function run(arguments_: string[]): Promise<Record<string, unknown>> {
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

    return JSON.parse(output);
}

/** Verify every executable and the default view before starting the distribution. */
async function verifyDistribution(): Promise<void> {
    // select the packaged application and its platform-specific executable paths
    const root = dirname(dirname(executable));
    const isMac = target.endsWith("apple-darwin");
    const suffix = target.endsWith("windows-msvc") ? ".exe" : "";
    const application = join(root, isMac ? "Destack.app/Contents" : "Destack");
    const commands = isMac ? "Helpers" : "helpers";
    const files = [
        join(application, isMac ? "MacOS/Destack" : `Destack${suffix}`),
        join(application, commands, `destack-desktop-host${suffix}`),
        join(application, "view/launch.json"),
    ];

    // require the standalone commands and the copies used by native desktop startup
    for (const name of ["destack", "destack-daemon", "destack-sandbox"]) {
        files.push(join(root, "bin", `${name}${suffix}`));
        files.push(join(application, commands, `${name}${suffix}`));
    }

    // reject empty files, links and missing executable permissions before runtime checks
    for (const path of files) {
        const file = await lstat(path);
        if (!file.isFile() || file.size === 0) {
            throw new Error(`missing packaged file: ${path}`);
        }
        if (!suffix && !path.endsWith(".json") && (file.mode & 0o111) === 0) {
            throw new Error(`packaged command is not executable: ${path}`);
        }
    }
}

/** Whether cleanup must stop the verification daemon. */
let isRunning = false;
try {
    // reject incomplete application archives before testing the executable service lifecycle
    await verifyDistribution();

    // start the compiled daemon and persist a change through its HTTP client
    const identity = await run(["version"]);
    if (identity.version !== version) {
        throw new Error("CLI version does not match");
    }
    await run(["daemon", "start"]);
    isRunning = true;
    const status = await run(["status"]);
    if (
        status.status !== "connected" ||
        (status.daemon as { version?: string })?.version !== version
    ) {
        throw new Error("daemon version does not match");
    }
    await run(["host", "rename", "Release check"]);
    await run(["daemon", "stop"]);
    isRunning = false;
    await run(["daemon", "start"]);
    isRunning = true;
    const host = await run(["host", "get"]);
    if (host.name !== "Release check") {
        throw new Error("host state did not survive restart");
    }
    console.log(`Verified ${version} for ${target} on ${Release.target()}.`);
} finally {
    if (isRunning) {
        await run(["daemon", "stop"]);
    }
    await rm(directory, { recursive: true });
}
