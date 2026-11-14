import { type JsonValue, present, schema } from "@destack/schema";
import { ModuleMetadata } from "../definition/metadata.ts";
import { DeclarationName } from "../definition/package.ts";
import { PackageError } from "../error/error.ts";
import { Moniker } from "../graph/moniker.ts";
import type { Declaration } from "./declaration.ts";
import { type Example, requireDescription } from "./example.ts";

/** What a scenario's steps and observations speak and its examples play in. */
export interface Interaction<Step = unknown, Observation = unknown, Globals = unknown> {
    /** The name a runner picks the driver by, such as `ui`. */
    readonly name: string;
    /** The schema of a step, a JSON object with its action. */
    readonly step: schema.Schema<Step>;
    /** The schema of an observation, a JSON object with its kind. */
    readonly observation: schema.Schema<Observation>;
    /** The schema of the globals the given examples play with, such as a theme and locale. */
    readonly globals: schema.Schema<Globals>;
}

/** The step every interaction shares: change example properties, as the user or a control would. */
export const SetStep = schema.object({
    /** Change properties an example declares. */
    action: schema.literal("set"),
    /** The new values, by property. */
    properties: schema.jsonObject(),
    /** The example by name, the scenario's only example when absent. */
    example: DeclarationName.exactOptional(),
});
/** The step every interaction shares: change properties an example declares. */
export type SetStep = schema.Infer<typeof SetStep>;

/** The schema of a set action step, read as the shared set step before any interaction's. */
const SetAction = schema.looseObject({ action: schema.literal("set") });

/** The name of an operation: an object's verb such as `issue.create`, or a world's. */
const OperationName = schema.string().regex(/^[a-z][a-z0-9-]*\.[a-z][a-zA-Z0-9]*$/u);

/** The signals a check selects by their name, the host emitting them and their attributes. */
export const Selector = schema.object({
    /** The span's or metric's name, such as `destack.replica.apply`, any when absent. */
    name: schema.string().min(1).exactOptional(),
    /** The host by its name, `*` matching any run of characters, such as `member-*.device-*`. */
    host: schema.string().min(1).exactOptional(),
    /** The attribute values a signal must have, such as `db.namespace`. */
    attributes: schema
        .record(schema.string(), schema.union([schema.string(), schema.number(), schema.boolean()]))
        .exactOptional(),
});
/** The signals a check selects. */
export type Selector = schema.Infer<typeof Selector>;

/** A check of the telemetry an operation's repetitions emit, after OpenTelemetry's signals. */
export const Check = schema.union([
    schema.object({
        /** The spans that ended. */
        spans: Selector,
        /** How many ended in all the repetitions. */
        count: schema.number().int().nonnegative(),
    }),
    schema.object({
        /** The spans that ended, as a cost such as the pages a device applies. */
        spans: Selector,
        /** The most that may end per repetition, as measured. */
        atMost: schema.number().nonnegative(),
    }),
    schema.object({
        /** The log records emitted, by severity text such as `ERROR`. */
        logs: Selector.omit({ name: true }).extend({
            severity: schema.string().min(1).exactOptional(),
        }),
        /** How many were emitted in all the repetitions. */
        count: schema.number().int().nonnegative(),
    }),
    schema.object({
        /** The metric: a histogram's records, or a counter's sum. */
        metric: Selector,
        /** How much it grew in all the repetitions. */
        count: schema.number().int().nonnegative(),
    }),
    schema.object({
        /** The metric, as a cost such as the statements a call runs. */
        metric: Selector,
        /** The most it may grow per repetition, as measured. */
        atMost: schema.number().nonnegative(),
    }),
    schema.object({
        /** An observation of the interaction, read once the step is taken. */
        observe: schema.json(),
        /** The value it must read. */
        is: schema.json(),
    }),
]);
/** A check of what a step does: the telemetry it emits, or an observation it leaves. */
export type Check<Observation = JsonValue> =
    | Exclude<schema.Infer<typeof Check>, { readonly observe: unknown }>
    | {
          /** An observation of the interaction, read once the step is taken. */
          readonly observe: Observation;
          /** The value it must read. */
          readonly is: JsonValue;
      };

