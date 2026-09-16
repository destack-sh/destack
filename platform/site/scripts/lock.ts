import { mkdirSync, rmdirSync } from "node:fs";
import { setTimeout } from "node:timers/promises";

/// Run one content generation at a time across processes.
export async function withLock<T>(directory: string, action: () => Promise<T>, timeout = 30000) {
    const deadline = Date.now() + timeout;

    // wait for the active writer before reading or publishing content
    for (; ;) {
        try {
            mkdirSync(directory);
            break;
        } catch (error) {
            if (!(error instanceof Error && "code" in error && error.code === "EEXIST")) throw error;
            if (Date.now() >= deadline) {
                throw new Error(
                    `Timed out waiting for ${directory}. If no generator is running, remove this stale lock directory.`,
                );
            }
            await setTimeout(25);
        }
    }

    // release the directory on completion, failure, or a normal process exit
    const release = () => rmdirSync(directory);
    const interrupt = () => process.exit(130);
    const terminate = () => process.exit(143);
    process.once("exit", release);
    process.once("SIGINT", interrupt);
    process.once("SIGTERM", terminate);
    try {
        return await action();
    } finally {
        process.removeListener("exit", release);
        process.removeListener("SIGINT", interrupt);
        process.removeListener("SIGTERM", terminate);
        release();
    }
}
