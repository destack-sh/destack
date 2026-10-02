import { FileLock } from "../lock.ts";

/** The lock file the parent passes as the first argument. */
const path = process.argv[2];
if (path === undefined) {
    throw new Error("lock holder expects a lock path argument");
}

/** The lock retained until the parent closes stdin or terminates this process. */
await using lock = await FileLock.acquire(path);
process.stdout.write("locked\n");

// release when the parent closes stdin
for await (const chunk of Bun.stdin.stream()) {
    if (chunk.length !== 0) {
        throw new Error("lock holder expects stdin closure");
    }
}
