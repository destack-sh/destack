import { SpaceAddress } from "@destack/account/address";
import type { Login } from "@destack/account/native";
import { Device, type MachineCredential } from "@destack/machine/device";
import { TestComputer, WorldDriver } from "@destack/machine/test";
import { type DatabaseConnection, Snapshot } from "@destack/db";
import type { ObjectType } from "@destack/object";
import { type InstallationRollout, Rollout, rollout } from "@destack/stack/object";
import { type Identifier, type JsonObject, present, schema } from "@destack/schema";
import { onTestFinished } from "@destack/test";
import { testSimulation } from "@destack/test/world";
import type { Scheduler } from "@destack/environment";
import { SimulatedProcess } from "@destack/environment/simulation";
import { CommandLine } from "../command/run.ts";
import { Context } from "../context/context.ts";
import { DESKTOP_OBJECTS } from "@destack/desktop/object";

/** The node this computer's command lines and their device run as. */
const TERMINAL_NODE = "terminal";

/** The process identifier of a terminal's command lines. */
const TERMINAL_PROCESS = 1;

/** The user a terminal's command lines run as: the first regular user on Linux. */
const TERMINAL_USER = 1000;

/** How long a rollout may take to meet a check: 2 min, a space pulling a closure in ~30 s. */
const ROLLOUT_MILLISECONDS = 120_000;

/** The input a command line runs on: its standard input and whether it runs on a terminal. */
export interface TerminalInput {
    /** The text or stream standard input reads, none by default. */
    readonly stdin?: string | ReadableStream<Uint8Array>;
    /** Whether the command line runs on a terminal, as an interactive shell runs it. */
    readonly isTerminal?: boolean;
}

/** What a command line printed and how it exited. */
export interface Printed {
    /** The exit status. */
    readonly code: number;
    /** The standard output. */
    readonly output: string;
    /** The standard error. */
    readonly error: string;
}

/** A terminal on the computer serving the owner's home, its command lines calling its machine. */
export class TerminalFixture {
    /** The computer serving the owner's home, the owner signed in on its device. */
    readonly computer: TestComputer;
    /** The computer's device as its command lines open it, a client of its machine. */
    readonly native: Device<MachineCredential>;
    /** The owner's login on the computer. */
    readonly login: Login["row"];
    /** The environment variables each command line runs with. */
    readonly #variables: Readonly<Record<string, string>>;

    /** Take the computer, its client device and the owner's login. */
    private constructor(
        computer: TestComputer,
        native: Device<MachineCredential>,
        login: Login["row"],
        variables: Readonly<Record<string, string>>,
    ) {
        // set the computer, its client device, the owner's login and the variables
        this.computer = computer;
        this.native = native;
        this.login = login;
        this.#variables = variables;
    }

    /** Open a terminal on the owner's home computer, with the command lines' variables. */
    static async open(variables: Readonly<Record<string, string>> = {}): Promise<TerminalFixture> {
        // start the owner's home with the desktop's local objects, then open its device as command lines do
        const { home, login } = await TestComputer.home({ localObjects: DESKTOP_OBJECTS });
        const { world } = home;
        const native = await Device.open({
            home: home.directory,
            environment: world.simulation.environment(TERMINAL_NODE),
            issuer: world.universe.origin,
        });
        onTestFinished(() => native[Symbol.asyncDispose]());

        // read the owner's login as the command lines read it
        const row = await native.logins.query.login.findFirst({ where: { id: login } });
        if (row === undefined) {
            throw new TypeError(`the home's device has no login ${login}`);
        }

        return new TerminalFixture(home, native, row, variables);
    }

    /** Read the address of one of the owner's spaces by its name. */
    address(space: string): SpaceAddress {
        const { handle } = this.computer.world.owner;

        return SpaceAddress.parse(SpaceAddress.of({ space, handle }));
    }

    /** Create a space of the owner's on the computer, waiting until it serves it. */
    space(name: string): Promise<Identifier<"space">> {
        const { owner } = this.computer.world;

        return this.computer.space(this.login, owner.account, name);
    }

    /** Make a terminal's process with only some environment variables, writing into its chunks. */
    static process(
        variables: Readonly<Record<string, string>>,
        scheduler: Scheduler,
        input: TerminalInput = {},
    ): SimulatedProcess {
        return new SimulatedProcess(
            {
                stdin:
                    input.stdin instanceof ReadableStream
                        ? input.stdin
                        : new TextEncoder().encode(input.stdin ?? ""),
                isTerminal: input.isTerminal ?? false,
                arguments: [],
                variables,
                workingDirectory: "/",
                id: TERMINAL_PROCESS,
                userId: TERMINAL_USER,
                executable: "destack",
                runtime: { revision: "0".repeat(40), isStandalone: false },
            },
            scheduler,
        );
    }

    /** Publish a fixture of the run into its account's forge from the home's machine. */
    async publish(fixture: string): Promise<void> {
        const build = await WorldDriver.fixture(fixture);
        await this.computer.world.distribution.publish(
            build,
            this.computer.machine,
            this.computer.machine,
        );
    }

