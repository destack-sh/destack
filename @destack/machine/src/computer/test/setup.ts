import { CompilerPool, Workspace } from "@destack/build";
import { fileURLToPath } from "node:url";
import { testStore } from "@destack/build/test";
import { TestRun } from "@destack/db/test/run";
import { canonicalize, type Digest, present, Version } from "@destack/schema";
import type { TestProject } from "@destack/test";
import { testHost } from "@destack/test/host";
import {
    BUILDS_VARIABLE,
    DESKTOP_PACKAGE,
    PLATFORM_PACKAGES,
    RUNTIME_PACKAGE,
    STACK_FIXTURE,
    START_FIXTURE,
    TestDistribution,
} from "./distribution.ts";
import { FIXTURE_RELEASE, NEXT_RELEASE, SCENARIO_FIXTURES, WORLD_FIXTURES } from "./fixture.ts";
import { MachineDriver } from "../../test/driver.ts";
import { Toolchain } from "@destack/check/toolchain";
import { COMPUTER_RUNTIMES } from "../computer.ts";

/** Set a run up with a package's own fixtures beside the machine's, by name and directory. */
export function setupRun(
    fixtures: Readonly<Record<string, string>> = {},
): (project: TestProject) => Promise<() => Promise<void>> {
    return (project) => setup(project, fixtures);
}

/** Set the machine's own run up. */
export default setupRun();

/** Build the platform, the stacks and the fixtures, write the templates and make a trash. */
async function setup(
    project: TestProject,
    fixtures: Readonly<Record<string, string>>,
): Promise<() => Promise<void>> {
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
        const builds = [
            [RUNTIME_PACKAGE, built, true, () => TestDistribution.runtime(workspace)] as const,
            [DESKTOP_PACKAGE, built, true, () => TestDistribution.desktop(workspace)] as const,
            ...PLATFORM_PACKAGES.map(
                (name) =>
                    [name, built, true, () => TestDistribution.platform(name, workspace)] as const,
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
                            ),
                    ] as const,
            ),
            ...Object.entries(fixtures).map(
                ([name, directory]) =>
                    [
                        name,
                        scenario,
                        false,
                        () => TestDistribution.compile(workspace, directory, FIXTURE_RELEASE),
                    ] as const,
            ),
        ];
        await Promise.all(
            builds.map(async ([name, digests, isShipped, build]) => {
                // make an image of the package, and take each unit a shipped package runs with
                const { image, units } = await build();
                digests[name] = image;
                if (isShipped) {
                    for (const [unit, unitImage] of units) {
                        built[unit] ??= unitImage;
                    }
                }
            }),
        );

        // make an image of the next release of the fixture updates install
        built[NEXT_RELEASE.name] = await TestDistribution.release(
            store,
            present(built[NEXT_RELEASE.fixture], `the run's image of ${NEXT_RELEASE.fixture}`),
            NEXT_RELEASE.version,
        );

        // write the layer every computer ships once here, so no test pays for it
        const shipped = await TestDistribution.shipped(store, built, []);
        await store.writeLinkedLayer(shipped, COMPUTER_RUNTIMES);
    }

    // hand the images to the tests
    project.provide("built", built);
    project.provide("scenarios", scenario);
    const variables = { [BUILDS_VARIABLE]: canonicalize(built) };
    const remove = await TestRun.setup(new URL(import.meta.url), variables)(project);

    // move the computers' state directories into one trash, removed with the run, as Bazel's execroots
    const { files } = testHost();
    const trash = await files.makeTemporaryDirectory("destack-machine-trash-");
    project.provide("trash", trash);

    return async () => {
        await remove();
        await files.remove(trash, { recursive: true, force: true });
    };
}
