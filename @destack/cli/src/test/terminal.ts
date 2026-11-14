import { SpaceAddress } from "@destack/account/address";
import type { Login } from "@destack/account/native";
import { Device, type MachineCredential } from "@destack/machine/device";
import { type TestInstall, TestComputer, TestDistribution } from "@destack/machine/computer/test";
import { type DatabaseConnection, Snapshot } from "@destack/db";
import type { ObjectType } from "@destack/object";
import { type InstallationRollout, Rollout, rollout } from "@destack/stack/object";
import { type Identifier, type JsonObject, present, schema } from "@destack/schema";
import { onTestFinished } from "@destack/test";
import { testSimulation } from "@destack/test/world";
import type { Scheduler } from "@destack/environment";
import { type Simulation, SimulatedProcess } from "@destack/environment/simulation";
import { CommandLine } from "../command/run.ts";
import { Context } from "../context/context.ts";
import { DESKTOP_OBJECTS } from "@destack/desktop/object";

/** The node this computer's command lines and their device run as. */
const TERMINAL_NODE = "terminal";

/** The process identifier of a terminal's command lines. */
const TERMINAL_PROCESS = 1;

/** The user a terminal's command lines run as: the first regular user on Linux. */
const TERMINAL_USER = 1000;

/** How long a command line may take to print a line: 10 s, far above its milliseconds. */
const PRINT_MILLISECONDS = 10_000;

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

    /** Open a terminal on the owner's home computer, with variables and installed fixtures. */
    static async open(
        variables: Readonly<Record<string, string>> = {},
        installs: readonly TestInstall[] = [],
    ): Promise<TerminalFixture> {
        // start the owner's home with the desktop's local objects, then open its device as command lines do
        const { home, login } = await TestComputer.home({
            localObjects: DESKTOP_OBJECTS,
            installs,
        });
        const { world } = home;
        const native = await Device.open({
            home: home.directory,
            environment: world.simulation.environment(TERMINAL_NODE),
            universe: world.universe.origin,
        });
        onTestFinished(() => native[Symbol.asyncDispose]());

        // read the owner's login as the command lines read it
        const row = await native.logins.query.login.findFirst({ where: { id: login } });
        if (row === undefined) {
            throw new TypeError(`the home's device has no login '${login}'`);
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

        return this.computer.space(this.login.id, owner.account, name);
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
                script: "destack.js",
                runtime: { revision: "0".repeat(40), isStandalone: false },
            },
            scheduler,
        );
    }

    /** Publish a fixture of the run into its account's forge as the platform's operator. */
    async publish(fixture: string): Promise<void> {
        await this.computer.publish(TestDistribution.lockOf(fixture));
    }

    /** Install a published fixture under a name in a space, the owner's home by default. */
    async install(
        fixture: string,
        name: string,
        space: Identifier<"space"> = this.computer.world.owner.home,
    ): Promise<void> {
        const build = await TestDistribution.fixture(fixture);
        await this.computer.install(this.login.id, space, build, name);
    }

    /** Read an object of an installation in the owner's home as the owner, as JSON prints it. */
    async object(type: ObjectType, installation: string, id: string): Promise<JsonObject> {
        // follow the installation's objects as the owner, until the object replicates
        await using stack = new AsyncDisposableStack();
        const session = await this.native.launcher(this.login.id);
        const { home } = this.computer.world.owner;
        const objects = await this.wait(
            this.native.open(session, { space: home, installation }, { type }, stack),
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
            () => Promise.reject(new TypeError(`'${words.join(" ")}' opened the device`)),
            { ...node, process: running },
        );

        return {
            code,
            output: decodeChunks(running.stdout.chunks),
            error: decodeChunks(running.stderr.chunks),
        };
    }

    /** Run a command line on its input, returning what it printed and its exit status. */
    printed(words: readonly string[], input: TerminalInput = {}): Promise<Printed> {
        return this.start(words, input).printed;
    }

    /** Start a command line on its input as a job of the terminal, which runs until it ends. */
    start(words: readonly string[], input: TerminalInput = {}): TerminalJob {
        // run the command line on the terminal's node through the computer's client device
        const { simulation } = this.computer.world;
        const node = simulation.environment(TERMINAL_NODE);
        const process = TerminalFixture.process(this.#variables, node.scheduler, input);
        const ended = CommandLine.run(words, async () => this.native, { ...node, process });

        return new TerminalJob(process, simulation, ended);
    }

    /** List the URLs the terminal's command lines opened in the person's browser, in order. */
    opened(): readonly string[] {
        return this.computer.world.simulation.opener.opened(TERMINAL_NODE);
    }
}

/** A command line running on the terminal as a shell job: its output, interrupt and end. */
export class TerminalJob {
    /** What it printed and how it exited, once it ends. */
    readonly printed: Promise<Printed>;
    /** The process it runs as. */
    readonly #process: SimulatedProcess;
    /** The world it runs in. */
    readonly #simulation: Simulation;
    /** Whether it ended. */
    #isEnded = false;

    /** Follow a command line's process until its run ends. */
    constructor(process: SimulatedProcess, simulation: Simulation, ended: Promise<number>) {
        // read what it wrote to standard output and standard error once it ends
        this.#process = process;
        this.#simulation = simulation;
        this.printed = simulation.wait(ended).then((code) => {
            this.#isEnded = true;

            return {
                code,
                output: decodeChunks(process.stdout.chunks),
                error: decodeChunks(process.stderr.chunks),
            };
        });
    }

    /** Read what it printed to standard output so far. */
    output(): string {
        return decodeChunks(this.#process.stdout.chunks);
    }

    /** Run the world until it printed some lines or ended, refusing a late one. */
    async waitLines(count: number): Promise<void> {
        const lines = () => this.output().split("\n").length - 1;
        const deadline = this.#simulation.clock.now() + PRINT_MILLISECONDS;
        await this.#simulation.waitUntil(() => this.#isEnded || lines() >= count, deadline);
    }

    /** Interrupt it, as Ctrl-C sends SIGINT to the terminal's foreground job. */
    interrupt(): void {
        this.#process.kill(this.#process.id, "SIGINT");
    }
}

/** Read the JSON lines a watch printed, one value per line. */
export function jsonLines(output: string): unknown[] {
    return output
        .split("\n")
        .filter((each) => each !== "")
        .map((each): unknown => JSON.parse(each));
}

/** Decode the chunks a process wrote to one of its standard streams. */
function decodeChunks(chunks: readonly Uint8Array[]): string {
    return chunks.map((chunk) => new TextDecoder().decode(chunk)).join("");
}
