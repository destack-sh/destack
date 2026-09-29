import { expect, test } from "@destack/test";
import { mkdtemp, realpath, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import solidPlugin from "@opentui/solid/bun-plugin";
import { buildExecutable } from "./executable.ts";

/** Source root used only while producing the executables. */
const ROOT = fileURLToPath(new URL("../../../../", import.meta.url));

test.skipIf(process.platform !== "darwin" || process.arch !== "arm64")(
    "run compiled clients, persist daemon state and enforce the macOS sandbox outside the checkout",
    async () => {
        const directory = await realpath(await mkdtemp(join(tmpdir(), "destack-executable-")));
        const environment = {
            PATH: "/usr/bin:/bin",
            HOME: directory,
            DESTACK_DIRECTORY: join(directory, "state"),
        };
        let isRunning = false;
        try {
            // compile the real programs and a consumer of the packaged sandbox API
            for (const [source, name] of [
                ["@destack/cli/src/main.ts", "destack"],
                ["@destack/daemon/src/main.ts", "destack-daemon"],
                ["@destack/sandbox/src/main.ts", "destack-sandbox"],
                ["platform/release/src/distribution/tests/sandbox.ts", "sandbox-check"],
            ]) {
                await buildExecutable({
                    root: ROOT,
                    entrypoint: join(ROOT, source),
                    outfile: join(directory, name),
                    target: "aarch64-apple-darwin",
                    runtime: "bun-darwin-arm64",
                    version: "2026.9.123-nightly.1",
                    identity: "nightly",
                    plugins: name === "destack" ? [solidPlugin] : [],
                });
            }

            // run commands with no checkout, inherited credentials or installed runtime on PATH
            const cli = join(directory, "destack");
            expect(
                JSON.parse(await run([cli, "version", "--json"], directory, environment)),
            ).toEqual({
                version: "2026.9.123-nightly.1",
            });
            await run([cli, "daemon", "start", "--json"], directory, environment);
            isRunning = true;
            const host = JSON.parse(
                await run([cli, "host", "get", "--json"], directory, environment),
            );
            const renamed = JSON.parse(
                await run(
                    [cli, "host", "rename", "Packaged host", "--json"],
                    directory,
                    environment,
                ),
            );
            expect(renamed).toEqual({ ...host, name: "Packaged host" });
            expect(
                JSON.parse(await run([cli, "daemon", "stop", "--json"], directory, environment)),
            ).toEqual({ stopped: true });
            isRunning = false;
            await run([cli, "daemon", "start", "--json"], directory, environment);
            isRunning = true;
            const restarted = JSON.parse(
                await run([cli, "host", "get", "--json"], directory, environment),
            );
            expect(restarted).toEqual(renamed);

            // enforce the same restrictions through the compiled parent and launcher
            await writeFile(join(directory, "readable.txt"), "public");
            await writeFile(join(directory, "private.txt"), "private");
            const sandbox = await run(
                [join(directory, "sandbox-check"), directory],
                directory,
                environment,
            );
            expect(JSON.parse(sandbox)).toEqual({ allowed: "public", denied: "EPERM" });
        } finally {
            if (isRunning) {
                await run(
                    [join(directory, "destack"), "daemon", "stop", "--json"],
                    directory,
                    environment,
                );
            }
            await rm(directory, { recursive: true, force: true });
        }
    },
);

/** Execute one compiled command with a bounded lifetime and retained diagnostics. */
async function run(
    command: string[],
    directory: string,
    environment: Record<string, string>,
): Promise<string> {
    const process = Bun.spawn(command, {
        cwd: directory,
        env: environment,
        stdout: "pipe",
        stderr: "pipe",
        timeout: 3000,
    });
    const [code, output, error] = await Promise.all([
        process.exited,
        new Response(process.stdout).text(),
        new Response(process.stderr).text(),
    ]);
    expect(code, error).toBe(0);

    return output;
}
