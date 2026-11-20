import { type Environment, FileSystemError } from "@destack/environment";
import { parseArgs } from "node:util";
import { type JsonObject, nth, schema } from "@destack/schema";

import type { Terminal } from "../output/index.ts";
import type { CommandContext } from "./context.ts";
import { CliError } from "../error/index.ts";
import { MachineError } from "@destack/machine/error";

/** The options every command line reads before its command's. */
export const GLOBAL_OPTIONS = {
    help: { flag: "--help", short: "-h", description: "Show help" },
    json: { flag: "--json", description: "Print JSON" },
    space: { flag: "--space", description: "Act in a space, DESTACK_SPACE by default" },
    version: { flag: "--version", short: "-V", description: "Print the installed version" },
} as const satisfies Readonly<Record<string, GlobalOption>>;

/** The value reading a JSON option from standard input. */
export const STDIN_VALUE = "-";

/** The prefix reading a JSON option from a file. */
export const FILE_PREFIX = "@";

/** The separator after which every word is a positional argument, as POSIX's utility syntax. */
export const SEPARATOR = "--";

/** The word help lists a group's subcommands of no journey under: the space's own nouns. */
const OTHER_JOURNEY = "objects";

/** The widest line help writes, in characters: the width terminals open at. */
const HELP_WIDTH = 80;

/** The characters a journey's line puts around its word: two before and two after. */
const JOURNEY_INDENT = 4;

/** The least width a help line pads its options to, in characters. */
const OPTION_WIDTH = 24;

/** The spaces between the widest option and its description, in characters. */
const OPTION_GAP = 2;

/** The most width a help line pads its options to: half the help width. */
const OPTION_MAX_WIDTH = HELP_WIDTH / 2;

/** An option every command line takes: its flag, its short flag and its line in help. */
interface GlobalOption {
    /** The long flag, such as --json. */
    readonly flag: string;
    /** The short flag, such as -h, absent for none. */
    readonly short?: string;
    /** The option's line in help. */
    readonly description: string;
}

/** A command of the command line: its subcommands, arguments, options and run, as cobra's. */
export interface Command {
    /** One line shown in help. */
    readonly description: string;
    /** The positional arguments in order, the optional ones last. */
    readonly arguments?: readonly CommandArgument[];
    /** The argument taking every word after the positional ones, such as test files. */
    readonly rest?: Pick<CommandArgument, "name" | "usage">;
    /** The command's options, by name. */
    readonly options?: Readonly<Record<string, CommandOption>>;
    /** Whether the command prints JSON with --json. */
    readonly isJson?: boolean;
    /** Run with the arguments and options the command line gives, help when absent. */
    readonly run?: (call: CommandCall, context: CommandContext) => Promise<void>;
    /** The named subcommands, in help order. */
    readonly commands?: Readonly<Record<string, Command>>;
    /** Load the subcommand of a word outside `commands`, such as a noun's. */
    readonly resolve?: (word: string, context: CommandContext) => Promise<Command | undefined>;
    /** List the words outside `commands` that `resolve` loads, for help and completion. */
    readonly names?: (context: CommandContext) => Promise<readonly string[]>;
    /** Explain why `resolve` loads no subcommand now, such as for a signed-out person. */
    readonly unavailable?: (context: CommandContext) => Promise<string | undefined>;
    /** The value leading to a subcommand of any value, such as an object's id. */
    readonly target?: CommandTarget;
    /** The journeys help lists the subcommands under, a line each, the other words last. */
    readonly journeys?: readonly CommandJourney[];
    /** Whether help and completion leave the command out. */
    readonly isHidden?: boolean;
}

/** A journey of a command's help: its word and the subcommands a person takes on it. */
export interface CommandJourney {
    /** The journey's one word, such as build. */
    readonly title: string;
    /** The words of its subcommands, in the order a person takes them. */
    readonly commands: readonly string[];
}

/** The value leading to a subcommand of any value, such as an object's id. */
export interface CommandTarget {
    /** The value's name in help, such as id. */
    readonly name: string;
    /** Build the subcommand of a value. */
    readonly command: (value: string) => Command;
    /** List the values completion offers. */
    readonly complete?: (context: CommandContext) => Promise<readonly string[]>;
}

