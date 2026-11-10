import type {} from "@destack/package/import-meta";
import { completionCommand, selfCommand } from "../distribution/command.ts";
import { COMPLETE, Completion } from "../distribution/completion.ts";
import { Launcher } from "../distribution/launcher.ts";
import type { Device, MachineCredential } from "@destack/machine/device";
import { type CliEnvironment, Context } from "../context/context.ts";
import { loginCommand, logoutCommand, whoamiCommand } from "../context/login.ts";
import { switchCommand } from "../context/space.ts";
import { nounCommand } from "../object/command.ts";
import { settingCommand, settingsCommand } from "../object/setting.ts";
import { applyCommand, approveCommand, planCommand, rejectCommand } from "../workflow/apply.ts";
import { buildCommand, devCommand, publishCommand } from "../workflow/dev.ts";
import { tagCommand } from "../workflow/tag.ts";
import { installCommand } from "../workflow/install.ts";
import { openCommand } from "../workflow/open.ts";
import { CommandTree, GLOBAL_OPTIONS, type Group, SEPARATOR } from "./command.ts";
import { CommandFailure } from "./error.ts";
import { CliError } from "../error/index.ts";
import { Terminal } from "../output/index.ts";
import { nth } from "@destack/schema";

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
                    throw CliError.of("USAGE_INVALID", "--space takes a value");
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

    /** Build the command tree. */
    static tree(context: Context): Group {
        // build the root verbs
        const commands = {
            // the person and the space the terminal acts in
            login: loginCommand(context),
            logout: logoutCommand(context),
            whoami: whoamiCommand(context),
            switch: switchCommand(context),

            // the nouns taking more than their type's methods
            setting: settingCommand(context),
            settings: settingsCommand(context),

            // the space's packages, and its stack
            install: installCommand(context),
            apply: applyCommand(context),
            plan: planCommand(context),
            approve: approveCommand(context),
            reject: rejectCommand(context),

            // this computer's checkouts
            dev: devCommand(context),
            build: buildCommand(context),
            publish: publishCommand(context),
            tag: tagCommand(context),

            // the desktop, this distribution and its completion
            open: openCommand(context),
            self: selfCommand(context),
            completion: completionCommand(context),
        };

        return {
            description: `Destack ${import.meta.destack.package.version}: destack <nouns> to list, destack <noun> <id> [method] to get and call`,
            commands,
            // find a noun the stores describe, listed qualified where a root verb takes its word
            resolve: async (word) => nounCommand(context, await context.catalog(), word),
            names: async () => (await context.catalog()).plurals(Object.keys(commands)),
            unavailable: () => CommandLine.#unavailable(context),
        };
    }

    /** Explain the nouns a command line lacks: signed out, or outside a space. */
    static async #unavailable(context: Context): Promise<string | undefined> {
        // point a signed-out person at signing in, and one outside a space at switching to one
        if ((await context.me()) === undefined) {
            return "sign in with destack login for the universe's and spaces' commands";
        } else if ((await context.space()) === undefined) {
            return "switch to a space with destack switch <space> for its commands";
        }

        return undefined;
    }

    /** Run a command line through this computer's native device and return a shell exit status. */
    static async run(
        words: readonly string[],
        open: () => Promise<Device<MachineCredential>>,
        environment: CliEnvironment,
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
            const context = new Context(open, space, environment);
            await using terminal = context.terminal;
            const tree = new CommandTree(CommandLine.tree(context));
            if (words[0] === COMPLETE) {
                for (const candidate of await Completion.candidates(tree, line.words)) {
                    await terminal.print(Completion.line(candidate));
                }
            }
            // print the version
            else if (line.isVersion) {
                if (line.words.length > 0) {
                    throw CliError.of("USAGE_INVALID", "use --version without a command");
                }
                const version = import.meta.destack.package.version;
                await terminal.print(line.isJson ? JSON.stringify({ version }) : version);
            }
            // run the command of the words, or show help without one
            else {
                await tree.execute(line.words, line.isJson, terminal);
            }

            return 0;
        } catch (error) {
            return await new CommandFailure(error).report(
                flags.includes(GLOBAL_OPTIONS.json.flag),
                new Terminal(environment.process, environment.timeZone),
            );
        }
    }
}
