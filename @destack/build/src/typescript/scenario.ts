import { ScenarioDescription, Then } from "@destack/package/declare";
import type { SourceFile } from "typescript/unstable/ast";
import type { SymbolReference } from "@destack/package/code";
import { schema } from "@destack/schema";
import { DefinitionFields, type DefinitionSite, DefinitionDeclaration } from "./definition.ts";
import type { SymbolInspector } from "./symbol.ts";

/** A scenario a module exports, located and described without evaluating the module. */
export interface ScenarioDeclaration extends DefinitionSite {
    /** The declaration the scenario exercises. */
    readonly of: SymbolReference;
    /** The interaction the steps and observations speak. */
    readonly interaction: SymbolReference;
    /** The examples the scenario starts from. */
    readonly examples: readonly SymbolReference[];
    /** The description without interaction and examples: globals, steps and observations. */
    readonly description: Omit<ScenarioDescription, "interaction" | "examples">;
}

/** A scenario a module exports, located at its definition. */
export const ScenarioDeclaration = {
    /** Collect a module's exported scenarios through the compiler. */
    async collect(
        source: SourceFile,
        file: string,
        inspector: SymbolInspector,
        owner: string,
    ): Promise<ScenarioDeclaration[]> {
        const definitions = await DefinitionDeclaration.collect(
            source,
            file,
            inspector,
            "defineScenario",
            "scenario",
            owner,
        );

        return await Promise.all(
            definitions.map(async ({ site, fields }) => {
                // read the scenario's subject, interaction, examples, globals, steps and observations
                const of = await fields.symbol("of", inspector);
                const interaction = await fields.symbol("interaction", inspector);
                const given = new DefinitionFields(
                    fields.object("given"),
                    "scenario",
                    fields.location,
                );
                const examples = await given.symbols("examples", inspector);
                const globals = given.literal("globals");

                return {
                    ...site,
                    of: await inspector.reference(of),
                    interaction: await inspector.reference(interaction),
                    examples: await Promise.all(
                        examples.map((example) => inspector.reference(example)),
                    ),
                    description: {
                        description: fields.text("description"),
                        ...(globals === undefined ? {} : { globals }),
                        when: schema.array(schema.json()).min(1).parse(fields.literal("when")),
                        then: Then.parse(fields.literal("then")),
                    },
                };
            }),
        );
    },
};
