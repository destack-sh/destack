import { expect, test } from "@destack/test";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { UpdateProcess } from "./process.ts";

test("authenticate desktop shutdown and wait for process exit", async () => {
    // start a real native process with a private registration
    const directory = await Deno.makeTempDir({ prefix: "destack-update-process-" });
    const process = new Deno.Command(Deno.execPath(), {
        args: ["run", "-A", fileURLToPath(new URL("./fixture.ts", import.meta.url)), directory],
        stdout: "piped",
        stderr: "inherit",
    }).spawn();
    let isExited = false;
    const status = process.status.then((result) => {
        isExited = true;
        return result;
    });
    try {
        const reader = process.stdout.getReader();
        const ready = await reader.read();
        expect(new TextDecoder().decode(ready.value)).toBe("ready\n");
        reader.releaseLock();
        const endpoint = JSON.parse(await Deno.readTextFile(join(directory, "desktop.json")));

        // reject browser requests and credentials from another caller
        for (
            const headers of [{ authorization: "Bearer wrong" }, {
                authorization: `Bearer ${endpoint.token}`,
                origin: "https://example.com",
            }]
        ) {
            const response = await fetch(`http://127.0.0.1:${endpoint.port}/_destack/update/stop`, {
                method: "POST",
                headers,
            });
            expect(response.status).toBe(403);
            await response.body?.cancel();
        }
        await expect(UpdateProcess.register(directory, endpoint.port, async () => {})).rejects
            .toThrow(
                "Destack desktop is already running.",
            );

        // authenticate shutdown and ignore stale registration after the process exits
        expect(await UpdateProcess.stop(directory)).toBe(true);
        expect(await status).toEqual({ success: true, code: 0, signal: null });
        expect(await UpdateProcess.stop(directory)).toBe(false);
    } finally {
        if (!isExited) process.kill("SIGKILL");
        await status;
        await Deno.remove(directory, { recursive: true });
    }
});
