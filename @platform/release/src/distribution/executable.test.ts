import { Bookmark } from "@destack/object/client";
import { expect, test } from "@destack/test";
import { testHost } from "@destack/test/host";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { Compiler, CompilerCommand, CompilerPool, outputsOf, Workspace } from "@destack/build";
import { testStore } from "@destack/build/test";
import machinePackage from "@destack/machine/package";
import { Loopback } from "@destack/machine/computer";
import { RESIDENCIES } from "@platform/stack";
import { Stage } from "@destack/update/release";
import { buildExecutable, toolchainOptions } from "./executable.ts";
import { buildToolchain, writeToolchainManifest } from "./toolchain.ts";
import { Toolchain } from "@destack/check/toolchain";
import { DISTRIBUTION_DIRECTORY, TOOLCHAIN_DIRECTORY } from "@destack/update";
import { schema } from "@destack/schema";
import { TestDatabase } from "@destack/db/test";
import { LocalUniverse } from "../local/index.ts";
import { PrintedMail } from "../local/mail.ts";

/** Source root used only while producing the executables. */
const ROOT = fileURLToPath(new URL("../../../..", import.meta.url));

/** The package the released compiler builds and inspects. */
const TEMPLATE_STACK = join(ROOT, "@destack/template-stack");

/** The keychain an isolated home keeps the machine's keys in, as a user's login keychain. */
const KEYCHAIN = "login.keychain";

/** The store keeping the universe's releases across runs, so warm runs skip their build. */
const RELEASES = join(ROOT, "target", "test", "releases");

/** The database of the loopback universe, dropped before and after each run. */
const DATABASE = "destack_executable";

/** The person signing up on the compiled machine. */
const PERSON = { email: "ada@destack.test", handle: "ada", residency: "eu", name: "Ada" };

/** How long building the releases cold, compiling and the scenario may take, in milliseconds. */
const TIMEOUT_MILLISECONDS = 600_000;

/** The real host the executables compile and run on. */
const HOST = testHost();

