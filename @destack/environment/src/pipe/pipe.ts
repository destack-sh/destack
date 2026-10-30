/** Pipes that read and write streams directly, each step seen by a world's scheduler. */
export const Pipe = {
    /** Run a source through a transform, failing the output with the source's failure. */
    through<In, Out>(
        source: AsyncIterable<In>,
        pair: ReadableWritablePair<Out, In>,
        signal?: AbortSignal,
    ): ReadableStream<Out> {
        // write the source, its failure reaching the reader through the aborted writable
        void Pipe.to(source, pair.writable, signal).catch(() => undefined);

        return pair.readable;
    },

    /** Write a source into a writable chunk by chunk, then close it, aborting it on a failure. */
    async to<In>(
        source: AsyncIterable<In>,
        writable: WritableStream<In>,
        signal?: AbortSignal,
    ): Promise<void> {
        // abort the writable once the signal aborts
        const writer = writable.getWriter();
        const abort = () => void writer.abort(signal?.reason).catch(() => undefined);
        signal?.addEventListener("abort", abort, { once: true });
        try {
            // write each chunk, then close
            signal?.throwIfAborted();
            for await (const chunk of source) {
                signal?.throwIfAborted();
                await writer.write(chunk);
            }
            await writer.close();
        } catch (error) {
            // fail the writable with the source's failure, then the caller
            await writer.abort(error).catch(() => undefined);
            throw error;
        } finally {
            signal?.removeEventListener("abort", abort);
        }
    },
};
