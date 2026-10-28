import { schema } from "@destack/schema";
import { ModuleMetadata } from "../definition/metadata.ts";
import { DeclarationName } from "../definition/package.ts";
import { PackageError } from "../error/error.ts";
import type { Declaration } from "./declaration.ts";
import { requireDescription } from "./example.ts";
import type { Interaction } from "./scenario.ts";

/** The name of a measured operation: an object's verb such as `issue.create`, or a world's. */
const OperationName = schema.string().regex(/^[a-z][a-z0-9-]*\.[a-z][a-z0-9-]*$/u);

/** A parameter value, written `key=value` in a result's name. */
const ParameterValue = schema.union([schema.string().regex(/^[^\s/=]+$/u), schema.number()]);

/** A benchmark as its module defines it: a world set up once, then each operation measured. */
export interface BenchmarkDefinition<Speaks extends Interaction = Interaction> {
    /** The declaration the benchmark measures, such as an app or the machine. */
    readonly of: unknown;
    /** The interaction of the steps, which picks the driver playing them. */
    readonly interaction: Speaks;
    /** The family, unique in its package, such as `tracker`, written `BenchmarkTracker`. */
    readonly name: string;
    /** One sentence saying what the benchmark measures. */
    readonly description: string;
    /** The world's parameters, such as its members, written `key=value` after the operation. */
    readonly parameters: Readonly<Record<string, string | number>>;
    /** The steps setting the world up, which are not measured. */
    readonly setup: readonly schema.Infer<Speaks["step"]>[];
    /** The operations by name, each a step measured once per iteration, in order. */
    readonly operations: Readonly<Record<string, schema.Infer<Speaks["step"]>>>;
    /** The iterations of each operation. */
    readonly iterations: number;
    /** The most of a counter one operation may grow, by operation and counter. */
    readonly limits: Readonly<Record<string, Readonly<Record<string, number>>>>;
}

/** Operations measured on a world set up once, after Go's benchmarks. */
export interface Benchmark<Speaks extends Interaction = Interaction>
    extends Declaration, BenchmarkDefinition<Speaks> {}

/** Declare a benchmark: setup steps, then operations measured per iteration against limits. */
export function defineBenchmark<Speaks extends Interaction>(
    definition: BenchmarkDefinition<Speaks>,
    module?: ModuleMetadata,
): Benchmark<Speaks> {
    // stamp the package supplied by the module transform and validate the name and description
    const owner = ModuleMetadata.require(module, "defineBenchmark").package;
    const { name, description, interaction, setup, operations, parameters, limits } = definition;
    DeclarationName.parse(name);
    requireDescription("benchmark", name, description);

    // require the interaction's steps, named operations and parameters
    for (const step of [...setup, ...Object.values(operations)]) {
        interaction.step.parse(step);
    }
    for (const operation of Object.keys(operations)) {
        OperationName.parse(operation);
    }
    for (const value of Object.values(parameters)) {
        ParameterValue.parse(value);
    }

    // require an operation, iterations and limits only on measured operations
    if (Object.keys(operations).length === 0) {
        throw PackageError.of("DEFINITION_INVALID", `benchmark ${name} measures no operation`);
    }
    if (!Number.isInteger(definition.iterations) || definition.iterations < 1) {
        throw PackageError.of("DEFINITION_INVALID", `benchmark ${name} takes no iterations`);
    }
    const unmeasured = Object.keys(limits).find((operation) => !(operation in operations));
    if (unmeasured !== undefined) {
        throw PackageError.of(
            "DEFINITION_INVALID",
            `benchmark ${name} limits ${unmeasured}, which it does not measure`,
        );
    }

    return Object.freeze({ package: owner, ...definition });
}
