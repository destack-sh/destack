import { CliError } from "../error/index.ts";
import type { TerminalProcess } from "./output.ts";
import { nth } from "@destack/schema";

/** The lines a person types or a script pipes, read one answer at a time. */
export class Prompt implements AsyncDisposable {
    /** The process whose standard input the answers come from. */
    readonly #process: Pick<TerminalProcess, "stdin" | "stderr">;
    /** The reader of standard input, taken on the first question. */
    #reader: Pick<ReadableStreamDefaultReader<Uint8Array>, "cancel"> | undefined;
    /** The lines of standard input, read on the first question. */
    #lines: AsyncIterator<string> | undefined;

    /** Read answers from a process's standard input. */
    constructor(process: Pick<TerminalProcess, "stdin" | "stderr">) {
        this.#process = process;
    }

    /** Ask a question on a terminal, or read the next piped line, refusing the end of input. */
    async ask(question: string): Promise<string> {
        // ask on the terminal, or read silently from a pipe
        const { stdin, stderr } = this.#process;
        if (stdin.isTerminal) {
            await stderr.write(`${question}: `);
        }
        this.#lines ??= this.#read();
        const next = await this.#lines.next();
        if (next.done === true) {
            throw CliError.of(
                "USAGE_INVALID",
                `standard input is invalid: it ended before the answer to '${question}'`,
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
        this.#lines ??= this.#read();
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

    /** Stop reading standard input, cancelling a waiting read so the process can end. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.#reader?.cancel();
    }

    /** Read the lines of standard input as they arrive, ending carriage returns dropped. */
    async *#read(): AsyncGenerator<string> {
        // split the decoded text at each line feed, leaving the partial rest for the next read
        const reader = this.#process.stdin.readable.getReader();
        this.#reader = reader;
        const decoder = new TextDecoder();
        let rest = "";
        for (let read = await reader.read(); !read.done; read = await reader.read()) {
            const lines = `${rest}${decoder.decode(read.value, { stream: true })}`.split("\n");
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
