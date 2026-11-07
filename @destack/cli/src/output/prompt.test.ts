import { expect, refusal, test } from "@destack/test";
import { Prompt } from "./prompt.ts";

/** A standard error that keeps nothing, for a piped prompt that asks silently. */
const SILENT = {
    write: async () => {},
    writeSync: () => {},
    isTerminal: false,
};

test("read an answer from piped input, then cancel the open input once disposed", async () => {
    // pipe one answer on an input that stays open, as a fifo or an idle terminal does
    const cancelled = Promise.withResolvers<unknown>();
    const readable = new ReadableStream<Uint8Array>({
        start: (controller) => controller.enqueue(new TextEncoder().encode("238231\n")),
        cancel: (reason) => cancelled.resolve(reason),
    });
    const prompt = new Prompt({ stdin: { readable, isTerminal: false }, stderr: SILENT });

    // answer the code, then release the input as a command line's end does
    const answer = await prompt.ask("Code");
    await prompt[Symbol.asyncDispose]();

    // answer the piped line, and cancel the input, which lets the process end
    expect({ answer, cancelled: await cancelled.promise }).toEqual({
        answer: "238231",
        cancelled: undefined,
    });
});

test("cancel input a question still waits on once disposed, ending the question", async () => {
    // ask on an input that never sends a line
    const readable = new ReadableStream<Uint8Array>();
    const prompt = new Prompt({ stdin: { readable, isTerminal: false }, stderr: SILENT });
    const asking = refusal(() => prompt.ask("Code"));

    // release the input while the question waits
    await prompt[Symbol.asyncDispose]();

    // refuse the question as input that ended
    expect(await asking).toEqual([
        "BAD_REQUEST",
        "USAGE_INVALID",
        "standard input ended before the answer to: Code",
    ]);
});
