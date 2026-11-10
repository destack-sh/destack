import { type Environment, FileSystemError } from "@destack/environment";
import { parseArgs } from "node:util";
import { type JsonObject, nth, schema } from "@destack/schema";

import type { Terminal } from "../output/index.ts";
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

/** The least width a help line pads its options to, in characters. */
const OPTION_WIDTH = 24;

/** The spaces between the widest option and its description, in characters. */
const OPTION_GAP = 2;

/** An option every command line takes: its flag, its short flag and its line in help. */
interface GlobalOption {
    /** The long flag, such as --json. */
    readonly flag: string;
    /** The short flag, such as -h, absent for none. */
    readonly short?: string;
    /** The option's line in help. */
    readonly description: string;
}

/** A node of the command tree: a group of words or a verb that runs. */
export type Node = Group | Verb;

/** A command with subcommands, running its default command when given none. */
export interface Group {
    /** One line shown in help. */
    readonly description: string;
    /** The named subcommands, in help order. */
    readonly commands: Readonly<Record<string, Node>>;
    /** Load the subcommand of a name outside `commands`, such as a noun's or an object's. */
    readonly resolve?: (name: string) => Promise<Node | undefined>;
    /** List the names outside `commands` that `resolve` loads, for help and completion. */
    readonly names?: () => Promise<readonly string[]>;
    /** Explain why `resolve` loads no subcommand now, such as for a signed-out person. */
    readonly unavailable?: () => Promise<string | undefined>;
    /** The positional argument leading to a subcommand of any value, such as an object's id. */
    readonly argument?: GroupArgument;
    /** The command the group runs without a subcommand, help when absent. */
    readonly default?: Verb;
    /** Whether help and completion leave the group out. */
    readonly isHidden?: boolean;
}

/** A group's positional argument, which leads to the subcommand of its value. */
export interface GroupArgument {
    /** The argument's name in help, such as id. */
    readonly name: string;
    /** Build the subcommand of a value. */
    readonly command: (value: string) => Node;
    /** List the values completion offers. */
    readonly complete?: () => Promise<readonly string[]>;
}

/** A command that runs. */
export interface Verb {
    /** One line shown in help. */
    readonly description: string;
    /** The positional arguments in order, the optional ones last. */
    readonly arguments?: readonly Argument[];
    /** The command's options, by name. */
    readonly options?: Readonly<Record<string, Option>>;
    /** Whether the command prints JSON with --json. */
    readonly json: boolean;
    /** Whether help and completion leave the command out. */
    readonly isHidden?: boolean;
    /** Run with the arguments and options the command line gives. */
    readonly run: (call: Call) => Promise<void>;
}

/** A positional argument of a command. */
export interface Argument {
    /** The argument's name in the call, and in help without a usage. */
    readonly name: string;
    /** The argument's placeholder in help, such as link|id. */
    readonly usage?: string;
    /** The value of an omitted argument, absent for a required one. */
    readonly default?: string;
    /** List the values completion offers. */
    readonly complete?: () => Promise<readonly string[]>;
    /** Whether the argument is a directory the shell completes. */
    readonly isDirectory?: boolean;
}

/** One named command-line option. */
export interface Option {
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

/** The values of a command's options, by option name. */
export type Options = Readonly<Record<string, string | boolean | undefined>>;

/** A command's run: its arguments, its options and whether it prints JSON. */
export interface Call {
    /** The positional arguments by name, an omitted optional one absent. */
    readonly arguments: Readonly<Record<string, string | undefined>>;
    /** The options given, by name. */
    readonly options: Options;
    /** Whether the command prints JSON. */
    readonly json: boolean;
}

/** The command tree below the program's name: its root, the words leading to its verbs. */
export class CommandTree {
    /** The root group. */
    readonly root: Node;

    /** Read command lines through a root. */
    constructor(root: Node) {
        this.root = root;
    }

    /** Find a group's subcommand of a name: a named one, a loaded one, or its argument's. */
    async child(group: Group, name: string): Promise<Node | undefined> {
        // take a named subcommand, then a loaded one
        const found = Object.hasOwn(group.commands, name)
            ? group.commands[name]
            : await group.resolve?.(name);

        return found ?? group.argument?.command(name);
    }

    /** List a group's shown subcommands in help order, the loaded ones after the named ones. */
    async children(group: Group): Promise<[string, Node][]> {
        // describe each loaded name by the command it loads
        const loaded: [string, Node][] = [];
        for (const name of (await group.names?.()) ?? []) {
            const child = await this.child(group, name);
            if (child !== undefined) {
                loaded.push([name, child]);
            }
        }

        return [...Object.entries(group.commands), ...loaded].filter(
            ([, child]) => child.isHidden !== true,
        );
    }

