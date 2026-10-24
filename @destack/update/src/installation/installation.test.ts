import { testSimulation } from "@destack/object/test";
import { join } from "node:path";
import { expect, test, refusal } from "@destack/test";
import { Installation } from "./installation.ts";

test("retain application registration across reopening and refuse installer conversion", async () => {
    // keep application registration separate from the executable distribution
    const world = testSimulation().environment("test");
    const directory = await world.files.makeTemporaryDirectory("destack-registration-");
    try {
        const installation = new Installation(directory, world);
        expect(await installation.read()).toBeUndefined();
        const original = {
            method: "application" as const,
            application: join(directory, "Destack.app"),
        };
        await installation.register(original);
        expect(await new Installation(directory, world).read()).toEqual(original);

        // retain the same installer when the user moves the application
        const moved = { ...original, application: join(directory, "Applications/Destack.app") };
        await installation.register(moved);
        await installation.register(moved);
        expect(await new Installation(directory, world).read()).toEqual(moved);

        // preserve registration when an incompatible installer requests the same user directory
        expect(await refusal(installation.register({ ...moved, method: "nsis" }))).toEqual([
            "PRECONDITION_FAILED",
            "INSTALLATION_METHOD_MISMATCH",
            "uninstall the existing installation before changing its method",
        ]);
        expect(await installation.read()).toEqual(moved);
    } finally {
        await world.files.remove(directory, { recursive: true });
    }
});
