import { present } from "@destack/schema";
import { type Event, Shell } from "../shell/index.ts";

/** A shell answering no events, with the commands the host sent it. */
export function recordShell(): { readonly shell: Shell; readonly sent: unknown[] } {
    // record each command sent, and send no events
    const sent: unknown[] = [];
    const silent = new ReadableStream<Uint8Array>({ start: (controller) => controller.close() });
    const shell = new Shell(async (line) => void sent.push(JSON.parse(line)), silent);

    return { shell, sent };
}

/** A shell recording the host's commands and sending the test's events, open until closed. */
export function openShell(): {
    readonly shell: Shell;
    readonly sent: unknown[];
    readonly next: (isMatch: (command: unknown) => boolean) => Promise<unknown>;
    readonly emit: (event: Event) => void;
    readonly close: () => void;
} {
    // record each command sent, answering each wait its command matches
    const sent: unknown[] = [];
    const waits = new Set<{
        isMatch: (command: unknown) => boolean;
        resolve: (command: unknown) => void;
    }>();
    const record = (command: unknown) => {
        sent.push(command);
        for (const wait of waits) {
            if (wait.isMatch(command)) {
                waits.delete(wait);
                wait.resolve(command);
            }
        }
    };

    // send each event the test emits as a line, and close the pipe once the test asks
    const { promise: closed, resolve: close } = Promise.withResolvers<void>();
    const encoder = new TextEncoder();
    let pipe: ReadableStreamDefaultController<Uint8Array> | undefined;
    const events = new ReadableStream<Uint8Array>({
        start: async (controller) => {
            pipe = controller;
            await closed;
            controller.close();
        },
    });
    const emit = (event: Event) => {
        present(pipe, "the event pipe").enqueue(encoder.encode(`${JSON.stringify(event)}\n`));
    };
    const shell = new Shell(async (line) => record(JSON.parse(line)), events);

    return {
        shell,
        sent,
        next: (isMatch) => {
            const { promise, resolve } = Promise.withResolvers<unknown>();
            waits.add({ isMatch, resolve });

            return promise;
        },
        emit,
        close: () => close(),
    };
}