test.skipIf(HOST.host.os !== "darwin" || HOST.host.architecture !== "arm64")(
    "run compiled clients, persist machine state, build with the released compiler and its toolchain, and enforce the macOS sandbox outside the checkout",
    async () => {
        // remove the isolated home, then drop the universe's database, after everything stopped
        const { files } = HOST;
        await using cleanup = new AsyncDisposableStack();
        const directory = await files.realPath(
            await files.makeTemporaryDirectory("destack-executable-"),
        );
        cleanup.defer(() => files.remove(directory, { recursive: true, force: true }));
        const server = new URL(TestDatabase.address());
        server.pathname = "";
        const database = { server: server.href, database: DATABASE, environment: HOST };
        await LocalUniverse.drop(database);
        cleanup.defer(() => LocalUniverse.drop(database));

        // start a loopback universe with its cloud machines over the releases built from the checkout
        const machines: Record<string, number> = {};
        for (const residency of RESIDENCIES) {
            machines[residency.code] = await freePort();
        }
        const layout = { universe: await freePort(), machine: await freePort(), machines };
        const mailbox = new Mailbox();
        const development = cleanup.use(
            await LocalUniverse.start({
                ...database,
                root: ROOT,
                directory: join(directory, "universe"),
                releases: RELEASES,
                layout,
                print: (name, line) => mailbox.receive(name, line),
            }),
        );

        // run the machine of an isolated home against the universe
        const variables = {
            PATH: "/usr/bin:/bin",
            HOME: directory,
            DESTACK_DIRECTORY: join(directory, "state"),
            DESTACK_UNIVERSE: development.deployment.universeOrigin,
            DESTACK_PORT: String(await freePort()),
        };
        const cli = join(directory, "destack");
        let isRunning = false;
        cleanup.defer(async () => {
            if (isRunning) {
                await run([cli, "self", "stop", "--json"], directory, variables);
            }
        });

        // give the isolated home its own keychain, which the machine keeps its keys in from its start
        await isolateKeychain(directory);

        // compile the real programs for the local stage and a consumer of the packaged sandbox API
        for (const { source, name } of [
            { source: "@destack/cli/src/main.ts", name: "destack" },
            { source: "@destack/desktop/src/machine.ts", name: "destack-machine" },
            { source: "@destack/sandbox/src/main.ts", name: "destack-sandbox" },
            {
                source: "@platform/release/src/distribution/tests/sandbox.ts",
                name: "sandbox-check",
            },
        ]) {
            await buildExecutable(
                {
                    root: ROOT,
                    entrypoint: join(ROOT, source),
                    outfile: join(directory, name),
                    target: "aarch64-apple-darwin",
                    runtime: "bun-darwin-arm64",
                    version: machinePackage.version,
                    stage: Stage.of("local"),
                    ...(name === "destack-machine" ? toolchainOptions(files) : {}),
                },
                HOST,
            );
        }
        await buildExecutable(
            {
                root: ROOT,
                entrypoint: join(ROOT, "@destack/build/src/main.ts"),
                outfile: join(directory, "destack-build"),
                target: "aarch64-apple-darwin",
                runtime: "bun-darwin-arm64",
                version: machinePackage.version,
                stage: Stage.of("local"),
                ...toolchainOptions(files),
            },
            HOST,
        );
        const tools = join(directory, TOOLCHAIN_DIRECTORY);
        const toolchain = Toolchain.at(tools);
        await buildToolchain(
            ROOT,
            "aarch64-apple-darwin",
            { resources: directory, toolchain: tools },
            HOST,
        );
        await writeToolchainManifest(tools, directory, HOST.host.architecture, HOST);

        // ship the distribution the universe's machines ship, the desktop's own release among it
        await files.copy(
            join(directory, "universe", "distribution"),
            join(directory, DISTRIBUTION_DIRECTORY),
        );

        // run commands with no checkout, inherited credentials or installed runtime on PATH
        expect(JSON.parse(await run([cli, "--version", "--json"], directory, variables))).toEqual({
            version: machinePackage.version,
        });
        await run([cli, "self", "start", "--json"], directory, variables);
        isRunning = true;

        // sign a new person up with the code the universe mails, enrolling the machine
        const signedIn: unknown = JSON.parse(await signUp(cli, directory, variables, mailbox));
        expect(signedIn).toMatchObject({ login: { name: PERSON.name }, space: "home--ada" });

        // keep a checkout across a restart of the enrolled machine
        const repository = join(directory, "repository");
        await run(["/usr/bin/git", "init", "--quiet", repository], directory, variables);
        const created: unknown = JSON.parse(
            await run(
                [
                    cli,
                    "checkout",
                    "create",
                    "--directory",
                    repository,
                    "--source-kind",
                    "existing",
                    "--json",
                ],
                directory,
                variables,
            ),
        );
        expect(created).toMatchObject({ directory: repository });
        expect(
            JSON.parse(await run([cli, "self", "stop", "--json"], directory, variables)),
        ).toEqual({ stopped: true });
        isRunning = false;
        await run([cli, "self", "start", "--json"], directory, variables);
        isRunning = true;
        const checkouts: unknown = JSON.parse(
            await run([cli, "checkouts", "--json"], directory, variables),
        );
        expect(checkouts).toEqual([created]);

        // build a package with the released compiler and its toolchain alone
        const compiler = CompilerCommand.beside(join(directory, "destack-build"), toolchain, HOST);
        let built: unknown;

        // check and compile the package on its closure's unit builds and evaluate its outputs
        {
            await using store = await testStore();
            await using compilers = await CompilerPool.open(store, async () =>
                Toolchain.workspace(),
            );
            const workspace = await Workspace.open(TEMPLATE_STACK, compilers);
            const outputs = outputsOf(workspace.root.packageJson);
            const dependencies = await workspace.storeDependencies({});
            await using builder = await Compiler.start(TEMPLATE_STACK, {
                command: compiler,
                store,
                toolchain: async () => toolchain,
            });
            const build = await builder.build({
                directory: TEMPLATE_STACK,
                dependencies,
                outputs,
            });
            built = [build.manifest.package.name, Object.keys(build.manifest.outputs)];
        }
        expect(built).toEqual(["@destack/template-stack", ["browser", "bun", "workerd"]]);

        // inspect it through the compiled machine, which starts the compiler beside it
        const registered: unknown = JSON.parse(
            await run(
                [
                    cli,
                    "checkout",
                    "create",
                    "--directory",
                    ROOT,
                    "--source-kind",
                    "existing",
                    "--json",
                ],
                directory,
                variables,
            ),
        );
        const { id, scope } = Registered.parse(registered);
        const inspection = await new Loopback(variables.DESTACK_DIRECTORY, HOST, new Bookmark())
            .connect(machinePackage)
            .checkout.inspect({
                machineId: scope,
                id,
                directory: "@destack/template-stack",
                output: "bun",
            })
            .catch(async (error: unknown) => {
                // name the failure with the machine's log, which the test's directory takes with it
                const log = await files.readTextFile(join(directory, "state", "machine.log"));
                throw new Error(`inspect failed; the machine logged:\n${log}`, { cause: error });
            });
        expect(inspection.name).toBe("@destack/template-stack");

        // enforce the same restrictions through the compiled parent and launcher
        await files.writeTextFile(join(directory, "readable.txt"), "public");
        await files.writeTextFile(join(directory, "private.txt"), "private");
        const sandbox = await run(
            [join(directory, "sandbox-check"), directory],
            directory,
            variables,
        );
        expect(JSON.parse(sandbox)).toEqual({ allowed: "public", denied: "EPERM" });
    },
    TIMEOUT_MILLISECONDS,
);