/** A positional argument of a command. */
export interface CommandArgument {
    /** The argument's name in the call, and in help without a usage. */
    readonly name: string;
    /** The argument's placeholder in help, such as link|id. */
    readonly usage?: string;
    /** The value of an omitted argument, absent for a required one. */
    readonly default?: string;
    /** List the values completion offers. */
    readonly complete?: (context: CommandContext) => Promise<readonly string[]>;
    /** Whether the argument is a directory the shell completes. */
    readonly isDirectory?: boolean;
}

/** One named option of a command. */
export interface CommandOption {
    /** The option's value type. */
    readonly type: "boolean" | "string";
    /** The option's line in help. */
    readonly description: string;
    /** The value's name in help. */
    readonly argument?: string;
    /** Whether help and completion leave the option out. */
    readonly isHidden?: boolean;
    /** The values completion offers. */
    readonly choices?: readonly string[];
}

/** A command's run: its arguments, its options and whether it prints JSON. */
export interface CommandCall {
    /** The positional arguments by name, an omitted optional one absent. */
    readonly arguments: Readonly<Record<string, string | undefined>>;
    /** The words after the positional arguments, for a command with a rest argument. */
    readonly rest: readonly string[];
    /** The options given, by name. */
    readonly options: Readonly<Record<string, string | boolean | undefined>>;
    /** Whether the command prints JSON. */
    readonly isJson: boolean;
}

/** The command tree below the program's name: its root and the words leading to its commands. */
export class CommandTree {
    /** The root command. */
    readonly root: Command;
    /** The context the commands run in. */
    readonly context: CommandContext;

    /** Read command lines through a root, in a context. */
    constructor(root: Command, context: CommandContext) {
        this.root = root;
        this.context = context;
    }

    /** Find a command's subcommand of a word: a named one, a loaded one, or its target's. */
    async child(command: Command, word: string): Promise<Command | undefined> {
        // take a named subcommand, then a loaded one
        const named = command.commands ?? {};
        const found = Object.hasOwn(named, word)
            ? named[word]
            : await command.resolve?.(word, this.context);

        return found ?? command.target?.command(word);
    }

    /** List a command's shown subcommands in help order, the loaded ones after the named ones. */
    async children(command: Command): Promise<[string, Command][]> {
        // describe each loaded word by the command it loads
        const loaded: [string, Command][] = [];
        for (const word of (await command.names?.(this.context)) ?? []) {
            const child = await this.child(command, word);
            if (child !== undefined) {
                loaded.push([word, child]);
            }
        }

        return [...Object.entries(command.commands ?? {}), ...loaded].filter(
            ([, child]) => child.isHidden !== true,
        );
    }

    /** List the lines of a command's target in help: its value alone, then with each subcommand. */
    lines(command: Command): [string, Command][] {
        // describe nothing for a command without a target
        const { target } = command;
        if (target === undefined) {
            return [];
        }

        // list the placeholder's own command and each of its subcommands
        const placeholder = `<${target.name}>`;
        const child = target.command(placeholder);
        const own: [string, Command][] = child.run === undefined ? [] : [[placeholder, child]];
        const subcommands = Object.entries(child.commands ?? {})
            .filter(([, subcommand]) => subcommand.isHidden !== true)
            .map(([word, subcommand]): [string, Command] => [`${placeholder} ${word}`, subcommand]);

        return [...own, ...subcommands];
    }

