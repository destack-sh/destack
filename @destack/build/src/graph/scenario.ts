import { ScenarioDescription } from "@destack/package/declare";
import { graph, type Package } from "@destack/package";
import type { SymbolReference } from "@destack/package/code";
import { BuildError } from "../error/index.ts";
import type { ScenarioDeclaration } from "../typescript/scenario.ts";
import type { DescribedDeclaration, ExampleGraph } from "./example.ts";

/** The scenarios of each module, as declarations covering what their examples show. */
export class ScenarioGraph {
    /** The scenarios of each module, by module path. */
    readonly #modules = new Map<string, DescribedDeclaration[]>();

    /** Describe each scenario at its constant, covering what each example it is given shows. */
    constructor(
        source: Package,
        kinds: Package,
        scenarios: readonly ScenarioDeclaration[],
        examples: ExampleGraph,
        name: (reference: SymbolReference) => graph.Moniker,
    ) {
        for (const scenario of scenarios) {
            // require each example it is given to be one of the package's
            const location = `${scenario.source.file}#${scenario.symbol}`;
            const given = scenario.examples.map((reference) => {
                const example = examples.at(name(reference));
                if (example === undefined) {
                    throw new BuildError(
                        "INSPECTION_FAILED",
                        `scenario ${location} is given ${name(reference)}, which is no example of the package`,
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
            const declaration: graph.Declaration = {
                moniker: graph.Moniker.parse(`${symbol}:scenario`),
                symbol,
                kind: "scenario",
                package: kinds.id,
                name: scenario.name,
                description: ScenarioDescription.parse({
                    interaction: name(scenario.interaction),
                    examples: given.map((example) => example.declaration.moniker),
                    ...scenario.description,
                }),
            };

            // cover what it exercises and what each example shows
            const covered = new Set([
                name(scenario.of),
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
    describe(path: string): readonly DescribedDeclaration[] {
        return this.#modules.get(path) ?? [];
    }
}