    /** List the lines of a group's argument in help: its value alone, then with each subcommand. */
    lines(group: Group): [string, Node][] {
        // describe nothing for a group without an argument
        const { argument } = group;
        if (argument === undefined) {
            return [];
        }

        // build the subcommand of the argument's placeholder
        const placeholder = `<${argument.name}>`;
        const child = argument.command(placeholder);
        if (!("commands" in child)) {
            return [[placeholder, child]];
        }

        // list the placeholder's command and each of its subcommands
        const own: [string, Node][] =
            child.default === undefined ? [] : [[placeholder, child.default]];
        const subcommands = Object.entries(child.commands)
            .filter(([, subcommand]) => subcommand.isHidden !== true)
            .map(([name, subcommand]): [string, Node] => [`${placeholder} ${name}`, subcommand]);

        return [...own, ...subcommands];
    }

    /** Read the verb a node runs: a verb itself, or a group's default. */
    verb(node: Node): Verb | undefined {
        return "commands" in node ? node.default : node;
    }

    /** Read the options of the verb a node runs, none for a group without one. */
    options(node: Node): Readonly<Record<string, Option>> {
        return this.verb(node)?.options ?? {};
    }

    /** Parse a command line below the program's name, then show help or run its command. */
    async execute(words: readonly string[], json: boolean, terminal: Terminal): Promise<void> {
        // read help from a leading help word or a help flag before the separator
        const { help } = GLOBAL_OPTIONS;
        const isHelpFlag = (word: string) => word === help.flag || word === help.short;
        const given = words[0] === "help" ? words.slice(1) : words;
        const separator = given.indexOf(SEPARATOR);
        const flags = separator < 0 ? given : given.slice(0, separator);
        const isHelp = words[0] === "help" || flags.some(isHelpFlag);
        const named = [...flags.filter((word) => !isHelpFlag(word)), ...given.slice(flags.length)];

        // walk the names of groups' subcommands to the command they lead to
        const path = ["destack"];
        let command = this.root;
        let index = 0;
        while ("commands" in command && index < named.length) {
            const name = nth(named, index);
            if (name.startsWith("-")) {
                break;
            }
            const child = await this.child(command, name);
            if (child === undefined) {
                const unknown = `unknown command ${[...path, name].join(" ")}`;
                const reason = await command.unavailable?.();
                throw CliError.of(
                    "USAGE_INVALID",
                    reason === undefined ? unknown : `${unknown}: ${reason}`,
                );
            }
            path.push(name);
            command = child;
            index++;
        }

        // find the verb the names lead to, absent for a group without a default
        const rest = named.slice(index);
        const verb = this.verb(command);

        // refuse help as JSON
        const isShown = isHelp || verb === undefined;
        if (isShown && json) {
            throw CliError.of("USAGE_INVALID", "help does not support --json");
        }
        // refuse a group's unknown subcommand
        else if (isShown && "commands" in command && rest.some((word) => !word.startsWith("-"))) {
            throw CliError.of("USAGE_INVALID", `unknown command ${[...path, rest[0]].join(" ")}`);
        }
        // show help without running anything
        else if (isHelp || verb === undefined) {
            await terminal.print(await this.help(command, path));
        }
        // refuse JSON from a verb that prints none
        else if (json && !verb.json) {
            throw CliError.of("USAGE_INVALID", `${path.join(" ")} does not support --json`);
        }
        // run the verb with its arguments and options
        else {
            await verb.run({ ...CommandTree.parse(verb, rest, path), json });
        }
    }

    /** Read a verb's arguments and options, refusing others. */
    static parse(
        command: Verb,
        words: readonly string[],
        path: readonly string[],
    ): Pick<Call, "arguments" | "options"> {
        // parse the options the command declares, passing on positional arguments
        const declaredOptions: Record<string, { readonly type: Option["type"] }> =
            Object.fromEntries(
                Object.entries(command.options ?? {}).map(([name, option]) => [
                    name,
                    { type: option.type },
                ]),
            );
        CommandTree.requireOptions(declaredOptions, words, path);
        let parsed: { readonly values: Options; readonly positionals: readonly string[] };
        try {
            parsed = parseArgs({
                args: [...words],
                allowPositionals: true,
                strict: true,
                options: declaredOptions,
            });
        } catch (error) {
            const message = error instanceof Error ? error.message : String(error);
            throw CliError.of("USAGE_INVALID", `${path.join(" ")}: ${message}`);
        }

        // read each positional argument, refusing extra and missing ones
        const declared = command.arguments ?? [];
        if (parsed.positionals.length > declared.length) {
            throw CliError.of(
                "USAGE_INVALID",
                `usage: ${path.join(" ")}${CommandTree.usage(command)}`,
            );
        }
        const values: Record<string, string | undefined> = {};
        for (const [position, argument] of declared.entries()) {
            const value = parsed.positionals[position] ?? argument.default;
            if (value === undefined) {
                throw CliError.of(
                    "USAGE_INVALID",
                    `usage: ${path.join(" ")}${CommandTree.usage(command)}`,
                );
            }
            values[argument.name] = value;
        }

        return { arguments: values, options: parsed.values };
    }

