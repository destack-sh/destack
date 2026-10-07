import type { Given, Scenario, SetStep, Interaction } from "@destack/package/declare";
import type { JsonValue, schema } from "@destack/schema";
import { expect, test } from "vitest";

/** A driver playing one scenario's steps and reading its observations, after WebDriver: a test DOM in process, or a real browser. */
export interface Driver<Speaks extends Interaction = Interaction> {
    /** Take one step. */
    act(step: SetStep | schema.Infer<Speaks["step"]>): void | Promise<void>;
    /** Read one observation. */
    observe(observation: schema.Infer<Speaks["observation"]>): JsonValue | Promise<JsonValue>;
}

/** The drivers of one interaction, each started on the examples a scenario is given. */
export interface DriverType<Speaks extends Interaction = Interaction, Instance = unknown> {
    /** The interaction the drivers speak. */
    readonly interaction: Speaks;
    /** Start a driver on the examples a scenario is given, in its environment. */
    start(given: Given<Instance>): Driver<Speaks> | Promise<Driver<Speaks>>;
}

/** The values of a scenario's named observations at one point. */
export type Observations = Record<string, JsonValue>;

/** The runner playing scenarios through the driver of each one's interaction. */
export const Runner = {
    /** Take a scenario's steps through a driver, reading its named observations after each step or once at the end. */
    async observe<Speaks extends Interaction, Instance>(
        scenario: Scenario<Speaks, Instance>,
        type: DriverType<Speaks, Instance>,
    ): Promise<Observations[]> {
        // start the driver, then take each step and read what it leaves
        const driver = await type.start(scenario.given);
        const isEach = "each" in scenario.then;
        const observed: Observations[] = [];
        for (const [index, step] of scenario.when.entries()) {
            await driver.act(step);
            if (isEach || index === scenario.when.length - 1) {
                observed.push(await read(driver, scenario.then.observe));
            }
        }

        return observed;
    },

    /** Play a scenario as a test through the driver of its interaction, expecting the observations it names. */
    play(scenario: Scenario, types: readonly DriverType[]): void {
        // pick the driver speaking the scenario's interaction
        const name = scenario.interaction.name;
        const type = types.find((candidate) => candidate.interaction.name === name);
        if (type === undefined) {
            throw new TypeError(`no driver plays the ${name} interaction of ${scenario.name}`);
        }

        // expect the observations it names
        const then = scenario.then;
        test(scenario.name, async () => {
            expect(await Runner.observe(scenario, type)).toEqual(
                "each" in then ? then.each : [then.end],
            );
        });
    },

    /** Play each scenario a module exports as a test. */
    playModule(exports: Readonly<Record<string, unknown>>, types: readonly DriverType[]): void {
        for (const exported of Object.values(exports)) {
            if (isScenario(exported)) {
                Runner.play(exported, types);
            }
        }
    },
};

/** Read each named observation through a driver. */
async function read<Speaks extends Interaction>(
    driver: Driver<Speaks>,
    observations: Readonly<Record<string, schema.Infer<Speaks["observation"]>>>,
): Promise<Observations> {
    const observed: Observations = {};
    for (const [name, observation] of Object.entries(observations)) {
        observed[name] = await driver.observe(observation);
    }

    return observed;
}

/** Report whether a module export is a scenario. */
function isScenario(value: unknown): value is Scenario {
    return (
        typeof value === "object" &&
        value !== null &&
        "interaction" in value &&
        "given" in value &&
        "when" in value &&
        "then" in value
    );
}
