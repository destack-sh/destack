import { FileLock } from "../lock.ts";

// retain ownership until the parent closes stdin or terminates this process
await using lock = await FileLock.acquire(process.argv[2]);
console.log("locked");

// release when the parent closes stdin
for await (const chunk of process.stdin) {
    if (chunk.length) {
        throw new Error("lock holder expects stdin closure");
    }
}
