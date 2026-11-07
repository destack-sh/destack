import { styleText } from "node:util";
import type { Environment } from "@destack/environment";
import { Prompt } from "./prompt.ts";

/** A text format understood by the terminal. */
export type Format = Parameters<typeof styleText>[0];

/** Format text in a format, or leave it as it is. */
export type Style = (format: Format, text: string) => string;

/** The process a command line writes its output to and reads its answers from. */
export type TerminalProcess = Environment<"process">["process"];

/** Leave text unformatted. */
export const plain: Style = (_, text) => text;

/** The terminal a command line writes its output and diagnostics to. */
export class Terminal {
    /** The process whose standard streams the terminal writes. */
    readonly process: TerminalProcess;
    /** Format text only when standard output is a terminal accepting colour. */
    readonly style: Style;
    /** Whether standard output is a terminal, which tables align for, else tab-separated. */
    readonly isTerminal: boolean;
    /** The answers a person types or a script pipes on standard input, read by one reader. */
    readonly prompt: Prompt;
    /** The time zone the terminal shows times in, such as Europe/Vienna. */
    readonly timeZone: string;

    /** Write to a process's standard streams. */
    constructor(running: TerminalProcess, timeZone: string) {
        // set the process, and colour output only on a terminal without NO_COLOR
        this.process = running;
        this.isTerminal = running.stdout.isTerminal;
        this.prompt = new Prompt(running);
        this.timeZone = timeZone;
        const isColored = this.isTerminal && running.variables["NO_COLOR"] === undefined;
        this.style = isColored
            ? (format, text) => styleText(format, text, { validateStream: false })
            : plain;
    }

    /** Write a line of a command's output to standard output. */
    async print(text: string): Promise<void> {
        await this.process.stdout.write(`${text}\n`);
    }

    /** Write a line of a command's progress notes and failures to standard error. */
    async printDiagnostic(text: string): Promise<void> {
        await this.process.stderr.write(`${text}\n`);
    }

    /** Start output following a changing value, as text or JSON. */
    live(json: boolean): Live {
        return new Live(this.process.stdout, json);
    }
}

/** Output following a changing value, redrawn in place on a terminal. */
export class Live {
    /** The standard output the renderings go to. */
    readonly #output: TerminalProcess["stdout"];
    /** Whether the renderings are JSON, appended one line per value. */
    readonly #isJson: boolean;
    /** The lines the last drawing took on the terminal. */
    #lines: number;

    /** Start with nothing drawn on a standard output, as text or JSON. */
    constructor(output: TerminalProcess["stdout"], json: boolean) {
        this.#output = output;
        this.#isJson = json;
        this.#lines = 0;
    }

    /** Show the latest rendering: text redrawn on a terminal, else appended line by line. */
    async show(text: string): Promise<void> {
        // redraw text in place on a terminal, erasing the previous rendering
        if (this.#output.isTerminal && !this.#isJson) {
            const erase = this.#lines === 0 ? "" : `\x1b[${this.#lines}F\x1b[J`;
            await this.#output.write(`${erase}${text}\n`);
            this.#lines = text.split("\n").length;
        }
        // append each rendering elsewhere, as NDJSON when the text is JSON
        else {
            await this.#output.write(`${text}\n`);
        }
    }

    /** Draw the next rendering below the last, such as after a question moved the cursor. */
    release(): void {
        this.#lines = 0;
    }
}
