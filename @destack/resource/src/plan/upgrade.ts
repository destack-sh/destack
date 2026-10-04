import { defineSchema, schema, Version } from "@destack/schema";
import { graph, type Package } from "@destack/package";
import type { BuildReader } from "@destack/package/manifest";
import { Address } from "./address.ts";
import { Plan, Step, type Comparator } from "./plan.ts";
import { Vocabulary } from "./vocabulary.ts";

/** The steps from a package's previous release to this one. */
export const Upgrade = Object.assign(
    defineSchema(
        schema.object({
            /** The earlier published release. */
            from: Version,
            /** The steps in review order. */
            steps: schema.array(Step),
        }),
    ),
    { plan: planUpgrade },
);
/** The steps from a package's previous release to this one. */
export type Upgrade = schema.Infer<typeof Upgrade>;

/** What a package has published. */
export const History = Object.assign(
    defineSchema(
        schema.object({
            /** The latest published release. */
            release: Version,
            /** The latest release's declarations, without the members they derive. */
            declarations: schema.array(graph.Declaration),
            /** The terms of the package's published releases. */
            vocabulary: Vocabulary,
        }),
    ),
    {
        /** Read what a package has published from its latest release and its vocabulary. */
        async read(latest: BuildReader, vocabulary: Vocabulary): Promise<History> {
            return {
                release: latest.manifest.package.version,
                declarations: await latest.declarations(),
                vocabulary,
            };
        },
    },
);
/** What a package has published. */
export type History = schema.Infer<typeof History>;

/** Plan the upgrade from a package's latest release to a release declaring its declarations. */
function planUpgrade(
    history: History,
    release: Package,
    declarations: readonly graph.Declaration[],
    compare: (declaration: graph.Declaration) => Comparator | undefined,
): Upgrade {
    // name the releases declaring each side of a comparison
    const earlier = { package: { ...release, version: history.release } };
    const later = { package: release };

    // pair the releases' declarations by kind and name
    const remaining = new Map(history.declarations.map((entry) => [key(entry), entry]));

    // add new declarations and compare kept ones
    const plans: (() => Plan)[] = [];
    for (const declaration of declarations) {
        const previous = remaining.get(key(declaration));
        const comparison = compare(declaration);
        remaining.delete(key(declaration));

        // add a new declaration
        if (previous === undefined) {
            const step = { action: "create", target: key(declaration), risk: "safe" } as const;
            plans.push(() => ({ steps: [{ ...step, detail: `add ${declaration.kind}` }] }));
        }
        // compare a kept declaration its kind compares
        else if (comparison !== undefined) {
            plans.push(() =>
                comparison(
                    { description: previous.description, symbol: earlier },
                    { description: declaration.description, symbol: later },
                ),
            );
        }
    }

    // remove declarations without terms, then plan the terms
    for (const [target, previous] of remaining) {
        if (previous.vocabulary === undefined) {
            const detail = `remove ${previous.kind}: earlier releases keep serving it`;
            const step = {
                action: "delete",
                target,
                risk: "backward-incompatible",
                detail,
            } as const;
            plans.push(() => ({ steps: [step] }));
        }
    }
    plans.push(() => Vocabulary.plan(history.vocabulary, declarations));

    return { from: history.release, steps: [...Plan.join(plans).steps] };
}

/** Address a declaration by its kind and name. */
function key(declaration: graph.Declaration): string {
    return Address.join(declaration.kind, declaration.name);
}
