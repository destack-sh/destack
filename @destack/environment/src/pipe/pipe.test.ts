import { expect, test } from "@destack/test";
import { Pipe } from "./pipe.ts";

/** Yield some words, then fail when given a failure. */
async function* words(list: readonly string[], broken?: Error): AsyncGenerator<string> {
    yield* list;
    if (broken !== undefined) {
        throw broken;
    }
}

/** Read the message a pending value fails with, or done once it settles. */
function failure(pending: Promise<unknown>): Promise<string> {
    return pending.then(
        () => "done",
        (error: unknown) => (error instanceof Error ? error.message : String(error)),
    );
}

/** Upper-case each chunk. */
function upper(): TransformStream<string, string> {
    return new TransformStream({
        transform: (chunk, controller) => controller.enqueue(chunk.toUpperCase()),
    });
}

test("run a source through a transform, and fail the output with the source's failure or an abort", async () => {
    // read a whole source through the transform
    const read = await Array.fromAsync(Pipe.through(words(["a", "b"]), upper()));

    // read until the source fails, then refuse the output with its failure
    const failed = await failure(
        Array.fromAsync(Pipe.through(words(["a"], new Error("source broke")), upper())),
    );

    // refuse the output once the signal aborts
    const controller = new AbortController();
    controller.abort(new Error("stopped"));
    const aborted = await failure(
        Array.fromAsync(Pipe.through(words(["a"]), upper(), controller.signal)),
    );
    expect({ read, failed, aborted }).toEqual({
        read: ["A", "B"],
        failed: "source broke",
        aborted: "stopped",
    });
});

test("write a source into a writable and close it, or abort it with the source's failure", async () => {
    // collect what the writable receives and how it ends
    const received: string[] = [];
    let ending = "open";
    const sink = () =>
        new WritableStream<string>({
            write: (chunk) => {
                received.push(chunk);
            },
            close: () => {
                ending = "closed";
            },
            abort: (reason: unknown) => {
                ending = `aborted: ${reason instanceof Error ? reason.message : String(reason)}`;
            },
        });

    // write a whole source, then one that fails
    await Pipe.to(words(["a", "b"]), sink());
    const closed = ending;
    const failed = await failure(Pipe.to(words(["c"], new Error("source broke")), sink()));
    expect({ received, closed, failed, ending }).toEqual({
        received: ["a", "b", "c"],
        closed: "closed",
        failed: "source broke",
        ending: "aborted: source broke",
    });
});
