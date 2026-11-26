import { join } from "node:path";
import { pathToFileURL } from "node:url";
import { expect, refusal, test } from "@destack/test";
import { testHost } from "@destack/test/host";
import { MachineProcess } from "./process.ts";

/** A machine entry that publishes its endpoint and stays up. */
const READY = pathToFileURL(join(import.meta.dirname, "test/ready.ts")).href;

/** A machine entry that exits before it serves. */
const EXIT = pathToFileURL(join(import.meta.dirname, "test/exit.ts")).href;

test("start a machine once it notifies its ready socket, refusing one that exits first", async () => {
    // start a machine that publishes its endpoint, and one that exits at once
    const environment = testHost();
    const { files, process } = environment;
    const started = await files.makeTemporaryDirectory("process-");
    const exited = await files.makeTemporaryDirectory("process-");
    await new MachineProcess(started, environment).start(READY, process.executable, false);
    const refused = await refusal(
        new MachineProcess(exited, environment).start(EXIT, process.executable, false),
    );

    // leave no ready socket behind, and report the exit with the machine's log
    expect({
        sockets: [
            await files.exists(join(started, "ready.sock")),
            await files.exists(join(exited, "ready.sock")),
        ],
        refused,
    }).toEqual({
        sockets: [false, false],
        refused: [
            "INTERNAL_SERVER_ERROR",
            "START_BROKEN",
            `machine exited while starting: its log '${join(exited, "machine.log")}' ends with ''`,
        ],
    });

    await files.remove(started, { recursive: true });
    await files.remove(exited, { recursive: true });
});
