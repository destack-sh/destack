import { testSimulation } from "@destack/object/test";
import { join } from "node:path";
import { expect, test, refusal } from "@destack/test";
import { Release, Stage } from "../release/index.ts";
import { Distribution, type DistributionEnvironment } from "./distribution.ts";

test("keep a distribution's record across reopening and refuse changing its installer", async () => {
    // keep the record apart from the executable distribution, in a home of its own
    const environment = testSimulation().environment("test");
    const directory = await environment.files.makeTemporaryDirectory("destack-registration-");
    try {
        const distribution = distributionAt(directory, environment);
        expect(await distribution.record()).toBeUndefined();
        const original = {
            method: "application" as const,
            application: join(directory, "Destack.app"),
        };
        await distribution.register(original);
        expect(await distributionAt(directory, environment).record()).toEqual(original);

        // keep the same installer when the person moves the application
        const moved = { ...original, application: join(directory, "Applications/Destack.app") };
        await distribution.register(moved);
        await distribution.register(moved);
        expect(await distributionAt(directory, environment).record()).toEqual(moved);

        // refuse another installer in the same home, keeping the record
        expect(await refusal(distribution.register({ ...moved, method: "nsis" }))).toEqual([
            "PRECONDITION_FAILED",
            "INSTALLATION_METHOD_MISMATCH",
            "uninstall the existing installation before changing its method",
        ]);
        expect(await distribution.record()).toEqual(moved);
    } finally {
        await environment.files.remove(directory, { recursive: true });
    }
});

/** Describe the stable distribution in a home, as a person's device keeps it. */
function distributionAt(home: string, environment: DistributionEnvironment): Distribution {
    return new Distribution({
        home,
        repository: new URL("https://download.destack.test/stable/"),
        root: Distribution.root(),
        running: new Release("2026.9.0", "aarch64-apple-darwin"),
        executable: join(home, "bin", "destack"),
        applications: join(home, "Applications"),
        stage: new Stage("production"),
        environment,
    });
}
