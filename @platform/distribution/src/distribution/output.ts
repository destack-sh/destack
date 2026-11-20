import { Environment, type HostCapability, LineExporter, Telemetry } from "@destack/environment";

/** The width of the status lines' name column: the longest service, `eu-central-tail`. */
const NAME_WIDTH = 15;

/** A command: it runs on the host with the words after its noun and verb. */
export type Command = (
    environment: Environment<HostCapability>,
    arguments_: readonly string[],
) => Promise<void>;

/** The verbs on commands. */
export const Command = {
    /** Run the command the command line's noun and verb select, printing its records. */
    async run(
        commands: Readonly<Record<string, Command>>,
        name: string,
        system: Environment<HostCapability>,
    ): Promise<void> {
        // read the noun and verb, and the host printing each record as a status line under the noun
        const [noun, verb, ...arguments_] = system.process.arguments;
        const host = Environment.instrument(
            system,
            Telemetry.local(
                system.clock,
                (line) => printStatus(noun ?? name, line.trimEnd(), system),
                system.telemetry.context,
                LineExporter.options(system.process.variables),
            ),
        );

        // run the selected command, refusing an unknown noun and verb
        const selected = commands[`${noun} ${verb}`];
        if (selected === undefined) {
            throw new Error(`use '${Object.keys(commands).join(", ")}'`);
        }
        await selected(host, arguments_);
    },
};

/** Write a line of a command's output to standard output. */
export function print(text: string, environment: Environment<"process">): void {
    environment.process.stdout.writeSync(`${text}\n`);
}

/** Write a status line under a name to standard output. */
export function printStatus(name: string, line: string, environment: Environment<"process">): void {
    environment.process.stdout.writeSync(`${name.padEnd(NAME_WIDTH)}| ${line}\n`);
}
