import { defineSchema, type JsonValue, schema } from "@destack/schema";
import { ModuleMetadata } from "../definition/metadata.ts";
import { DeclarationName } from "../definition/package.ts";
import { PackageError } from "../error/error.ts";
import { Moniker } from "../graph/moniker.ts";
import type { Declaration } from "./declaration.ts";
import { type Example, requireDescription } from "./example.ts";

/** What a scenario's steps and observations speak and its examples play in. */
export interface Interaction<Step = unknown, Observation = unknown, Environment = unknown> {
    /** The name a runner picks the driver by, such as `ui`. */
    readonly name: string;
    /** The schema of a step, a JSON object naming its action. */
    readonly step: schema.Schema<Step>;
    /** The schema of an observation, a JSON object naming its kind. */
    readonly observation: schema.Schema<Observation>;
    /** The schema of the environment the given examples play in, such as a theme and locale. */
    readonly environment: schema.Schema<Environment>;
}

/** The step every interaction shares: change properties an example declares, as its owner or a control would. */
export const SetStep = defineSchema(
    schema.object({
        /** Change properties an example declares. */
        action: schema.literal("set"),
        /** The new values, by property. */
        properties: schema.record(schema.string(), schema.json()),
        /** The example by name, the scenario's only example when absent. */
        example: DeclarationName.exactOptional(),
    }),
);
/** The step every interaction shares: change properties an example declares. */
export type SetStep = schema.Infer<typeof SetStep>;

/** The schema of a step naming the set action, read as the shared set step before any interaction's. */
const SetAction = schema.looseObject({ action: schema.literal("set") });

/** The observations a scenario expects, after Gherkin's Then: named, and expected after each step or once at the end. */
export const Then = Object.assign(defineSchema(thenOf(schema.json())), {
    /** Build the schema of the observations a scenario expects, each one of an interaction's observations. */
    of: thenOf,
});
/** The observations a scenario expects, each an observation of the scenario's interaction. */
export type Then<Observation = JsonValue> = schema.Infer<
    ReturnType<typeof thenOf<schema.Schema<Observation>>>
>;

/** The examples a scenario starts from and the environment it plays in, after Gherkin's Given. */
export interface Given<Instance = unknown, Environment = unknown> {
    /** The examples a driver starts when the scenario starts. */
    readonly examples: readonly Example<object, Instance>[];
    /** The environment the examples play in, its interaction's, the host's own when absent. */
    readonly environment?: Environment;
}

/** A scenario as its module defines it: given examples, steps taken on them and the observations they leave, all as data. */
export interface ScenarioDefinition<Speaks extends Interaction = Interaction, Instance = unknown> {
    /** The declaration the scenario exercises, such as a component, or the view or app a flow runs in. */
    readonly of: unknown;
    /** The interaction the steps and observations speak, which picks the driver playing them. */
    readonly interaction: Speaks;
    /** The name of the behaviour it plays, verb first and unique among the declaration's scenarios, such as `select-with-arrow-keys`. */
    readonly name: string;
    /** One sentence saying what the scenario shows. */
    readonly description: string;
    /** The examples it starts from and the environment it plays in. */
    readonly given: Given<Instance, schema.Infer<Speaks["environment"]>>;
    /** The steps, taken in order. */
    readonly when: readonly (SetStep | schema.Infer<Speaks["step"]>)[];
    /** The observations expected after each step or once at the end. */
    readonly then: Then<schema.Infer<Speaks["observation"]>>;
}

/** Examples set in motion: steps across declarations and the observations they leave, played by a runner through a driver. */
export interface Scenario<Speaks extends Interaction = Interaction, Instance = unknown>
    extends Declaration, ScenarioDefinition<Speaks, Instance> {}

