import { expect, test } from "@destack/test";
import { mkdtemp, realpath, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import solidPlugin from "@opentui/solid/bun-plugin";
import { Compiler, PackageBuilder, readDependencies, readOutputs } from "@destack/build";
import { LocalClient } from "@destack/daemon/client/local";
import { ReleaseChannel } from "@destack/daemon/process";
import { buildExecutable, COMPILER } from "./executable.ts";
import { buildToolchain } from "./toolchain.ts";
import { schema } from "@destack/schema";

/** Source root used only while producing the executables. */
const ROOT = fileURLToPath(new URL("../../../../", import.meta.url));

/** The package the released compiler builds and inspects. */
const TEMPLATE_STACK = join(ROOT, "@template/stack");

/** How long compiling five executables, building a package twice and starting the daemon twice may take, in milliseconds. */
const TIMEOUT_MILLISECONDS = 60000;

test.skipIf(process.platform !== "darwin" || process.arch !== "arm64")(
    "run compiled clients, persist daemon state, build with the released compiler and its toolchain, and enforce the macOS sandbox outside the checkout",
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
            for (const { source, name } of [
                { source: "@destack/cli/src/main.ts", name: "destack" },
                { source: "@destack/daemon/src/main.ts", name: "destack-daemon" },
                { source: "@destack/sandbox/src/main.ts", name: "destack-sandbox" },
                {
                    source: "@platform/release/src/distribution/tests/sandbox.ts",
                    name: "sandbox-check",
                },
            ]) {
                await buildExecutable({
                    root: ROOT,
                    entrypoint: join(ROOT, source),
                    outfile: join(directory, name),
                    target: "aarch64-apple-darwin",
                    runtime: "bun-darwin-arm64",
                    version: "2026.9.123-nightly.1",
                    channel: ReleaseChannel.of("nightly"),
                    plugins: name === "destack" ? [solidPlugin] : [],
                });
            }
            await buildExecutable({
                root: ROOT,
                entrypoint: join(ROOT, "@destack/build/src/main.ts"),
                outfile: join(directory, "destack-build"),
                target: "aarch64-apple-darwin",
                runtime: "bun-darwin-arm64",
                version: "2026.9.123-nightly.1",
                channel: ReleaseChannel.of("nightly"),
                ...COMPILER,
            });
            await buildToolchain(ROOT, "aarch64-apple-darwin", join(directory, "toolchain"));

            // run commands with no checkout, inherited credentials or installed runtime on PATH
            const cli = join(directory, "destack");
            expect(
                JSON.parse(await run([cli, "version", "--json"], directory, environment)),
            ).toEqual({
                version: "2026.9.123-nightly.1",
            });
            await run([cli, "daemon", "start", "--json"], directory, environment);
            isRunning = true;
            const repository = join(directory, "repository");
            await run(["/usr/bin/git", "init", "--quiet", repository], directory, environment);
            const created: unknown = JSON.parse(
                await run(
                    [cli, "checkout", "create", repository, "--json"],
                    directory,
                    environment,
                ),
            );
            expect(created).toMatchObject({ directory: repository });
            expect(
                JSON.parse(await run([cli, "daemon", "stop", "--json"], directory, environment)),
            ).toEqual({ stopped: true });
            isRunning = false;
            await run([cli, "daemon", "start", "--json"], directory, environment);
            isRunning = true;
            const checkouts: unknown = JSON.parse(
                await run([cli, "checkout", "list", "--json"], directory, environment),
            );
            expect(checkouts).toEqual([created]);

            // build a package with the released compiler and its toolchain alone
            const compiler = Compiler.beside(join(directory, "destack-build"));
            const outputs = await readOutputs(TEMPLATE_STACK);
            let built: unknown;

            // check and compile the package and evaluate its outputs
            {
                await using builder = await PackageBuilder.start(TEMPLATE_STACK, compiler);
                await using build = await builder.build({
                    dependencies: await readDependencies(TEMPLATE_STACK),
                    outputs,
                });
                built = [build.manifest.package.name, Object.keys(build.manifest.outputs)];
            }
            expect(built).toEqual(["@template/stack", ["bun", "workerd", "browser"]]);

            // inspect it through the compiled daemon, which starts the compiler beside it
            const registered: unknown = JSON.parse(
                await run([cli, "checkout", "create", ROOT, "--json"], directory, environment),
            );
            const { id } = Registered.parse(registered);
            const inspection = await new LocalClient(environment.DESTACK_DIRECTORY)
                .connect()
                .inspect({ checkout: id, directory: "@template/stack", output: "bun" });
            expect(inspection.name).toBe("@template/stack");

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
    TIMEOUT_MILLISECONDS,
);

/** The identifier of a registered checkout. */
const Registered = schema.object({ id: schema.identifier("checkout") }).strip();

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