    /** Refuse an option the command lacks, and a string option whose value starts with a dash. */
    static requireOptions(
        declared: Readonly<Record<string, { readonly type: Option["type"] }>>,
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
                    `${path.join(" ")} does not support ${word.split("=")[0]}`,
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
                    `${word} takes a value: write ${word}=<value> when the value starts with a dash`,
                );
            }
        }
    }

    /** Render a command's help: its description, usage, subcommands and options. */
    async help(command: Node, path: readonly string[]): Promise<string> {
        // render the description and usage line
        const executable = this.verb(command);
        const suffix =
            "commands" in command
                ? ` <command>${executable === undefined ? "" : ` | ${path.join(" ")}${CommandTree.usage(executable)}`}`
                : CommandTree.usage(command);
        const lines = [command.description, "", `Usage: ${path.join(" ")}${suffix}`, ""];

        // list subcommands in help order, reporting why loaded ones are missing, as help must work offline
        if ("commands" in command) {
            let children: [string, Node][];
            let missing: string | undefined;
            try {
                children = await this.children(command);
            } catch (error) {
                // list the named ones alone while the machine is stopped, nobody signed in or no space set
                const isOffline =
                    MachineError.is(error, "MACHINE_STOPPED") ||
                    CliError.is(error, "SIGN_IN_REQUIRED") ||
                    CliError.is(error, "SPACE_REQUIRED");
                if (!isOffline) {
                    throw error;
                }
                children = Object.entries(command.commands).filter(
                    ([, child]) => child.isHidden !== true,
                );
                missing = error instanceof Error ? error.message : String(error);
            }
            children.push(...this.lines(command));
            const width = Math.max(0, ...children.map(([name]) => name.length));
            for (const [name, child] of children) {
                lines.push(`  ${name.padEnd(width + 2)}${child.description}`);
            }
            if (missing !== undefined) {
                lines.push("", `  More commands load once the machine answers: ${missing}`);
            }
            lines.push("");
        }

        // list the options of the command that runs, padded to the widest beside the least width
        const shown = Object.entries(executable?.options ?? {})
            .filter(([, option]) => option.isHidden !== true)
            .map(([name, option]): [string, string] => {
                const value = option.type === "string" ? ` <${option.argument ?? "value"}>` : "";

                return [`    --${name}${value}`, option.description];
            });
        const { help, json } = GLOBAL_OPTIONS;
        const globals: GlobalOption[] = [help, ...(executable?.json === true ? [json] : [])];
        const options: [string, string][] = [
            ...globals.map((option): [string, string] => [
                "short" in option ? `${option.short}, ${option.flag}` : `    ${option.flag}`,
                option.description,
            ]),
            ...shown,
        ];
        const width = Math.max(
            OPTION_WIDTH,
            ...options.map(([label]) => label.length + OPTION_GAP),
        );
        for (const [label, description] of options) {
            lines.push(`  ${label.padEnd(width)}${description}`);
        }

        return lines.join("\n");
    }

    /** Render a command's positional arguments, bracketing optional ones. */
    static usage(command: Verb): string {
        return (command.arguments ?? [])
            .map((argument) => {
                const placeholder = argument.usage ?? argument.name;

                return argument.default === undefined ? ` <${placeholder}>` : ` [${placeholder}]`;
            })
            .join("");
    }
}

/** The values of command-line options, read by their option. */
export const OptionValue = {
    /** Read an option's JSON object inline, from stdin or from a file, none when absent. */
    async json(
        value: string | boolean | undefined,
        option: string,
        environment: Environment<"files">,
        terminal: Terminal,
    ): Promise<JsonObject> {
        // read an absent option as no fields
        const given = OptionValue.string(value, option);
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
                if (FileSystemError.is(error, "NOT_FOUND")) {
                    throw CliError.of("USAGE_INVALID", `--${option} file ${file} does not exist`);
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
            throw CliError.of("USAGE_INVALID", `--${option} must be a JSON object: ${reason}`);
        }
        const fields = schema.jsonObject().safeParse(parsed);
        if (!fields.success) {
            throw CliError.of("USAGE_INVALID", `--${option} must be a JSON object`);
        }

        return fields.data;
    },

    /** Read an option's text as a positive whole number, absent when not given. */
    count(value: string | boolean | undefined, option: string): number | undefined {
        // read the number, refusing other text
        const text = OptionValue.string(value, option);
        if (text === undefined) {
            return undefined;
        }
        const count = Number(text);
        if (!Number.isSafeInteger(count) || count < 1) {
            throw CliError.of("USAGE_INVALID", `--${option} takes a positive whole number`);
        }

        return count;
    },

    /** Read a string option, absent when not given, refusing a flag without its value. */
    string(value: string | boolean | undefined, option: string): string | undefined {
        if (typeof value === "boolean") {
            throw CliError.of("USAGE_INVALID", `--${option} takes a value`);
        }

        return value;
    },
};
