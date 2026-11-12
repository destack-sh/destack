import { expect, test } from "@destack/test";
import { ChunkReader } from "./reader.ts";

/** The error a reader fails with when its stream ends early. */
const ENDED = "stream ends early";

/** Stream bytes in chunks of some sizes, counting the chunks taken and noting a stop. */
function chunked(sizes: readonly number[]): {
    readonly chunks: AsyncIterable<Uint8Array>;
    readonly state: { taken: number; isStopped: boolean };
} {
    const state = { taken: 0, isStopped: false };
    const chunks = (async function* () {
        try {
            let start = 0;
            for (const size of sizes) {
                state.taken += 1;
                const offset = start;
                yield new Uint8Array(size).map((_, index) => (offset + index) % 256);
                start += size;
            }
        } finally {
            state.isStopped = true;
        }
    })();

    return { chunks, state };
}

/** Read a reader's stream of some bytes as numbers. */
async function streamed(reader: ChunkReader, count: number): Promise<number[]> {
    const parts = await Array.fromAsync(reader.stream(count));

    return parts.flatMap((part) => [...part]);
}

test("read exact byte counts within and across chunks, then fail with the owner's error at an early end", async () => {
    // read a part of the first chunk, then across three chunks, then stream the rest
    const { chunks } = chunked([3, 2, 4]);
    const reader = new ChunkReader(chunks, () => new Error(ENDED));
    const first = [...(await reader.read(2))];
    const across = [...(await reader.read(5))];
    const rest = await streamed(reader, 2);

    // fail a read past the end with the given error
    const ended = await reader.read(1).then(
        () => "read",
        (error: unknown) => (error instanceof Error ? error.message : String(error)),
    );
    expect({ first, across, rest, ended }).toEqual({
        first: [0, 1],
        across: [2, 3, 4, 5, 6],
        rest: [7, 8],
        ended: ENDED,
    });
});

test("stop the stream's source when disposed before its end", async () => {
    // read one byte of a stream of three chunks, then dispose
    const { chunks, state } = chunked([1, 1, 1]);
    {
        await using reader = new ChunkReader(chunks, () => new Error(ENDED));
        await reader.read(1);
    }

    // take one chunk and stop the source
    expect(state).toEqual({ taken: 1, isStopped: true });
});
