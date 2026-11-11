import { expect, test } from "@destack/test";
import { CommandFailure, Interrupted } from "./error.ts";
import manifest from "../../package.json" with { type: "json" };
import { TerminalFixture } from "../test/terminal.ts";
import { CliError } from "../error/index.ts";
import { Terminal } from "../output/index.ts";
import { testHost } from "@destack/test/host";

/** Split printed text into its lines. */
function lines(text: string): string[] {
    return text.split("\n").filter((line) => line !== "");
}

test("dispatch fixed verbs, and refuse unknown options, missing arguments and help as JSON", async () => {
    const cases = [
        {
            arguments: ["--version", "--json"],
            code: 0,
            output: [JSON.stringify({ version: manifest.version })],
            error: [],
        },
        {
            arguments: ["--version"],
            code: 0,
            output: [manifest.version],
            error: [],
        },
        {
            arguments: ["whoami", "--version"],
            code: 2,
            output: [],
            error: ["destack: usage is invalid: use --version without a command"],
        },
        {
            arguments: ["self", "update", "--check", "--stage", "--json"],
            code: 2,
            output: [],
            error: [
                JSON.stringify({
                    error: {
                        code: "BAD_REQUEST",
                        message:
                            "command 'destack self update' is invalid: it does not support '--check'",
                    },
                }),
            ],
        },
        {
            arguments: ["apply", "--archive", "/unused/archive", "--json"],
            code: 2,
            output: [],
            error: [
                JSON.stringify({
                    error: {
                        code: "BAD_REQUEST",
                        message:
                            "command 'destack apply' is invalid: it does not support '--archive'",
                    },
                }),
            ],
        },
        {
            arguments: ["switch"],
            code: 2,
            output: [],
            error: ["destack: usage is invalid: use 'destack switch <space>'"],
        },
        {
            arguments: ["completion", "powershell"],
            code: 2,
            output: [],
            error: [
                "destack: shell 'powershell' is invalid: it has no completion, so use bash, zsh or fish",
            ],
        },
        {
            arguments: ["self", "--help", "--json"],
            code: 2,
            output: [],
            error: [
                JSON.stringify({
                    error: {
                        code: "BAD_REQUEST",
                        message:
                            "command 'destack self' is invalid: its help does not support '--json'",
                    },
                }),
            ],
        },
        {
            arguments: ["open"],
            code: 2,
            output: [],
            error: ["destack: usage is invalid: use 'destack open <link|id>'"],
        },
    ];

    // run each command line, splitting what it printed into lines
    const outcomes = [];
    for (const scenario of cases) {
        const { code, output, error } = await TerminalFixture.printed(scenario.arguments);
        outcomes.push({
            arguments: scenario.arguments,
            code,
            output: lines(output),
            error: lines(error),
        });
    }

    expect(outcomes).toEqual(cases);
});

test("leave hidden commands and options out of help, and run them when named", async () => {
    const help = async (words: string[]) => lines((await TerminalFixture.printed(words)).output);

    // list the distribution's commands and the uninstall's options
    expect({
        self: await help(["self", "--help"]),
        uninstall: await help(["self", "uninstall", "--help"]),
        activate: await help(["self", "activate", "--help"]),
    }).toEqual({
        self: [
            "Install, start, stop, update and uninstall this Destack distribution",
            "Usage: destack self <command>",
            "  start      Start this computer's machine, or use the one running",
            "  stop       Stop this computer's machine",
            "  install    Install Destack for this user",
            "  update     Stage the latest signed Destack release and restart into it",
            "  uninstall  Uninstall Destack and retain user state",
            "  -h, --help              Show help",
        ],
        uninstall: [
            "Uninstall Destack and retain user state",
            "Usage: destack self uninstall",
            "  -h, --help              Show help",
        ],
        activate: [
            "Restart into the staged release, as the machine asks once a restart is chosen",
            "Usage: destack self activate",
            "  -h, --help              Show help",
        ],
    });
});

test("report a failure without a service code as internal, exiting with 1", async () => {
    // report a broken service's failure as JSON on a terminal of its own
    const running = TerminalFixture.process({}, testHost().scheduler);
    const terminal = new Terminal(running, "UTC");
    const code = await new CommandFailure(new TypeError("the title index is corrupt")).report(
        true,
        terminal,
    );
    const written = running.stderr.chunks.map((chunk) => new TextDecoder().decode(chunk));

    // keep the message for the person running the command line, as servers mask theirs before
    expect({ code, written }).toEqual({
        code: 1,
        written: [
            `${JSON.stringify({ error: { code: "INTERNAL_SERVER_ERROR", message: "the title index is corrupt" } })}\n`,
        ],
    });
});

test("exit an interrupted command as an interrupted program, printing nothing", async () => {
    // report an interrupt and a usage error on a terminal of its own
    const running = TerminalFixture.process({}, testHost().scheduler);
    const terminal = new Terminal(running, "UTC");
    const codes = [
        await new CommandFailure(new Interrupted()).report(false, terminal),
        await new CommandFailure(
            CliError.of("USAGE_INVALID", "answer is invalid: it must be yes"),
        ).report(false, terminal),
    ];
    const written = running.stderr.chunks.map((chunk) => new TextDecoder().decode(chunk));
    expect({ codes, written }).toEqual({
        codes: [130, 2],
        written: ["destack: answer is invalid: it must be yes\n"],
    });
});
