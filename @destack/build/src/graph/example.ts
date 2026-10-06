import { graph, type Package } from "@destack/package";
import type { SymbolReference } from "@destack/package/code";
import type { SourceRange } from "@destack/package/source";
import type { ExampleDeclaration } from "../typescript/example.ts";
import type { DeclarationGraph } from "./declaration.ts";

/** A declaration of a module with the edges from it and the range of its defining call. */
export interface DescribedDeclaration {
    /** The declaration. */
    readonly declaration: graph.Declaration;
    /** The edges from the declaration. */
    readonly edges: readonly graph.Edge[];
    /** The range of the defining call. */
    readonly source: SourceRange;
}

/** The examples of each module, as declarations showing the symbols they render. */
export class ExampleGraph {
    /** The examples of each module, by module path. */
    readonly #modules = new Map<string, DescribedDeclaration[]>();
    /** The examples, by the moniker of the constant holding each. */
    readonly #constants = new Map<graph.Moniker, DescribedDeclaration>();

    /** Describe each example at its constant, showing its symbol and the declarations at it. */
    constructor(
        source: Package,
        kinds: Package,
        examples: readonly ExampleDeclaration[],
        declared: DeclarationGraph,
        name: (reference: SymbolReference) => graph.Moniker,
    ) {
        for (const example of examples) {
            // describe the example at its constant
            const symbol = graph.Moniker.of({
                packageId: source.id,
                module: example.source.file,
                name: example.symbol,
            });
            const declaration: graph.Declaration = {
                moniker: graph.Moniker.parse(`${symbol}:example`),
                symbol,
                kind: "example",
                package: kinds.id,
                name: example.name,
                description: {
                    ...example.description,
                    objects: Object.fromEntries(
                        Object.entries(example.objects).map(([scope, calls]) => [
                            scope,
                            calls.map((call) => ({ ...call, object: name(call.object) })),
                        ]),
                    ),
                },
            };

            // show the symbol and every declaration at it
            const shown = name(example.of);
            const edges = [shown, ...declared.at(shown)].map((to) => ({
                from: declaration.moniker,
                to,
                kind: "shows" as const,
            }));
            const described = {
                declaration,
                edges,
                source: example.source,
            };
            this.#constants.set(symbol, described);
            this.#modules.set(example.source.file, [
                ...(this.#modules.get(example.source.file) ?? []),
                described,
            ]);
        }
    }

    /** List a module's examples in source order. */
    describe(path: string): readonly DescribedDeclaration[] {
        return this.#modules.get(path) ?? [];
    }

    /** Find the example a constant holds, absent for a constant holding none. */
    at(constant: graph.Moniker): DescribedDeclaration | undefined {
        return this.#constants.get(constant);
    }
}
