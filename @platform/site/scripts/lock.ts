import { mkdirSync, rmdirSync } from "node:fs";
import { setTimeout } from "node:timers/promises";

/** Run one content generation at a time across processes. */
export async function withLock<Result>(
    directory: string,
    action: () => Promise<Result>,
    timeout = 30000,
) {
    // give up on a lock held past the timeout
    const deadline = Date.now() + timeout;

    // wait for the active writer before reading or publishing content
    for (;;) {
        try {
            mkdirSync(directory);
            break;
        } catch (error) {
            if (!(error instanceof Error && "code" in error && error.code === "EEXIST")) {
                throw error;
            }
            if (Date.now() >= deadline) {
                throw new Error(
                    `timed out waiting for ${directory}. If no generator is running, remove this stale lock directory`,
                    { cause: error },
                );
            }
            await setTimeout(25);
        }
    }

    // release the directory on completion, failure, or a normal process exit
    const release = () => rmdirSync(directory);
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

/** Exit on an interrupt signal with the shell's status for SIGINT. */
function interrupt() {
    process.exit(130);
}

/** Exit on a termination signal with the shell's status for SIGTERM. */
function terminate() {
    process.exit(143);
}