    /** Parse a command line below the program's name, then show help or run its command. */
    async execute(words: readonly string[], isJson: boolean): Promise<void> {
        // read help from a leading help word or a help flag before the separator
        const { help } = GLOBAL_OPTIONS;
        const isHelpFlag = (word: string) => word === help.flag || word === help.short;
        const given = words[0] === "help" ? words.slice(1) : words;
        const separator = given.indexOf(SEPARATOR);
        const flags = separator < 0 ? given : given.slice(0, separator);
        const isHelp = words[0] === "help" || flags.some(isHelpFlag);
        const named = [...flags.filter((word) => !isHelpFlag(word)), ...given.slice(flags.length)];

        // walk the words of subcommands to the command they lead to
        const path = ["destack"];
        let command = this.root;
        let index = 0;
        while (CommandTree.hasSubcommands(command) && index < named.length) {
            const name = nth(named, index);
            if (name.startsWith("-")) {
                break;
            }
            const child = await this.child(command, name);
            if (child === undefined) {
                const unknown = `unknown command ${[...path, name].join(" ")}`;
                const reason = await command.unavailable?.(this.context);
                throw CliError.of(
                    "USAGE_INVALID",
                    reason === undefined ? unknown : `${unknown}: ${reason}`,
                );
            }
            path.push(name);
            command = child;
            index++;
        }

        // show help for a command that runs nothing
        const rest = named.slice(index);
        const { run } = command;
        const isShown = isHelp || run === undefined;

        // refuse help as JSON
        if (isShown && isJson) {
            throw CliError.of(
                "USAGE_INVALID",
                `command '${path.join(" ")}' is invalid: its help does not support '--json'`,
            );
        }
        // refuse an unknown subcommand
        else if (
            isShown &&
            CommandTree.hasSubcommands(command) &&
            rest.some((word) => !word.startsWith("-"))
        ) {
            throw CliError.of(
                "USAGE_INVALID",
                `command '${[...path, rest[0]].join(" ")}' is invalid: it is unknown`,
            );
        }
        // show help without running anything
        else if (isShown) {
            await this.context.terminal.print(await this.help(command, path));
        }
        // refuse JSON from a command that prints none
        else if (isJson && command.isJson !== true) {
            throw CliError.of(
                "USAGE_INVALID",
                `command '${path.join(" ")}' is invalid: it does not support '--json'`,
            );
        }
        // run the command with its arguments and options
        else {
            await run({ ...CommandTree.parse(command, rest, path), isJson }, this.context);
        }
    }

    /** Report whether a command has subcommands: named ones, loaded ones or its target's. */
    static hasSubcommands(command: Command): boolean {
        return (
            command.commands !== undefined ||
            command.resolve !== undefined ||
            command.target !== undefined
        );
    }

    /** Read a command's arguments and options, refusing others. */
    static parse(
        command: Command,
        words: readonly string[],
        path: readonly string[],
    ): Omit<CommandCall, "isJson"> {
        // parse the options the command declares, passing on positional arguments
        const declaredOptions: Record<string, { readonly type: CommandOption["type"] }> =
            Object.fromEntries(
                Object.entries(command.options ?? {}).map(([name, option]) => [
                    name,
                    { type: option.type },
                ]),
            );
        CommandTree.requireOptions(declaredOptions, words, path);
        let parsed: {
            readonly values: CommandCall["options"];
            readonly positionals: readonly string[];
        };
        try {
            parsed = parseArgs({
                args: [...words],
                allowPositionals: true,
                strict: true,
                options: declaredOptions,
            });
        } catch (error) {
            const message = error instanceof Error ? error.message : String(error);
            throw CliError.of(
                "USAGE_INVALID",
                `command '${path.join(" ")}' is invalid: ${message}`,
            );
        }

        // read each positional argument, refusing extra words without a rest argument, and missing ones
        const declared = command.arguments ?? [];
        if (parsed.positionals.length > declared.length && command.rest === undefined) {
            throw CliError.of(
                "USAGE_INVALID",
                `usage is invalid: use '${path.join(" ") + CommandTree.usage(command)}'`,
            );
        }
        const values: Record<string, string | undefined> = {};
        for (const [position, argument] of declared.entries()) {
            const value = parsed.positionals[position] ?? argument.default;
            if (value === undefined) {
                throw CliError.of(
                    "USAGE_INVALID",
                    `usage is invalid: use '${path.join(" ") + CommandTree.usage(command)}'`,
                );
            }
            values[argument.name] = value;
        }

        return {
            arguments: values,
            rest: parsed.positionals.slice(declared.length),
            options: parsed.values,
        };
    }