/** Declare a scenario after Gherkin's Given, When and Then, its steps and observations read through its interaction. */
export function defineScenario<Speaks extends Interaction, Instance>(
    definition: ScenarioDefinition<Speaks, Instance>,
    module?: ModuleMetadata,
): Scenario<Speaks, Instance> {
    // stamp the package supplied by the module transform and validate the name and description
    const owner = ModuleMetadata.require(module, "defineScenario").package;
    const { of, interaction, name, description, given, when } = definition;
    DeclarationName.parse(name);
    requireDescription("scenario", name, description);

    // require steps and observations of the interaction, the observations expected after each step or at the end
    requireSteps(name, when, interaction);
    const then = Then.of(interaction.observation).parse(definition.then);
    requireExpectations(name, when.length, then);
    requireExamples(name, when, given);

    // require an environment the interaction plays in
    if (given.environment !== undefined) {
        interaction.environment.parse(given.environment);
    }

    return Object.freeze({
        package: owner,
        of,
        interaction,
        name,
        description,
        given,
        when,
        then: definition.then,
    });
}

/** Require at least one step, each the set step every interaction shares or one of the interaction's own. */
function requireSteps(name: string, when: readonly unknown[], interaction: Interaction): void {
    // require a step
    if (when.length === 0) {
        throw new PackageError("INVALID_DEFINITION", `scenario ${name} takes no step`);
    }

    // parse each step as the shared set step or as one of the interaction's
    for (const step of when) {
        (isSetStep(step) ? SetStep : interaction.step).parse(step);
    }
}

/** Report whether a step names the set action. */
function isSetStep(step: unknown): step is SetStep {
    return SetAction.safeParse(step).success;
}

/** Require the expected values to name every observation, once after each step or once at the end. */
function requireExpectations(name: string, steps: number, then: Then<unknown>): void {
    // compare the names each record expects with the observations
    const records = "each" in then ? then.each : [then.end];
    const names = Object.keys(then.observe).toSorted().join(", ");
    const mismatch = records.find((record) => Object.keys(record).toSorted().join(", ") !== names);
    if (("each" in then && records.length !== steps) || mismatch !== undefined) {
        throw new PackageError(
            "INVALID_DEFINITION",
            `scenario ${name} expects ${names} after each of its ${steps} steps or once at the end`,
        );
    }
}

/** Require each set step to name one example the scenario is given, or the only one. */
function requireExamples(name: string, when: readonly unknown[], given: Given): void {
    // find a set step naming no example given, or none of several
    const examples = given.examples.map((example) => example.name);
    const unnamed = when.find(
        (step) =>
            isSetStep(step) &&
            (step.example === undefined ? examples.length !== 1 : !examples.includes(step.example)),
    );
    if (unnamed !== undefined) {
        throw new PackageError(
            "INVALID_DEFINITION",
            `scenario ${name} sets the properties of no one example it is given`,
        );
    }
}

/** Build the schema of the observations a scenario expects, each one of an interaction's observations. */
function thenOf<Observe extends schema.Schema>(observation: Observe) {
    const observe = schema.record(schema.string().min(1), observation);

    return schema.union([
        schema.object({
            /** The observations by name. */
            observe,
            /** The values expected after each step, by observation name. */
            each: schema.array(schema.record(schema.string(), schema.json())),
        }),
        schema.object({
            /** The observations by name. */
            observe,
            /** The values expected after the last step, by observation name. */
            end: schema.record(schema.string(), schema.json()),
        }),
    ]);
}

/** The description of a scenario declaration in the graph, its steps and observations stored as JSON its interaction reads. */
export const ScenarioDescription = defineSchema(
    schema.object({
        /** One sentence saying what the scenario shows. */
        description: schema.string().min(1),
        /** The interaction the steps and observations speak, by symbol moniker. */
        interaction: Moniker,
        /** The examples the scenario starts from, by declaration moniker. */
        examples: schema.array(Moniker),
        /** The environment the examples play in as JSON its interaction reads, the host's own when absent. */
        environment: schema.json().exactOptional(),
        /** The steps, taken in order. */
        when: schema.array(schema.json()).min(1),
        /** The observations expected after each step or once at the end. */
        then: Then,
    }),
);
/** The description of a scenario declaration in the graph. */
export type ScenarioDescription = schema.Infer<typeof ScenarioDescription>;
