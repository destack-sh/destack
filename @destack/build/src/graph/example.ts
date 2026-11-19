import { graph, type PackageVersion } from "@destack/package";
import { SourceRange } from "@destack/package/source";
import { schema } from "@destack/schema";
import type { Definition, DefinitionSite } from "../outline/definition.ts";
import type { DeclarationGraph } from "./declaration.ts";

/** An example a module exports, located and described, with the properties its module gives. */
const exampleSchema = schema.object({
    /** The literal name. */
    name: schema.string(),
    /** The exported constant with the example. */
    symbol: schema.string(),
    /** The defining call's range in its module. */
    source: SourceRange,
    /** The symbol the example shows. */
    of: graph.Moniker,
    /** The calls bringing the shown objects into the example's state, by scope. */
    objects: schema.record(
        schema.string(),
        schema.array(
            schema.object({
                object: graph.Moniker,
                method: schema.string(),
                input: schema.jsonObject(),
            }),
        ),
    ),
    /** The sentence saying what the example shows. */
    description: schema.string(),
    /** The properties the example renders with, as its module evaluates them. */
    properties: schema.jsonObject(),
});
/** An example a module exports, located and described, with the properties its module gives. */
export type ExampleDeclaration = schema.Infer<typeof exampleSchema> & DefinitionSite;

/** An example a module exports, located at its definition. */
export const ExampleDeclaration = Object.assign(exampleSchema, {
    /** Read an example's definition: what it shows and the calls bringing its objects. */
    async read(definition: Definition): Promise<Omit<ExampleDeclaration, "properties">> {
        // read the calls of each scope, each object by its moniker
        const { site, fields } = definition;
        const objects: Record<string, ExampleDeclaration["objects"][string]> = {};
        for (const [scope, calls] of fields.lists("objects")) {
            objects[scope] = await Promise.all(
                calls.map(async (call) => ({
                    object: await call.symbol("object"),
                    method: call.text("method"),
                    input: schema.jsonObject().parse(call.literal("input") ?? {}),
                })),
            );
        }

        return {
            ...site,
            of: await fields.symbol("of"),
            objects,
            description: fields.text("description"),
        };
    },
});

/** A declaration a module registers, with the edges from it and its registering call's range. */
export interface Registration {
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
    readonly #modules = new Map<string, Registration[]>();
    /** The examples, by the moniker of the constant with each. */
    readonly #constants = new Map<graph.Moniker, Registration>();

    /** Describe each example at its constant, showing its symbol and the declarations at it. */
    constructor(
        source: PackageVersion,
        kinds: PackageVersion,
        examples: readonly ExampleDeclaration[],
        declared: DeclarationGraph,
    ) {
        for (const example of examples) {
            // describe the example at its constant
            const symbol = graph.Moniker.of({
                packageId: source.id,
                module: example.source.file,
                name: example.symbol,
            });
            const declaration = graph.Declaration.at(symbol, {
                kind: "example",
                kindPackageId: kinds.id,
                name: example.name,
                description: {
                    description: example.description,
                    properties: example.properties,
                    objects: example.objects,
                },
            });

            // show the symbol and every declaration at it
            const shown = example.of;
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
    describe(path: string): readonly Registration[] {
        return this.#modules.get(path) ?? [];
    }

    /** Find the example of a constant, absent for a constant with none. */
    at(constant: graph.Moniker): Registration | undefined {
        return this.#constants.get(constant);
    }
}