    /** Refuse an option the command lacks, and a string option whose value starts with a dash. */
    static requireOptions(
        declared: Readonly<Record<string, { readonly type: CommandOption["type"] }>>,
        words: readonly string[],
        path: readonly string[],
    ): void {
        // read the options before the positional separator
        const separator = words.indexOf(SEPARATOR);
        const flags = separator < 0 ? words : words.slice(0, separator);
        const options = [...flags.entries()].filter(
            ([, word]) =>
                word.startsWith("-") && word !== STDIN_VALUE && Number.isNaN(Number(word)),
        );
        for (const [index, word] of options) {
            // refuse an option the command lacks
            const name = word.startsWith("--") ? word.slice(2).split("=")[0] : undefined;
            if (name === undefined || !Object.hasOwn(declared, name)) {
                throw CliError.of(
                    "USAGE_INVALID",
                    `command '${path.join(" ")}' is invalid: it does not support '${word.split("=")[0]}'`,
                );
            }
            // refuse a string option whose next word starts with a dash, standard input aside
            else if (
                declared[name]?.type === "string" &&
                !word.includes("=") &&
                flags[index + 1]?.startsWith("-") === true &&
                flags[index + 1] !== STDIN_VALUE
            ) {
                throw CliError.of(
                    "USAGE_INVALID",
                    `option '${word}' is invalid: it takes a value, so write '${word}'=<value> when the value starts with a dash`,
                );
            }
        }
    }

    /** Render a command's help: its description, usage, subcommands and options. */
    async help(command: Command, path: readonly string[]): Promise<string> {
        // render the description and usage line
        const { terminal } = this.context;
        const isRun = command.run !== undefined;
        const suffix = CommandTree.hasSubcommands(command)
            ? ` <command>${isRun ? ` | ${path.join(" ")}${CommandTree.usage(command)}` : ""}`
            : CommandTree.usage(command);
        const lines = [command.description, "", `Usage: ${path.join(" ")}${suffix}`, ""];

        // list subcommands in help order, reporting why loaded ones are missing, as help must work offline
        if (CommandTree.hasSubcommands(command)) {
            let children: [string, Command][];
            let missing: string | undefined;
            try {
                children = await this.children(command);
            } catch (error) {
                // list the named ones alone while the machine is stopped, nobody signed in or no space set
                const isOffline =
                    MachineError.is(error, "MACHINE_UNAVAILABLE") ||
                    CliError.is(error, "SIGN_IN_UNAUTHENTICATED") ||
                    CliError.is(error, "SPACE_REQUIRED");
                if (!isOffline) {
                    throw error;
                }
                children = Object.entries(command.commands ?? {}).filter(
                    ([, child]) => child.isHidden !== true,
                );
                missing = error instanceof Error ? error.message : String(error);
            }
            // list a group with journeys by them, a line each, the names of no journey last
            if (command.journeys === undefined) {
                children.push(...this.lines(command));
                const width = Math.max(0, ...children.map(([name]) => name.length));
                for (const [name, child] of children) {
                    lines.push(`  ${name.padEnd(width + 2)}${child.description}`);
                }
            } else {
                const names = new Set(children.map(([name]) => name));
                const taken = new Set(command.journeys.flatMap((journey) => journey.commands));
                const others = [...names].filter((name) => !taken.has(name));
                const journeys = [
                    ...command.journeys,
                    ...(others.length === 0 ? [] : [{ title: OTHER_JOURNEY, commands: others }]),
                ];
                const width = Math.max(...journeys.map(({ title }) => title.length));
                for (const { title, commands } of journeys) {
                    // wrap a journey's names under its first, within the help's width
                    const shown = commands.filter((name) => names.has(name));
                    const rows = wrap(shown, HELP_WIDTH - width - JOURNEY_INDENT);
                    for (const [index, row] of rows.entries()) {
                        const head = index === 0 ? title : "";
                        lines.push(`  ${terminal.style("dim", head.padEnd(width))}  ${row}`);
                    }
                }
            }
            if (missing !== undefined) {
                lines.push("", `  More commands load once the machine answers: ${missing}`);
            }
            lines.push("");
        }

        // list the options of the command that runs, padded to the widest beside the least width
        const shown = Object.entries(command.options ?? {})
            .filter(([, option]) => option.isHidden !== true)
            .map(([name, option]): [string, string] => {
                const value = option.type === "string" ? ` <${option.argument ?? "value"}>` : "";

                return [`    --${name}${value}`, option.description];
            });
        const { help, json } = GLOBAL_OPTIONS;
        const globals: GlobalOption[] = [help, ...(command.isJson === true ? [json] : [])];
        const options: [string, string][] = [
            ...globals.map((option): [string, string] => [
                "short" in option ? `${option.short}, ${option.flag}` : `    ${option.flag}`,
                option.description,
            ]),
            ...shown,
        ];
        const width = Math.min(
            OPTION_MAX_WIDTH,
            Math.max(OPTION_WIDTH, ...options.map(([label]) => label.length + OPTION_GAP)),
        );
        for (const [label, description] of options) {
            // put a label wider than the column on its own line, its description below at the column
            if (label.length + OPTION_GAP > width) {
                lines.push(`  ${label}`, `  ${" ".repeat(width)}${description}`);
            }
            // put a label beside its description
            else {
                lines.push(`  ${label.padEnd(width)}${description}`);
            }
        }

        return lines.join("\n");
    }

