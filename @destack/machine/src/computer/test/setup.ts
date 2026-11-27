import { CompilerPool, Workspace } from "@destack/build";
import { fileURLToPath } from "node:url";
import { testStore } from "@destack/build/test";
import { type RunRecord, TestRun } from "@destack/db/test/run";
import { TestDatabase } from "@destack/db/test";
import { canonicalize, Digest, present, Version } from "@destack/schema";
import type { TestProject } from "@destack/test";
import { testHost } from "@destack/test/host";
import {
    BUILDS_VARIABLE,
    SCENARIOS_VARIABLE,
    DESKTOP_PACKAGE,
    PLATFORM_PACKAGES,
    RUNTIME_PACKAGE,
    STACK_FIXTURE,
    START_FIXTURE,
    TestDistribution,
} from "./distribution.ts";
import {
    FIXTURE_RELEASE,
    NEXT_RELEASE,
    SCENARIO_FIXTURES,
    TestFixture,
    WORLD_FIXTURES,
} from "./fixture.ts";
import { MachineDriver } from "../../test/driver.ts";
import { Toolchain } from "@destack/check/toolchain";
import { COMPUTER_RUNTIMES } from "../computer.ts";
import { TelemetryScope } from "@destack/telemetry";

/** Set a run up with a package's own fixtures beside the machine's, by name and directory. */
export function setupRun(
    fixtures: Readonly<Record<string, string>> = {},
    images: readonly RunImage[] = [],
): (project: TestProject) => Promise<() => Promise<void>> {
    return (project) => setup(project, fixtures, images);
}

/** Build an image a package's tests fork in the run, before any test starts. */
type RunImage = (run: RunRecord) => Promise<unknown>;

/** Set the machine's own run up. */
export default setupRun();

/** Build the platform, the stacks and the fixtures, write the templates and make a trash. */
async function setup(
    project: TestProject,
    fixtures: Readonly<Record<string, string>>,
    images: readonly RunImage[],
): Promise<() => Promise<void>> {
    // measure each phase in the host's telemetry
    const host = testHost();
    const { span, log } = TelemetryScope.of(import.meta.destack.package, host.telemetry);
    const phase = <Result>(name: string, work: () => Promise<Result>) =>
        span(`destack.test.setup.${name}`, {}, work, { level: "info" });

    // build the platform's packages, the stacks and the fixtures at once, so each world only reads the run's store
    const built: Record<string, Digest> = {};
    const scenario: Record<string, string> = {};
    {
        await using store = await testStore();
        await using compilers = await CompilerPool.open(store, async () => Toolchain.workspace());
        const workspace = await Workspace.open(
            fileURLToPath(new URL("../../../", import.meta.url)),
            compilers,
        );
        // ship the runtime, the desktop, the platform packages and the stacks with their units, as a platform build does
        const shipping = new Set<string>([STACK_FIXTURE, START_FIXTURE]);

        // stamp the shipped packages in every build, so a unit several packages reach is one build
        const fixtureNames = await Promise.all(
            [...shipping].map(
                async (fixture) =>
                    (await workspace.package(new TestFixture(fixture).directory)).root.name,
            ),
        );
        const shipped = Object.fromEntries(
            [RUNTIME_PACKAGE, DESKTOP_PACKAGE, ...PLATFORM_PACKAGES, ...fixtureNames].map(
                (name) => [name, Version.development] as const,
            ),
        );
        const builds = [
            [
                RUNTIME_PACKAGE,
                built,
                true,
                () => TestDistribution.runtime(workspace, shipped),
            ] as const,
            [
                DESKTOP_PACKAGE,
                built,
                true,
                () => TestDistribution.desktop(workspace, shipped),
            ] as const,
            ...PLATFORM_PACKAGES.map(
                (name) =>
                    [
                        name,
                        built,
                        true,
                        () => TestDistribution.platform(name, workspace, shipped),
                    ] as const,
            ),
            ...[...WORLD_FIXTURES, STACK_FIXTURE, START_FIXTURE, ...SCENARIO_FIXTURES].map(
                (name) =>
                    [
                        name,
                        SCENARIO_FIXTURES.some((each) => each === name) ? scenario : built,
                        shipping.has(name),
                        () =>
                            MachineDriver.build(
                                name,
                                workspace,
                                shipping.has(name) ? Version.development : FIXTURE_RELEASE,
                                shipped,
                            ),
                    ] as const,
            ),
            ...Object.entries(fixtures).map(
                ([name, directory]) =>
                    [
                        name,
                        scenario,
                        false,
                        () =>
                            TestDistribution.compile(
                                workspace,
                                directory,
                                FIXTURE_RELEASE,
                                shipped,
                            ),
                    ] as const,
            ),
        ];
        await phase("build", () =>
            Promise.all(
                builds.map(async ([name, digests, isShipped, build]) => {
                    // make an image of the package, and take each unit a shipped package runs with
                    const { image, units } = await span(
                        "destack.test.setup.package",
                        { "destack.package.name": name },
                        build,
                    );
                    digests[name] = image;
                    if (isShipped) {
                        for (const [unit, unitImage] of units) {
                            // refuse two images of one unit, as every build stamps the same versions
                            const recorded = built[unit];
                            if (recorded !== undefined && recorded !== unitImage) {
                                throw new TypeError(
                                    `unit '${unit}' is invalid: it has images '${recorded}' and '${unitImage}'`,
                                );
                            }
                            built[unit] = unitImage;
                        }
                    }
                }),
            ),
        );

        // make an image of the next release of the fixture updates install
        built[NEXT_RELEASE.name] = await phase("release", () =>
            TestDistribution.release(
                store,
                present(built[NEXT_RELEASE.fixture], `the run's image of ${NEXT_RELEASE.fixture}`),
                NEXT_RELEASE.version,
            ),
        );

        // write the layer every computer ships once here, so no test pays for it
        await phase("layer", async () => {
            const layered = await TestDistribution.shipped(store, built, []);
            await store.writeLinkedLayer(layered, COMPUTER_RUNTIMES);
        });
    }

    // hand the images to the tests and to the templates the setup builds
    project.provide("built", built);
    project.provide("scenarios", scenario);
    const variables = {
        [BUILDS_VARIABLE]: canonicalize(built),
        [SCENARIOS_VARIABLE]: canonicalize(scenario),
    };

    // open the run, building the images the package's tests fork, so no test pays for them
    for (const [name, image] of Object.entries(built)) {
        log.debug("destack.test.setup.image", {
            "destack.package.name": name,
            "destack.image": image,
        });
    }
    log.info("destack.test.setup.builds", {
        "destack.test.builds": await Digest.json(built, host.crypto),
    });
    const run = await phase("run", () =>
        TestRun.open(new URL(import.meta.url), TestDatabase.target(project), { variables }),
    );
    project.provide("run", run.record);
    await phase("images", async () => {
        for (const image of images) {
            await image(run.record);
        }
    });

    // move the computers' state directories into one trash, removed with the run, as Bazel's execroots
    const { files } = host;
    const trash = await files.makeTemporaryDirectory("destack-machine-trash-");
    project.provide("trash", trash);

    return async () => {
        await run.release();
        await files.remove(trash, { recursive: true, force: true });
    };
}