/** A step taken with checks, repeated and measured under a name when it has one. */
export interface Operation<Step = unknown, Observation = JsonValue> {
    /** The name, an object and its verb such as `space.create`, unmeasured when absent. */
    readonly operation?: string;
    /** How often the step repeats, once when absent. */
    readonly times?: number;
    /** How often the step repeats before it is measured, warming caches up, none when absent. */
    readonly warmup?: number;
    /** The step. */
    readonly step: Step;
    /** The checks of what its repetitions do, none when absent. */
    readonly then?: readonly Check<Observation>[];
}

/** A whole-string reference to parameters or summed products of them, such as `{a * b + c}`. */
export type Reference = `{${string}}`;

/** A value whose numbers may be references to the scenario's parameters. */
export type Parametric<Value> = Value extends number
    ? number | Reference
    : Value extends readonly (infer Item)[]
      ? readonly Parametric<Item>[]
      : Value extends object
        ? { readonly [Key in keyof Value]: Parametric<Value[Key]> }
        : Value;

/** A whole-string reference: parameter names and integers joined by `*`, products joined by `+`. */
const REFERENCE = /^\{((?:[a-z][a-zA-Z0-9]*|\d+)(?: [*+] (?:[a-z][a-zA-Z0-9]*|\d+))*)\}$/u;

/** An integer within a reference, such as the roles a home starts with. */
const INTEGER = /^\d+$/u;

/** The number of a step's repetition, which the runner fills in and no parameter may be named. */
const REPETITION = "n";

/** A step's repetition within a string, such as `Issue {n}`. */
const REPETITION_TEXT = /\{n\}/gu;

/** The verbs of the references within a scenario's steps and checks. */
export const Reference = {
    /** List the parameter names a value's references name, beside the repetition. */
    names(value: unknown): string[] {
        // read a whole-string reference's names
        if (typeof value === "string") {
            const terms = REFERENCE.exec(value)?.[1]?.split(/ [*+] /u) ?? [];
            const names = terms.filter((term) => !INTEGER.test(term));

            return terms.length === 1 && terms[0] === REPETITION ? [] : names;
        }
        // read each item's and field's
        else if (typeof value === "object" && value !== null) {
            return Object.values(value).flatMap((each) => Reference.names(each));
        }
        // read none of any other value
        else {
            return [];
        }
    },

    /** Number a step's repetition: `{n}` alone becomes the number, and within a string its text. */
    repeat(value: unknown, repetition: number): unknown {
        // replace the repetition number in a string
        if (typeof value === "string") {
            return value === `{${REPETITION}}`
                ? repetition
                : value.replaceAll(REPETITION_TEXT, String(repetition));
        }
        // number each item and field
        else if (Array.isArray(value)) {
            return value.map((each) => Reference.repeat(each, repetition));
        } else if (typeof value === "object" && value !== null) {
            return Object.fromEntries(
                Object.entries(value).map(([key, each]) => [
                    key,
                    Reference.repeat(each, repetition),
                ]),
            );
        }
        // keep any other value
        else {
            return value;
        }
    },

    /** Resolve a value's references to the sums of products of the parameters they name. */
    resolve(value: unknown, parameters: Readonly<Record<string, number>>): unknown {
        // replace a reference naming parameters, leaving the repetition to the runner
        if (typeof value === "string") {
            const sum = REFERENCE.exec(value)?.[1];
            if (sum === undefined || Reference.names(value).length === 0) {
                return value;
            }
            const products = sum
                .split(" + ")
                .map((product) =>
                    product
                        .split(" * ")
                        .reduce(
                            (total, each) =>
                                total *
                                (INTEGER.test(each)
                                    ? Number(each)
                                    : present(parameters[each], `a parameter ${each}`)),
                            1,
                        ),
                );

            return products.reduce((total, each) => total + each, 0);
        }
        // resolve each item and field
        else if (Array.isArray(value)) {
            return value.map((each) => Reference.resolve(each, parameters));
        } else if (typeof value === "object" && value !== null) {
            return Object.fromEntries(
                Object.entries(value).map(([key, each]) => [
                    key,
                    Reference.resolve(each, parameters),
                ]),
            );
        }
        // keep any other value
        else {
            return value;
        }
    },
};

