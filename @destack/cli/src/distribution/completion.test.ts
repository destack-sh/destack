import { join } from "node:path";
import { testSimulation } from "@destack/object/test";
import { expect, onTestFinished, test } from "@destack/test";
import { CommandTree, type Node } from "../command/command.ts";
import { Completion } from "./completion.ts";

/** A note type's commands, as a kind's singular noun loads them. */
const NOTE: Node = {
    description: "Get, create and change notes",
    commands: {
        create: {
            description: "Call note.create",
            json: true,
            options: {
                title: { type: "string", description: "The title", argument: "string" },
                pinned: { type: "boolean", description: "Set pinned" },
            },
            run: async () => {},
        },
        archive: {
            description: "Call note.archive",
            arguments: [{ name: "id" }],
            json: true,
            options: { input: { type: "string", description: "The input", argument: "json" } },
            run: async () => {},
        },
    },
    argument: {
        name: "id",
        command: () => ({ description: "Get a note", json: true, run: async () => {} }),
        complete: async () => ["note-1", "note-2"],
    },
};

/** A command tree with a group, an executable command, and a group loading kinds. */
const TREE: Node = {
    description: "Tool",
    commands: {
        app: {
            description: "Manage apps",
            commands: {
                open: {
                    description: "Open an app's view",
                    arguments: [{ name: "name", complete: async () => ["board", "notes"] }],
                    json: false,
                    options: {
                        browser: { type: "boolean", description: "Open it in the browser" },
                    },
                    run: async () => {},
                },
            },
        },
        notes: {
            description: "List notes",
            json: true,
            options: {
                where: { type: "string", description: "The filter rows meet", argument: "filter" },
                order: { type: "string", description: "The order", choices: ["asc", "desc"] },
                watch: { type: "boolean", description: "Follow the rows live" },
            },
            run: async () => {},
        },
        version: { description: "Print the version", json: true, run: async () => {} },
        build: {
            description: "Build a package",
            arguments: [{ name: "directory", default: ".", isDirectory: true }],
            json: false,
            run: async () => {},
        },
    },
    resolve: async (name) => (name === "note" ? NOTE : undefined),
    names: async () => ["note"],
};

/** The words a shell completes after each line of typed words. */
async function complete(
    shell: "bash" | "zsh",
    lines: readonly (readonly string[])[],
): Promise<string[]> {
    // write the script where the shell sources it, in a separate environment
    const environment = testSimulation().environment("shell");
    const directory = await environment.files.makeTemporaryDirectory("destack-completion-");
    onTestFinished(() => environment.files.remove(directory, { recursive: true, force: true }));
    const script = join(directory, "completion");
    await environment.files.writeTextFile(script, Completion.write(shell, "my-tool"));

    // answer each word the program is asked with itself in brackets, described by its position, and a directory's words with the directive
    const program = `my-tool() { shift; if [ "$1" = dir ]; then echo ':directories'; return; fi; local index=0 word; for word in "$@"; do index=$((index + 1)); printf '[%s]\\tword %s\\n' "$word" "$index"; done; }`;
    const setup =
        shell === "bash"
            ? `source '${script}'`
            : `compdef() { :; }; _describe() { echo "\${(P)4}"; }; _files() { echo "files $*"; }; source '${script}'`;
    await environment.files.mkdir(join(directory, "alpha"));
    await environment.files.writeTextFile(join(directory, "apple.txt"), "");
    const completions = lines.map((typed) => {
        const words = typed.map((word) => `'${word}'`).join(" ");

        return shell === "bash"
            ? `COMP_WORDS=(${words}); COMP_CWORD=${typed.length - 1}; _my_tool; echo "\${COMPREPLY[*]}"`
            : `words=(${words}); CURRENT=${typed.length}; _my_tool`;
    });

    // complete every line in one shell
    const child = environment.process.spawn(
        [shell, "-c", [program, setup, ...completions].join("; ")],
        {
            workingDirectory: directory,
            stdout: "pipe",
            stderr: "pipe",
        },
    );
    const [output, errors] = await Promise.all([
        new Response(child.stdout).text(),
        new Response(child.stderr).text(),
    ]);
    expect(await child.exited, errors).toBe(0);

    return output.trimEnd().split("\n");
}

test("pass the typed words and the partial one to the program, and offer its answers in bash and zsh", async () => {
    // offer the words after the program's name, the partial one last, an empty one, and colons zsh escapes
    const lines = [
        ["my-tool", "note", "cr"],
        ["my-tool", ""],
        ["my-tool", "a:b"],
        ["my-tool", "dir", "a"],
    ];
    const [bash, zsh] = await Promise.all([complete("bash", lines), complete("zsh", lines)]);

    // expect the program's words, and the shell's own directories for the directive, leaving out files
    expect({ bash, zsh }).toEqual({
        bash: ["[note] [cr]", "[]", "[a:b]", "alpha"],
        zsh: ["[note]:word 1 [cr]:word 2", "[]:word 1", "[a\\:b]:word 1", "files -/"],
    });
});

test("complete names, the nouns a group loads and their methods, arguments, option values and flags", async () => {
    const cases = [
        [""],
        ["n"],
        ["note", ""],
        ["note", "create", "--title", "Ideas", "--p"],
        ["notes", "--where=status:open", "--w"],
        ["notes", "--where", "status = open", "-"],
        ["page", ""],
        ["version", ""],
        ["app", "open", "b"],
        ["notes", "--order", ""],
        ["notes", "--where", ""],
        ["build", ""],
    ];

    // describe each candidate by its help line
    const candidates = await Promise.all(
        cases.map((words) => Completion.candidates(new CommandTree(TREE), words)),
    );
    expect(candidates.map((found) => found.map((candidate) => Completion.line(candidate)))).toEqual(
        [
            [
                "app\tManage apps",
                "notes\tList notes",
                "version\tPrint the version",
                "build\tBuild a package",
                "note\tGet, create and change notes",
                "--help\tShow help",
                "--version\tPrint the installed version",
            ],
            ["notes\tList notes", "note\tGet, create and change notes"],
            [
                "create\tCall note.create",
                "archive\tCall note.archive",
                "note-1\tid",
                "note-2\tid",
                "--help\tShow help",
            ],
            ["--pinned\tSet pinned"],
            ["--where\tThe filter rows meet", "--watch\tFollow the rows live"],
            [
                "--where\tThe filter rows meet",
                "--order\tThe order",
                "--watch\tFollow the rows live",
                "--help\tShow help",
                "--json\tPrint JSON",
            ],
            [],
            ["--help\tShow help", "--json\tPrint JSON"],
            ["board\tname"],
            ["asc\tThe order", "desc\tThe order"],
            [],
            [":directories", "--help\tShow help"],
        ],
    );
});
