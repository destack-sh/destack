import { expect, test } from "@destack/test";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { UpdateProcess } from "./process.ts";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";

test("authenticate desktop shutdown and wait for process exit", async () => {
    // start a real native process with a private registration
    const directory = await mkdtemp(join(tmpdir(), "destack-update-process-"));
    const child = Bun.spawn(
        [
            process.execPath,
            "run",
            "--no-env-file",
            fileURLToPath(new URL("./fixture.ts", import.meta.url)),
            directory,
        ],
        {
            stdout: "pipe",
            stderr: "inherit",
        },
    );
    let isExited = false;
    const status = child.exited.then((result) => {
        isExited = true;
        return result;
    });
    try {
        const reader = child.stdout.getReader();
        // read the readiness line independently of pipe chunk boundaries
        let ready = "";
        const decoder = new TextDecoder();
        while (!ready.includes("\n")) {
            const chunk = await reader.read();
            if (chunk.done) {
                throw new Error("desktop process exited before reporting readiness");
            }
            ready += decoder.decode(chunk.value, { stream: true });
        }
        expect(ready).toBe("ready\n");
        reader.releaseLock();
        const endpoint = JSON.parse(await readFile(join(directory, "desktop.json"), "utf8"));

        // reject browser requests and credentials from another caller
        const requests: Record<string, string>[] = [
            { authorization: "Bearer wrong" },
            {
                authorization: `Bearer ${endpoint.token}`,
                origin: "https://example.com",
            },
        ];
        for (const headers of requests) {
            const response = await fetch(`http://127.0.0.1:${endpoint.port}/_destack/update/stop`, {
                method: "POST",
                headers,
            });
            expect(response.status).toBe(403);
            await response.body?.cancel();
        }
        await expect(
            UpdateProcess.register(directory, endpoint.port, async () => {}),
        ).rejects.toThrow("Destack desktop is already running.");

        // authenticate shutdown and ignore stale registration after the process exits
        expect(await UpdateProcess.stop(directory)).toBe(true);
        expect(await status).toBe(0);
        expect(await UpdateProcess.stop(directory)).toBe(false);
    } finally {
        if (!isExited) {
            child.kill("SIGKILL");
        }
        await status;
        await rm(directory, { recursive: true });
    }
});
