import { ScenarioDescription } from "@destack/package/declare";
import { graph, type PackageVersion } from "@destack/package";
import { schema } from "@destack/schema";
import { BuildError } from "../error/index.ts";
import type { Definition, DefinitionSite } from "../outline/definition.ts";
import type { Registration, ExampleGraph } from "./example.ts";

/** A scenario a module exports, located and described without evaluating the module. */
export interface ScenarioDeclaration extends DefinitionSite {
    /** The declaration the scenario exercises. */
    readonly of: graph.Moniker;
    /** The interaction the steps and observations speak. */
    readonly interaction: graph.Moniker;
    /** The examples the scenario starts from. */
    readonly examples: readonly graph.Moniker[];
    /** The description without interaction and examples: sentence, globals, steps, observations. */
    readonly description: Omit<ScenarioDescription, "interaction" | "examples">;
}

/** A scenario as its module's outline reads it, before evaluation gives the rest. */
export type ScenarioOutline = Omit<ScenarioDeclaration, "description"> & {
    /** The description without interaction, examples and sentence. */
    readonly description: Omit<ScenarioDescription, "interaction" | "examples" | "description">;
};

/** A scenario a module exports, located at its definition. */
export const ScenarioDeclaration = {
    /** Read a scenario's definition: its subject, interaction, examples, steps and observations. */
    async read(definition: Definition): Promise<ScenarioOutline> {
        // read the given examples and globals, and the steps and observations
        const { site, fields } = definition;
        const given = fields.object("given");
        const globals = given.literal("globals");
        const then = fields.literal("then");

        return {
            ...site,
            of: await fields.symbol("of"),
            interaction: await fields.symbol("interaction"),
            examples: await given.symbols("examples"),
            description: {
                ...(globals === undefined ? {} : { globals }),
                parameters: schema
                    .record(schema.string(), schema.number())
                    .parse(fields.literal("parameters")),
                when: schema.array(schema.json()).min(1).parse(fields.literal("when")),
                ...(then === undefined ? {} : { then: schema.array(schema.json()).parse(then) }),
            },
        };
    },
};

/** The scenarios of each module, as declarations covering what their examples show. */
export class ScenarioGraph {
    /** The scenarios of each module, by module path. */
    readonly #modules = new Map<string, Registration[]>();

    /** Describe each scenario at its constant, covering what each example it is given shows. */
    constructor(
        source: PackageVersion,
        kinds: PackageVersion,
        scenarios: readonly ScenarioDeclaration[],
        examples: ExampleGraph,
    ) {
        for (const scenario of scenarios) {
            // require each example it is given to be one of the package's
            const location = `${scenario.source.file}#${scenario.symbol}`;
            const given = scenario.examples.map((reference) => {
                const example = examples.at(reference);
                if (example === undefined) {
                    throw BuildError.of(
                        "INSPECTION_INVALID",
                        `example '${reference}' is invalid: scenario '${location}' is given it, and it is not an example of the package`,
                    );
                }

                return example;
            });

            // describe the scenario at its constant with its interaction, examples, steps and observations
            const symbol = graph.Moniker.of({
                packageId: source.id,
                module: scenario.source.file,
                name: scenario.symbol,
            });
            const declaration = graph.Declaration.at(symbol, {
                kind: "scenario",
                kindPackageId: kinds.id,
                name: scenario.name,
                description: ScenarioDescription.parse({
                    interaction: scenario.interaction,
                    examples: given.map((example) => example.declaration.moniker),
                    ...scenario.description,
                }),
            });

            // cover what it exercises and what each example shows
            const covered = new Set([
                scenario.of,
                ...given.flatMap((example) => example.edges.map((edge) => edge.to)),
            ]);
            const edges = [...covered].map((to) => ({
                from: declaration.moniker,
                to,
                kind: "covers" as const,
            }));
            this.#modules.set(scenario.source.file, [
                ...(this.#modules.get(scenario.source.file) ?? []),
                {
                    declaration,
                    edges,
                    source: scenario.source,
                },
            ]);
        }
    }

    /** List a module's scenarios in source order. */
    describe(path: string): readonly Registration[] {
        return this.#modules.get(path) ?? [];
    }
}