    /** Install a published fixture in the owner's home under a name, until applied. */
    async install(fixture: string, name: string): Promise<void> {
        const { owner } = this.computer.world;
        const build = await WorldDriver.fixture(fixture);
        await this.computer.install(this.login.id, owner.home, build, name);
    }

    /** Read an object of an installation in the owner's home as the owner, as JSON prints it. */
    async object(type: ObjectType, installation: string, id: string): Promise<JsonObject> {
        // follow the installation's objects as the owner, until the object replicates
        await using stack = new AsyncDisposableStack();
        const session = await this.native.session(this.login.id);
        const { home } = this.computer.world.owner;
        const objects = await this.wait(
            session.open({ space: home, installation }, { type }, stack),
        );
        const read = () => objects.query.type.findFirst({ where: { id } });
        await this.computer.waitUntil(
            objects.database,
            async () => (await read()) !== undefined,
            ROLLOUT_MILLISECONDS,
        );
        const row = present(await read(), `the ${type.name} ${id}`);

        return schema.jsonObject().parse(JSON.parse(JSON.stringify(row)));
    }

    /** Wait until an installation's rollout in the owner's home meets a check, and read it. */
    async rollout(
        installation: Identifier<"installation">,
        isMet: (target: InstallationRollout) => boolean,
    ): Promise<InstallationRollout> {
        // wait at each commit of the home's database until the rollout meets the check
        const { home } = this.computer.world.owner;
        const { database } = await this.computer.machine.space(home);
        await this.computer.waitUntil(
            database,
            async () => {
                const joined = await TerminalFixture.#rollout(database, installation);

                return joined !== undefined && isMet(joined);
            },
            ROLLOUT_MILLISECONDS,
        );

        return present(
            await TerminalFixture.#rollout(database, installation),
            `the rollout of ${installation}`,
        );
    }

    /** Read an installation's rollout with its installation from a space's database. */
    static async #rollout(
        database: DatabaseConnection,
        installation: Identifier<"installation">,
    ): Promise<InstallationRollout | undefined> {
        // read the rollout, then join its installation
        const snapshot = Snapshot.live(database);
        const rows = await snapshot
            .read(rollout.table)
            .findMany({ where: { installationId: installation } });
        const [joined] = await Rollout.read(snapshot, rows);

        return joined;
    }

    /** Open a command line's context acting in a space, as a command line opens it. */
    context(space: string | undefined): Context {
        // run on the terminal's node with a process of the terminal's variables
        const { simulation } = this.computer.world;
        const node = simulation.environment(TERMINAL_NODE);
        const process = TerminalFixture.process(this.#variables, node.scheduler);

        return new Context(async () => this.native, space, { ...node, process });
    }

    /** Wait for work the test started outside a command line, running the world meanwhile. */
    wait<Value>(work: Promise<Value>): Promise<Value> {
        return this.computer.world.simulation.wait(work);
    }

    /** Run a mail sign-in's command line, typing the answers, then a line for the mailed code. */
    signIn(
        words: readonly string[],
        email: string,
        answers: readonly string[],
        type: (code: string) => string,
    ): Promise<Printed> {
        // type each answer as the command line reads it, then the line for the last code mailed
        const { mail } = this.computer.world.universe;
        const lines = [...answers];
        const keyboard = new ReadableStream<Uint8Array>(
            {
                async pull(controller) {
                    const mailed = (await mail.codes()).filter((code) => code.email === email);
                    const line = lines.shift() ?? type(present(mailed.at(-1), "a code").otp);
                    controller.enqueue(new TextEncoder().encode(`${line}\n`));
                },
            },
            { highWaterMark: 0 },
        );

        return this.printed(words, { stdin: keyboard });
    }

    /** Run a command line with no machine behind its device, as the commands needing none run. */
    static async printed(words: readonly string[], input: TerminalInput = {}): Promise<Printed> {
        // run the command line in a world of its own, refusing to open a device
        const node = testSimulation().environment(TERMINAL_NODE);
        const running = TerminalFixture.process({}, node.scheduler, input);
        const code = await CommandLine.run(
            words,
            () => Promise.reject(new TypeError(`${words.join(" ")} opened the device`)),
            { ...node, process: running },
        );

        return {
            code,
            output: decodeChunks(running.stdout.chunks),
            error: decodeChunks(running.stderr.chunks),
        };
    }

    /** Run a command line on its input, returning what it printed and its exit status. */
    async printed(words: readonly string[], input: TerminalInput = {}): Promise<Printed> {
        // run the command line on the terminal's node through the computer's client device
        const { simulation } = this.computer.world;
        const node = simulation.environment(TERMINAL_NODE);
        const running = TerminalFixture.process(this.#variables, node.scheduler, input);
        const environment = { ...node, process: running };
        const code = await simulation.wait(
            CommandLine.run(words, async () => this.native, environment),
        );

        // read what it wrote to standard output and standard error
        return {
            code,
            output: decodeChunks(running.stdout.chunks),
            error: decodeChunks(running.stderr.chunks),
        };
    }
}

/** Decode the chunks a process wrote to one of its standard streams. */
function decodeChunks(chunks: readonly Uint8Array[]): string {
    return chunks.map((chunk) => new TextDecoder().decode(chunk)).join("");
}