/** The identifier of a registered checkout. */
const Registered = schema
    .object({ id: schema.identifier("checkout"), scope: schema.identifier("machine") })
    .strip();

/** The lines the loopback universe prints, with the sign-in code it mails. */
class Mailbox {
    /** The lines printed so far, each under its process's name. */
    readonly lines: string[] = [];
    /** The universe's printed mail, which sign-ins read their codes from. */
    readonly mail = new PrintedMail();

    /** Keep a printed line, and a mail's line apart. */
    receive(name: string, line: string): void {
        this.lines.push(`${name}: ${line}`);
        if (name === "mail") {
            this.mail.push(line);
        }
    }
}

/** Sign a new person up through the compiled CLI, typing back the code the universe mails. */
async function signUp(
    cli: string,
    directory: string,
    variables: Record<string, string>,
    mailbox: Mailbox,
): Promise<string> {
    // ask for a mailed code as a new person in a residency, reading the mail printed after asking
    const { email, handle, residency, name } = PERSON;
    const asked = mailbox.mail.lines.length;
    const child = HOST.process.spawn(
        [
            cli,
            "login",
            "--email",
            email,
            "--handle",
            handle,
            "--residency",
            residency,
            "--name",
            name,
            "--json",
        ],
        { workingDirectory: directory, variables, stdin: "pipe", stdout: "pipe", stderr: "pipe" },
    );
    const printed = Promise.all([
        new Response(child.stdout).text(),
        new Response(child.stderr).text(),
    ]);

    // type the code back once the universe mails it, until the CLI exits
    void mailbox.mail.code(asked, HOST.clock).then(async (code) => {
        const writer = child.stdin.getWriter();
        await writer.write(new TextEncoder().encode(`${code}\n`));
        await writer.close();
    });
    const [exited, [output, error]] = await Promise.all([child.exited, printed]);
    expect(exited, `login: ${error}\n${mailbox.lines.join("\n")}`).toBe(0);

    return output;
}

/** Find a free loopback port, so the test's machine runs beside one serving this computer. */
async function freePort(): Promise<number> {
    const probe = HOST.server.http({ hostname: "127.0.0.1", port: 0, fetch: () => new Response() });
    const port = Number(probe.url.port);
    await probe.stop();

    return port;
}

/** Create an unlocked default keychain in an isolated home, leaving the user's keychains alone. */
async function isolateKeychain(home: string): Promise<void> {
    await HOST.files.mkdir(join(home, "Library", "Keychains"), { recursive: true });
    const variables = { PATH: "/usr/bin:/bin", HOME: home };
    for (const command of [
        ["create-keychain", "-p", "", KEYCHAIN],
        ["default-keychain", "-d", "user", "-s", KEYCHAIN],
        ["unlock-keychain", "-p", "", KEYCHAIN],
    ]) {
        await run(["/usr/bin/security", ...command], home, variables);
    }
}

/** Execute one compiled command with retained diagnostics, bounded by the test's timeout. */
async function run(
    command: string[],
    directory: string,
    variables: Record<string, string>,
): Promise<string> {
    const child = HOST.process.spawn(command, {
        workingDirectory: directory,
        variables,
        stdout: "pipe",
        stderr: "pipe",
    });
    const [code, output, error] = await Promise.all([
        child.exited,
        new Response(child.stdout).text(),
        new Response(child.stderr).text(),
    ]);

    // name the failure with the machine's log, which the test's directory takes with it
    const log = join(directory, "state", "machine.log");
    const logged =
        code !== 0 && (await HOST.files.exists(log)) ? await HOST.files.readTextFile(log) : "";
    expect(code, `${command.join(" ")}: ${error}\n${logged}`).toBe(0);

    return output;
}
