import { expect, test } from "@destack/test";
import { Autostart, start, stop } from "@destack/daemon/process";
import { LocalClient } from "@destack/daemon/client/local";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { buildExecutable } from "./executable.ts";

test.skipIf(process.platform !== "darwin" || process.env.DESTACK_TEST_AUTOSTART !== "1")(
    "start, stop and restart a compiled daemon through a temporary LaunchAgent",
    async () => {
        // isolate the native registration and all persistent state
        const root = fileURLToPath(new URL("../../../../", import.meta.url));
        const directory = await mkdtemp(join(tmpdir(), "destack-autostart-"));
        const client = new LocalClient(directory);
        const registration = new Autostart(directory);
        const executable = join(directory, "destack-daemon");
        try {
            await buildExecutable({
                root,
                entrypoint: join(root, "@destack/daemon/src/main.ts"),
                outfile: executable,
                target: process.arch === "arm64" ? "aarch64-apple-darwin" : "x86_64-apple-darwin",
                runtime: process.arch === "arm64" ? "bun-darwin-arm64" : "bun-darwin-x64",
                version: "2026.9.123-nightly.1",
            });
            await registration.register(executable);
            await start(executable, client, true);
            expect((await client.status()).version).toBe("2026.9.123-nightly.1");

            // require shutdown to release discovery before starting the same installed executable
            await stop(client);
            expect(await client.inspect()).toEqual({ status: "disconnected" });
            await start(executable, client, true);
            expect((await client.status()).version).toBe("2026.9.123-nightly.1");
            await stop(client);
        } finally {
            await registration.unregister();
            await rm(directory, { recursive: true, force: true });
        }
    },
);