    /** Render a command's positional arguments, bracketing optional ones. */
    static usage(command: Command): string {
        // name each positional argument, then the rest
        const positional = (command.arguments ?? []).map((argument) => {
            const placeholder = argument.usage ?? argument.name;

            return argument.default === undefined ? ` <${placeholder}>` : ` [${placeholder}]`;
        });
        const rest =
            command.rest === undefined ? "" : ` [${command.rest.usage ?? command.rest.name}...]`;

        return positional.join("") + rest;
    }
}

/** Join names with two spaces into lines no wider than a width, a longer name on a line alone. */
function wrap(names: readonly string[], width: number): string[] {
    const rows: string[] = [];
    for (const name of names) {
        // start a line with a name that does not fit the last
        const last = rows.at(-1);
        if (last === undefined || last.length + name.length + 2 > width) {
            rows.push(name);
        } else {
            rows[rows.length - 1] = `${last}  ${name}`;
        }
    }

    return rows;
}

/** The values of command options, read by their option. */
export const CommandOption = {
    /** Read an option's JSON object inline, from stdin or from a file, none when absent. */
    async json(
        value: string | boolean | undefined,
        option: string,
        environment: Environment<"files">,
        terminal: Terminal,
    ): Promise<JsonObject> {
        // read an absent option as no fields
        const given = CommandOption.string(value, option);
        if (given === undefined) {
            return {};
        }

        // read standard input, a file, or the value itself
        let text = given;
        if (given === STDIN_VALUE) {
            text = await terminal.prompt.rest();
        }
        // read the file after the prefix, refusing a file that does not exist
        else if (given.startsWith(FILE_PREFIX)) {
            const file = given.slice(FILE_PREFIX.length);
            text = await environment.files.readTextFile(file).catch((error: unknown) => {
                if (FileSystemError.is(error, "FILE_NOT_FOUND")) {
                    throw CliError.of(
                        "USAGE_INVALID",
                        `--'${option}' file '${file}' is invalid: it does not exist`,
                    );
                }
                throw error;
            });
        }

        // require a JSON object
        let parsed: unknown;
        try {
            parsed = JSON.parse(text);
        } catch (error) {
            const reason = error instanceof Error ? error.message : String(error);
            throw CliError.of(
                "USAGE_INVALID",
                `--'${option}' is invalid: it must be a JSON object ('${reason}')`,
            );
        }
        const fields = schema.jsonObject().safeParse(parsed);
        if (!fields.success) {
            throw CliError.of(
                "USAGE_INVALID",
                `--'${option}' is invalid: it must be a JSON object`,
            );
        }

        return fields.data;
    },

    /** Read an option's text as a positive whole number, absent when not given. */
    count(value: string | boolean | undefined, option: string): number | undefined {
        // read the number, refusing other text
        const text = CommandOption.string(value, option);
        if (text === undefined) {
            return undefined;
        }
        const count = Number(text);
        if (!Number.isSafeInteger(count) || count < 1) {
            throw CliError.of(
                "USAGE_INVALID",
                `--'${option}' is invalid: it takes a positive whole number`,
            );
        }

        return count;
    },

    /** Read a string option, absent when not given, refusing a flag without its value. */
    string(value: string | boolean | undefined, option: string): string | undefined {
        if (typeof value === "boolean") {
            throw CliError.of("USAGE_INVALID", `--'${option}' is invalid: it takes a value`);
        }

        return value;
    },
};
