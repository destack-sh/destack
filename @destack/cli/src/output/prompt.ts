import { CliError } from "../error/index.ts";
import type { TerminalProcess } from "./output.ts";
import { nth } from "@destack/schema";

/** The lines a person types or a script pipes, read one answer at a time. */
export class Prompt {
    /** The process whose standard input the answers come from. */
    readonly #process: TerminalProcess;
    /** The lines of standard input, read on the first question. */
    #lines: AsyncIterator<string> | undefined;

    /** Read answers from a process's standard input. */
    constructor(process: TerminalProcess) {
        this.#process = process;
    }

    /** Ask a question on a terminal, or read the next piped line, refusing the end of input. */
    async ask(question: string): Promise<string> {
        // ask on the terminal, or read silently from a pipe
        const { stdin, stderr } = this.#process;
        if (stdin.isTerminal) {
            await stderr.write(`${question}: `);
        }
        this.#lines ??= Prompt.#read(stdin.readable);
        const next = await this.#lines.next();
        if (next.done === true) {
            throw CliError.of(
                "USAGE_INVALID",
                `standard input ended before the answer to: ${question}`,
            );
        }

        return next.value.trim();
    }

    /** Ask a yes or no question, refusing anything but yes. */
    async confirm(question: string): Promise<boolean> {
        return (await this.ask(`${question} (yes/no)`)).toLowerCase() === "yes";
    }

    /** Read the rest of standard input as text. */
    async rest(): Promise<string> {
        // read every line not read yet
        this.#lines ??= Prompt.#read(this.#process.stdin.readable);
        const lines: string[] = [];
        for (
            let next = await this.#lines.next();
            next.done !== true;
            next = await this.#lines.next()
        ) {
            lines.push(next.value);
        }

        return lines.join("\n");
    }

    /** Read the lines of a stream as they arrive, ending carriage returns dropped. */
    static async *#read(readable: ReadableStream<Uint8Array>): AsyncGenerator<string> {
        // split the decoded text at each line feed, leaving the partial rest for the next read
        const decoder = new TextDecoder();
        let rest = "";
        for await (const bytes of readable) {
            const lines = `${rest}${decoder.decode(bytes, { stream: true })}`.split("\n");
            rest = nth(lines, -1);
            for (const line of lines.slice(0, -1)) {
                yield line.replace(/\r$/u, "");
            }
        }

        // answer a last line without a line feed
        if (rest !== "") {
            yield rest;
        }
    }
}