/** The verbs of operations within a scenario's steps. */
export const Operation = {
    /** Report whether a step of a scenario is an operation. */
    is<Step>(step: Step | Operation<Step>): step is Operation<Step> {
        return typeof step === "object" && step !== null && "step" in step;
    },

    /** Read the step a scenario plays: an operation's own, else the step itself. */
    step<Step>(step: Step | Operation<Step>): Step {
        return Operation.is(step) ? step.step : step;
    },
};

/** The examples a scenario starts from and the globals it plays with, after Gherkin's Given. */
export interface Given<Instance = unknown, Globals = unknown> {
    /** The examples a driver starts when the scenario starts. */
    readonly examples: readonly Example<object, Instance>[];
    /** The globals the examples play with, its interaction's, the host's own when absent. */
    readonly globals?: Globals;
}

/** A scenario as its module defines it: examples, steps on them and resulting observations. */
export interface ScenarioDefinition<
    Step = unknown,
    Observation = unknown,
    Globals = unknown,
    Instance = unknown,
> {
    /** The declaration the scenario exercises, such as a component, or the flow's view or app. */
    readonly of: unknown;
    /** The interaction of the steps and observations, which picks the driver playing them. */
    readonly interaction: Interaction<Step, Observation, Globals>;
    /** The behaviour it plays, verb first and unique, such as `select-with-arrow-keys`. */
    readonly name: string;
    /** One sentence saying what the scenario shows. */
    readonly description: string;
    /** The examples it starts from and the globals it plays with. */
    readonly given: Given<Instance, Globals>;
    /** The named numbers its steps and observations refer to, none when empty. */
    readonly parameters: Readonly<Record<string, number>>;
    /** The steps, taken in order, some measured. */
    readonly when: readonly Parametric<SetStep | Step | Operation<SetStep | Step, Observation>>[];
    /** The checks after the last step: its observations and the whole play's telemetry. */
    readonly then?: readonly Parametric<Check<Observation>>[];
}

/** Examples in motion: steps across declarations and their observations, run through a driver. */
export interface Scenario<
    Step = unknown,
    Observation = unknown,
    Globals = unknown,
    Instance = unknown,
>
    extends Declaration, ScenarioDefinition<Step, Observation, Globals, Instance> {}

/** Declare a scenario after Gherkin's Given, When and Then, read through its interaction. */
export function defineScenario<Step, Observation, Globals, Instance>(
    definition: ScenarioDefinition<Step, Observation, Globals, Instance>,
    module?: ModuleMetadata,
): Scenario<Step, Observation, Globals, Instance> {
    // stamp the package supplied by the module transform and validate the name and description
    const owner = ModuleMetadata.require(module, "defineScenario").package;
    const { of, interaction, name, description, given, parameters } = definition;
    DeclarationName.parse(name);
    requireDescription("scenario", name, description);

    // resolve the references to the parameters in the steps and observations
    if (REPETITION in parameters) {
        throw PackageError.of(
            "DEFINITION_INVALID",
            `scenario '${name}' is invalid: it names a parameter '${REPETITION}', the repetition each step takes`,
        );
    }
    const unknown = Reference.names([definition.when, definition.then]).find(
        (each) => !(each in parameters),
    );
    if (unknown !== undefined) {
        throw PackageError.of(
            "DEFINITION_INVALID",
            `scenario '${name}' is invalid: it refers to '${unknown}', which is none of its parameters`,
        );
    }
    const when = definition.when.map((step) => Reference.resolve(step, parameters));
    const then = (definition.then ?? []).map((check) => Reference.resolve(check, parameters));

    // require steps and observations of the interaction, and checks reading its observations
    requireSteps(name, when, interaction);
    requireChecks(then, interaction);
    requireExamples(name, when, given);

    // require globals the interaction accepts
    if (given.globals !== undefined) {
        interaction.globals.parse(given.globals);
    }

    return Object.freeze({
        package: owner,
        of,
        interaction,
        name,
        description,
        given,
        parameters,
        when: definition.when,
        ...(definition.then === undefined ? {} : { then: definition.then }),
    });
}

