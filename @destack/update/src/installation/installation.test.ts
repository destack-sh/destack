import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { expect, test } from "@destack/test";
import { Installation } from "./installation.ts";
import { UpdateError } from "../error/index.ts";

test("retain application registration across reopening and refuse installer conversion", async () => {
    // keep application registration separate from the executable distribution
    const directory = await mkdtemp(join(tmpdir(), "destack-registration-"));
    try {
        const installation = new Installation(directory);
        expect(await installation.read()).toBeUndefined();
        const original = {
            method: "application" as const,
            application: join(directory, "Destack.app"),
        };
        await installation.register(original);
        expect(await new Installation(directory).read()).toEqual(original);

        // retain the same installer when the user moves the application
        const moved = { ...original, application: join(directory, "Applications/Destack.app") };
        await installation.register(moved);
        await installation.register(moved);
        expect(await new Installation(directory).read()).toEqual(moved);

        // preserve registration when an incompatible installer requests the same user directory
        await expect(installation.register({ ...moved, method: "nsis" })).rejects.toEqual(
            new UpdateError(
                "INSTALL",
                "uninstall the existing installation before changing its method",
            ),
        );
        expect(await installation.read()).toEqual(moved);
    } finally {
        await rm(directory, { recursive: true, force: true });
    }
});
