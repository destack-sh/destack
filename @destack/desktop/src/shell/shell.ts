import type { Environment } from "@destack/environment";
import { type Command, Event } from "./protocol.ts";
import { DesktopError } from "../error/index.ts";

/** The native shell on the other end of the host's private pipes: commands out, events in. */
export class Shell {
    /** Write one line to the shell. */
    readonly #write: (line: string) => Promise<void>;
    /** The stream of the shell's event lines. */
    readonly #input: ReadableStream<Uint8Array>;

    /** Talk to the shell through a line writer and an event stream. */
    constructor(write: (line: string) => Promise<void>, input: ReadableStream<Uint8Array>) {
        this.#write = write;
        this.#input = input;
    }

    /** Talk to the shell over a process's standard output and input, its private pipes. */
    static of(running: Environment<"process">["process"]): Shell {
        return new Shell((line) => running.stdout.write(line), running.stdin.readable);
    }

    /** Send one command as one line of the pipe the shell reads, which refuses any other line. */
    send(command: Command): Promise<void> {
        return this.#write(`${JSON.stringify(command)}\n`);
    }

    /** Read the shell's events until it closes the pipe, refusing a line that is no event. */
    async *events(): AsyncIterable<Event> {
        // read each complete line as it arrives, holding back a partial one
        const decoder = new TextDecoder();
        let rest = "";
        for await (const bytes of this.#input) {
            const text = `${rest}${decoder.decode(bytes, { stream: true })}`;
            const end = text.lastIndexOf("\n");
            rest = text.slice(end + 1);
            const lines = end === -1 ? [] : text.slice(0, end).split("\n");
            for (const line of lines) {
                yield Shell.#event(line.replace(/\r$/u, ""));
            }
        }

        // refuse a pipe closed in the middle of a line
        const last = `${rest}${decoder.decode()}`;
        if (last !== "") {
            throw DesktopError.of(
                "EVENT_INVALID",
                `shell event '${last}' is invalid: the shell closed its pipe within a line`,
            );
        }
    }

    /** Read one line as an event, refusing one that is no event. */
    static #event(line: string): Event {
        const parsed = Event.safeParse(Shell.#json(line));
        if (!parsed.success) {
            throw DesktopError.of(
                "EVENT_INVALID",
                `shell line '${line}' is invalid: it must be an event`,
            );
        }

        return parsed.data;
    }

    /** Read one line as JSON, refusing a line that is not. */
    static #json(line: string): unknown {
        try {
            return JSON.parse(line);
        } catch (error) {
            throw DesktopError.of(
                "EVENT_INVALID",
                `shell line '${line}' is invalid: it must be JSON`,
                {
                    cause: error,
                },
            );
        }
    }
}
