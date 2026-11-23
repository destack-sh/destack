import type {} from "@destack/package/import-meta";
import { COMPLETE, Completion } from "../device/completion.ts";
import { Launcher } from "../device/launcher.ts";
import type { Device, MachineCredential } from "@destack/machine/device";
import { type CommandEnvironment, CommandContext } from "./context.ts";
import { CommandTree, GLOBAL_OPTIONS, SEPARATOR } from "./command.ts";
import { root } from "./root.ts";
import { CommandFailure } from "./failure.ts";
import { CliError } from "../error/index.ts";
import { Terminal } from "../terminal/index.ts";
import { nth } from "@destack/schema";
import { TelemetryScope } from "@destack/telemetry";

/** The program's executable name, which names its root span by OpenTelemetry's CLI conventions. */
const PROGRAM = "destack";

/** The variable giving the space command lines act in without --space. */
export const SPACE_VARIABLE = "DESTACK_SPACE";

/** A command line as the terminal passes it: its global flags and the words they leave. */
export class CommandLine {
    /** The space of --space. */
    readonly space: string | undefined;
    /** Whether the output is JSON. */
    readonly isJson: boolean;
    /** Whether the line asks for the version. */
    readonly isVersion: boolean;
    /** The words of the command, its arguments and its options. */
    readonly words: readonly string[];

    /** Take a command line's global flags and the words they leave. */
    constructor(fields: Pick<CommandLine, "space" | "isJson" | "isVersion" | "words">) {
        // set the space, the output, the version flag and the words
        this.space = fields.space;
        this.isJson = fields.isJson;
        this.isVersion = fields.isVersion;
        this.words = fields.words;
    }

    /** Read a command line's global flags, leaving the words of the command. */
    static read(words: readonly string[]): CommandLine {
        // read each word, passing on the ones no global flag takes
        const { space: spaceOption, json, version } = GLOBAL_OPTIONS;
        let space: string | undefined;
        let isJson = false;
        let isVersion = false;
        const remaining: string[] = [];
        for (let index = 0; index < words.length; index++) {
            const word = nth(words, index);

            // pass on everything after the separator
            if (word === SEPARATOR) {
                remaining.push(...words.slice(index));
                break;
            }
            // read the space as a separate value
            else if (word === spaceOption.flag) {
                space = words[index + 1];
                index++;
                if (space === undefined) {
                    throw CliError.of("USAGE_INVALID", "--space is invalid: it takes a value");
                }
            }
            // read the space as an attached value
            else if (word.startsWith(`${spaceOption.flag}=`)) {
                space = word.slice(spaceOption.flag.length + 1);
            }
            // read JSON output
            else if (word === json.flag) {
                isJson = true;
            }
            // read the version
            else if (word === version.flag || word === version.short) {
                isVersion = true;
            }
            // pass on the other words
            else {
                remaining.push(word);
            }
        }

        return new CommandLine({ space, isJson, isVersion, words: remaining });
    }

    /** Run a command line through this computer's native device and return a shell exit status. */
    static async run(
        words: readonly string[],
        open: () => Promise<Device<MachineCredential>>,
        environment: CommandEnvironment,
    ): Promise<number> {
        // only recognize flags before the positional argument separator
        const separator = words.indexOf(SEPARATOR);
        const flags = separator < 0 ? words : words.slice(0, separator);
        try {
            // route launcher failures through the same command error reporting
            const forwarded = await Launcher.forward(words, environment);
            if (forwarded !== undefined) {
                return forwarded;
            }

            // answer a completion script's candidates for the typed words
            const line = CommandLine.read(words[0] === COMPLETE ? words.slice(1) : words);
            const space = line.space ?? environment.process.variables[SPACE_VARIABLE];
            await using context = new CommandContext(open, space, environment);
            await using terminal = context.terminal;
            const tree = new CommandTree(root, context);
            if (words[0] === COMPLETE) {
                for (const candidate of await Completion.candidates(tree, line.words)) {
                    await terminal.print(Completion.line(candidate));
                }
            }
            // print the version
            else if (line.isVersion) {
                if (line.words.length > 0) {
                    throw CliError.of(
                        "USAGE_INVALID",
                        "usage is invalid: use --version without a command",
                    );
                }
                const version = import.meta.destack.package.version;
                await terminal.print(line.isJson ? JSON.stringify({ version }) : version);
            }
            // run the command of the words in the program's span, as OpenTelemetry's CLI conventions name it
            else {
                const { span } = TelemetryScope.of(
                    import.meta.destack.package,
                    environment.telemetry,
                );
                const program = { "process.executable.name": PROGRAM };
                await span(PROGRAM, program, () => tree.execute(line.words, line.isJson), {
                    duration: { attributes: program },
                });
            }

            return 0;
        } catch (error) {
            return await new CommandFailure(error).report(
                flags.includes(GLOBAL_OPTIONS.json.flag),
                new Terminal(environment.process, environment),
            );
        }
    }
}
