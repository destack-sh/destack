import { nth } from "@destack/schema";
import { type CommandTree, GLOBAL_OPTIONS, type Node, type Option } from "../command/command.ts";

/** The shells a completion script is written for. */
export const SHELLS = ["bash", "zsh", "fish"] as const;

/** A shell a completion script is written for. */
export type Shell = (typeof SHELLS)[number];

/** The hidden command completion scripts call with the typed words. */
export const COMPLETE = "__complete";

/** The line asking the shell to list directories. */
const DIRECTORIES = ":directories";

/** What the typed command completes to: a word, or the directories the shell lists itself. */
export type Candidate =
    | {
          /** A subcommand's name, an argument's value or an option's flag. */
          readonly kind: "word";
          /** The word as typed. */
          readonly text: string;
          /** The word's line in help. */
          readonly description: string;
      }
    | {
          /** The directories of the partial word, which the shell lists. */
          readonly kind: "directories";
      };

/** Completion scripts that ask the program for candidates, and the candidates they are answered. */
export const Completion = {
    /** Write the completion script of a shell for the program's name. */
    write(shell: Shell, program: string): string {
        // prefix the shell functions with the program, replacing its hyphens
        const prefix = program.replaceAll(/[^A-Za-z0-9]/gu, "_");

        // write bash's script
        if (shell === "bash") {
            return bash(program, prefix);
        }
        // write zsh's script
        else if (shell === "zsh") {
            return zsh(program, prefix);
        }
        // write fish's script
        else {
            return fish(program, prefix);
        }
    },

    /** List the candidates of the last typed word, which may be empty, after the earlier words. */
    async candidates(tree: CommandTree, words: readonly string[]): Promise<Candidate[]> {
        // walk the typed names to their command, passing over options and counting arguments
        const typed = words.slice(0, -1);
        const partial = words.at(-1) ?? "";
        let command = tree.root;
        let positionals = 0;
        let pending: Option | undefined;
        for (let index = 0; index < typed.length; index++) {
            // pass over an option, and the value a string option takes as the next word
            const word = nth(typed, index);
            const options = tree.options(command);
            if (word.startsWith("-")) {
                const option = options[word.slice(2)];
                const isValued = option?.type === "string" && !word.includes("=");
                pending = isValued && index === typed.length - 1 ? option : undefined;
                index += isValued ? 1 : 0;
            }
            // descend into a group's subcommand, offering nothing after an unknown name
            else if ("commands" in command) {
                const child = await tree.child(command, word);
                if (child === undefined) {
                    return [];
                }
                command = child;
            }
            // count an executable's positional argument
            else {
                positionals++;
            }
        }

        // offer the choices of the option the partial word is the value of
        if (pending !== undefined) {
            return values(pending.choices ?? [], pending.description).filter(
                (candidate) => candidate.kind === "word" && candidate.text.startsWith(partial),
            );
        }

        // offer the command's names, its next argument's values and its flags the partial word starts
        const argument = await argumentValues(command, positionals);
        const offered = [
            ...(await names(tree, command)),
            ...argument,
            ...flags(tree, command, command === tree.root),
        ].filter(
            (candidate) => candidate.kind === "directories" || candidate.text.startsWith(partial),
        );

        return offered;
    },

    /** Render a candidate as the line completion scripts read: the word, a tab, its description. */
    line(candidate: Candidate): string {
        return candidate.kind === "word"
            ? `${candidate.text}\t${candidate.description}`
            : DIRECTORIES;
    },
};

/** Offer values, each described by what it completes. */
function values(offered: readonly string[], description: string): Candidate[] {
    return offered.map((value) => ({ kind: "word", text: value, description }));
}

/** List the values of a group's argument, or of the next positional argument of a verb. */
async function argumentValues(command: Node, positionals: number): Promise<Candidate[]> {
    // take a group's argument, or the verb's next positional one
    const argument = "commands" in command ? command.argument : command.arguments?.[positionals];

    // offer nothing past the arguments
    if (argument === undefined) {
        return [];
    }
    // let the shell list directories
    else if ("isDirectory" in argument && argument.isDirectory) {
        return [{ kind: "directories" }];
    }
    // offer the argument's values, as a setting's names or a noun's identifiers, none without any
    else {
        return argument.complete === undefined
            ? []
            : values(await argument.complete(), argument.name);
    }
}

/** List a group's subcommands in help order. */
async function names(tree: CommandTree, command: Node): Promise<Candidate[]> {
    // offer no names under a verb
    if (!("commands" in command)) {
        return [];
    }

    return (await tree.children(command)).map(([name, child]) => ({
        kind: "word",
        text: name,
        description: child.description,
    }));
}

/** List a command's flags: its options, help, and JSON output or the root's version. */
function flags(tree: CommandTree, command: Node, isRoot: boolean): Candidate[] {
    // list the shown options of the command that runs
    const own = Object.entries(tree.options(command))
        .filter(([, option]) => option.isHidden !== true)
        .map(([name, option]): Candidate => ({
            kind: "word",
            text: `--${name}`,
            description: option.description,
        }));

    // add help and JSON output or the root's version
    const executable = tree.verb(command);
    const { help, json, version } = GLOBAL_OPTIONS;
    const isJson = executable?.json === true;
    const globals = [help, ...(isJson ? [json] : []), ...(isRoot ? [version] : [])];

    return [
        ...own,
        ...globals.map((option): Candidate => ({
            kind: "word",
            text: option.flag,
            description: option.description,
        })),
    ];
}

/** Write the bash script: ask the program for the candidates of the typed words. */
function bash(program: string, prefix: string): string {
    return `# bash completion for ${program}
_${prefix}() {
    local candidate description
    COMPREPLY=()
    while IFS=$'\\t' read -r candidate description; do
        if [[ "$candidate" == "${DIRECTORIES}" ]]; then
            compopt -o filenames 2>/dev/null
            COMPREPLY+=($(compgen -d -- "\${COMP_WORDS[COMP_CWORD]}"))
        else
            COMPREPLY+=("$candidate")
        fi
    done < <(${program} ${COMPLETE} "\${COMP_WORDS[@]:1:COMP_CWORD}" 2>/dev/null)
}
complete -F _${prefix} ${program}`;
}

/** Write the zsh script, which runs from fpath as an autoloaded function or when sourced. */
function zsh(program: string, prefix: string): string {
    return `#compdef ${program}
_${prefix}() {
    local line is_directories=0
    local -a candidates
    for line in \${(f)"$(${program} ${COMPLETE} "\${(@)words[2,CURRENT]}" 2>/dev/null)"}; do
        if [[ "$line" == "${DIRECTORIES}" ]]; then
            is_directories=1
        else
            candidates+=("\${\${line%%$'\\t'*}//:/\\\\:}:\${line#*$'\\t'}")
        fi
    done
    (( is_directories )) && _files -/
    _describe -t candidates ${program} candidates
}
if [ "$funcstack[1]" = "_${prefix}" ]; then
    _${prefix} "$@"
else
    compdef _${prefix} ${program}
fi`;
}

/** Write the fish script. */
function fish(program: string, prefix: string): string {
    return `# fish completion for ${program}
function __${prefix}_complete
    set -l typed (commandline -opc) (commandline -ct)
    for line in (${program} ${COMPLETE} $typed[2..-1] 2>/dev/null)
        if test "$line" = "${DIRECTORIES}"
            __fish_complete_directories (commandline -ct)
        else
            echo $line
        end
    end
end
complete -c ${program} -f -a '(__${prefix}_complete)'`;
}
