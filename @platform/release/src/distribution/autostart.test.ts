import { expect, test } from "@destack/test";
import { testHost } from "@destack/test/host";
import { MACHINE_ENTRY } from "@destack/desktop/workload";
import { Autostart, MachineProcess } from "@destack/machine/computer";
import { Device } from "@destack/machine/device";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { buildExecutable, toolchainOptions } from "./executable.ts";

/** The real host the compiled machine registers on. */
const HOST = testHost();

/** Whether the host is an arm64 Mac. */
const isArm = HOST.host.architecture === "arm64";

test.skipIf(HOST.host.os !== "darwin" || HOST.process.variables["DESTACK_TEST_AUTOSTART"] !== "1")(
    "start, stop and restart a compiled machine through a temporary LaunchAgent",
    async () => {
        // isolate the native registration and all persistent state
        const root = fileURLToPath(new URL("../../../..", import.meta.url));
        const host = HOST;
        const directory = await host.files.makeTemporaryDirectory("destack-autostart-");
        const registration = await Autostart.of(directory, host);
        const executable = join(directory, "destack-machine");
        try {
            await buildExecutable(
                {
                    root,
                    entrypoint: join(root, "@destack/desktop/src/machine.ts"),
                    outfile: executable,
                    target: isArm ? "aarch64-apple-darwin" : "x86_64-apple-darwin",
                    runtime: isArm ? "bun-darwin-arm64" : "bun-darwin-x64",
                    version: "2026.9.123",
                    ...toolchainOptions(host.files),
                },
                host,
            );
            await registration.register(executable);
            const machine = new MachineProcess(directory, host);
            await machine.start(MACHINE_ENTRY, executable, true);
            expect((await machine.loopback.computer()).version).toBe("2026.9.123");

            // require shutdown to release discovery before starting the same installed executable
            await machine.stop();
            expect(await Device.inspect(directory, host)).toEqual({ kind: "stopped" });
            await machine.start(MACHINE_ENTRY, executable, true);
            expect((await machine.loopback.computer()).version).toBe("2026.9.123");
            await machine.stop();
        } finally {
            await registration.unregister();
            await host.files.remove(directory, { recursive: true, force: true });
        }
    },
);
