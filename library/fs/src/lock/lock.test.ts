import { expect, test } from "@destack/test";
import { spawn } from "node:child_process";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { createInterface } from "node:readline";
import { FileLock } from "./lock.ts";
import { FileSystemError } from "../error/index.ts";
import { setTimeout } from "node:timers/promises";

test.for(["release", "terminate"] as const)(
    "recover exclusive ownership after %s",
    async (mode) => {
        const directory = await mkdtemp(join(tmpdir(), "destack-file-lock-"));
        const path = join(directory, "owner.lock");
        await writeFile(path, "preserve this file");
        const child = spawn(
            process.execPath,
            [
                "run",
                "--no-env-file",
                fileURLToPath(new URL("./tests/hold.ts", import.meta.url)),
                path,
            ],
            { stdio: ["pipe", "pipe", "inherit"], timeout: 3000, killSignal: "SIGKILL" },
        );
        const exited = new Promise<{ code: number | null; signal: string | null }>(
            (resolve, reject) => {
                child.once("error", reject);
                child.once("exit", (code, signal) => resolve({ code, signal }));
            },
        );
        try {
            // observe real acquisition before probing from a separate process
            const lines = createInterface({ input: child.stdout });
            const ready = await lines[Symbol.asyncIterator]().next();
            expect(ready.value).toBe("locked");
            lines.close();
            expect(await FileLock.tryAcquire(path)).toBeUndefined();

            // cancel a waiting acquisition without disturbing the current owner
            const controller = new AbortController();
            const waiting = FileLock.acquire(path, { signal: controller.signal });
            const rejected = expect(waiting).rejects.toMatchObject({ name: "AbortError" });
            await setTimeout(30);
            controller.abort();
            await rejected;
            expect(await FileLock.tryAcquire(path)).toBeUndefined();

            // recover ownership after either explicit release or abrupt process termination
            const acquired = FileLock.acquire(path, { signal: AbortSignal.timeout(1000) });
            if (mode === "release") {
                child.stdin.end();
            } else {
                child.kill("SIGKILL");
            }
            await using owner = await acquired;
            const termination = await exited;
            if (mode === "release") {
                expect(termination).toEqual({ code: 0, signal: null });
            }

            // preserve the file through repeated close and subsequent acquisition
            expect(await FileLock.tryAcquire(path)).toBeUndefined();
            await owner.close();
            await owner.close();
            expect(await readFile(path, "utf8")).toBe("preserve this file");
            await using next = await FileLock.tryAcquire(path);
            expect(next?.path).toBe(path);
        } finally {
            if (child.exitCode === null && child.signalCode === null) {
                child.kill("SIGKILL");
            }
            await exited;
            await rm(directory, { recursive: true });
        }
    },
);

test("reject invalid paths without changing the filesystem", async () => {
    const directory = await mkdtemp(join(tmpdir(), "destack-file-lock-"));
    try {
        const path = join(directory, "missing", "owner.lock");
        await expect(FileLock.tryAcquire(path)).rejects.toMatchObject({
            name: "FileSystemError",
            operation: "open",
            path,
            code: process.platform === "win32" ? 3 : "ENOENT",
        } satisfies Partial<FileSystemError>);

        // preserve native string termination across both platform implementations
        const invalid = join(directory, "owner.lock") + "\0suffix";
        await expect(FileLock.tryAcquire(invalid)).rejects.toMatchObject({
            name: "FileSystemError",
            operation: "open",
            path: invalid,
            code: "EINVAL",
        } satisfies Partial<FileSystemError>);
    } finally {
        await rm(directory, { recursive: true });
    }
});