/** Require at least one step, each the shared set step or one of the interaction's own. */
function requireSteps(name: string, when: readonly unknown[], interaction: Interaction): void {
    // require a step
    if (when.length === 0) {
        throw PackageError.of(
            "DEFINITION_INVALID",
            `scenario '${name}' is invalid: it has no steps`,
        );
    }

    // parse each step as the shared set step, one of the interaction's, or an operation of one
    const operation = schema.object({
        operation: schema.string().exactOptional(),
        times: schema.number().exactOptional(),
        warmup: schema.number().exactOptional(),
        step: schema.union([SetStep, interaction.step]),
        then: schema.array(Check).exactOptional(),
    });
    for (const entry of when) {
        (Operation.is(entry) ? operation : isSetStep(entry) ? SetStep : interaction.step).parse(
            entry,
        );
        requireChecks(Operation.is(entry) ? (entry.then ?? []) : [], interaction);
    }

    // require each operation to have a distinct object and verb, and repeat at least once
    const operations = when.filter((entry) => Operation.is(entry));
    for (const entry of operations) {
        const label = entry.operation ?? "a checked step";
        if (entry.operation !== undefined && !OperationName.safeParse(entry.operation).success) {
            throw PackageError.of(
                "DEFINITION_INVALID",
                `scenario '${name}' is invalid: '${label}' names no object and verb`,
            );
        } else if (
            entry.times !== undefined &&
            (!Number.isInteger(entry.times) || entry.times < 1)
        ) {
            throw PackageError.of(
                "DEFINITION_INVALID",
                `scenario '${name}' is invalid: it repeats '${label}' no times`,
            );
        } else if (
            entry.warmup !== undefined &&
            (!Number.isInteger(entry.warmup) || entry.warmup < 0)
        ) {
            throw PackageError.of(
                "DEFINITION_INVALID",
                `scenario '${name}' is invalid: it warms '${label}' up a negative or partial number of times`,
            );
        }
    }
    const names = operations.flatMap((entry) =>
        entry.operation === undefined ? [] : [entry.operation],
    );
    const repeated = names.find((each, index) => names.indexOf(each) !== index);
    if (repeated !== undefined) {
        throw PackageError.of(
            "DEFINITION_INVALID",
            `scenario '${name}' is invalid: it names '${repeated}' twice`,
        );
    }
}

/** Report whether a step is the set action. */
function isSetStep(step: unknown): step is SetStep {
    return SetAction.safeParse(step).success;
}

/** Require each check that reads an observation to read one of the interaction's. */
function requireChecks(checks: readonly unknown[], interaction: Interaction): void {
    for (const check of schema.array(Check).parse(checks)) {
        if (typeof check === "object" && check !== null && "observe" in check) {
            interaction.observation.parse(check.observe);
        }
    }
}

/** Require each set step to name one example the scenario is given, or the only one. */
function requireExamples(name: string, when: readonly unknown[], given: Given): void {
    // find a set step that picks no single given example
    const examples = given.examples.map((example) => example.name);
    const unnamed = when
        .map((entry) => Operation.step(entry))
        .find(
            (step) =>
                isSetStep(step) &&
                (step.example === undefined
                    ? examples.length !== 1
                    : !examples.includes(step.example)),
        );
    if (unnamed !== undefined) {
        throw PackageError.of(
            "DEFINITION_INVALID",
            `scenario '${name}' is invalid: it sets properties but does not pick one given example`,
        );
    }
}

/** The graph description of a scenario, its steps and observations as JSON for its interaction. */
export const ScenarioDescription = schema.object({
    /** One sentence saying what the scenario shows. */
    description: schema.string().min(1),
    /** The interaction the steps and observations speak, by symbol moniker. */
    interaction: Moniker,
    /** The examples the scenario starts from, by declaration moniker. */
    examples: schema.array(Moniker),
    /** The globals of the examples as JSON for the interaction, the host's when absent. */
    globals: schema.json().exactOptional(),
    /** The named numbers the steps and observations refer to. */
    parameters: schema.record(schema.string(), schema.number()),
    /** The steps, taken in order, as authored with their references. */
    when: schema.array(schema.json()).min(1),
    /** The checks after the last step, as authored, none when absent. */
    then: schema.array(schema.json()).exactOptional(),
});
/** The description of a scenario declaration in the graph. */
export type ScenarioDescription = schema.Infer<typeof ScenarioDescription>;
