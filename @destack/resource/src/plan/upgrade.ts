import { defineSchema, schema, Version } from "@destack/schema";
import type { DeclarationDescription } from "@destack/package/inspect";
import type { BuildReader } from "@destack/package/manifest";
import { Address } from "./address.ts";
import { Plan, Step, type Compare } from "./plan.ts";
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
export const History = {
    /** Read what a package has published from its latest release and its vocabulary. */
    async read(latest: BuildReader, vocabulary: Vocabulary): Promise<History> {
        return {
            release: latest.manifest.package.version,
            declarations: await latest.declarations(),
            vocabulary,
        };
    },
};

/** What a package has published. */
export interface History {
    /** The latest published release. */
    readonly release: Version;
    /** The latest release's own declarations. */
    readonly declarations: readonly DeclarationDescription[];
    /** The terms of the package's published releases. */
    readonly vocabulary: Vocabulary;
}

/** Plan the upgrade from a package's latest release. */
function planUpgrade(
    history: History,
    declarations: readonly DeclarationDescription[],
    compare: (declaration: DeclarationDescription) => Compare | undefined,
): Upgrade {
    // pair the releases' declarations by kind and name
    const remaining = new Map(history.declarations.map((entry) => [key(entry), entry]));

    // add new declarations and compare kept ones
    const parts: (() => Plan)[] = [];
    for (const declaration of declarations) {
        const previous = remaining.get(key(declaration));
        const comparison = compare(declaration);
        remaining.delete(key(declaration));

        // add a new declaration
        if (previous === undefined) {
            const step = { action: "create", target: key(declaration), risk: "safe" } as const;
            parts.push(() => ({ steps: [{ ...step, detail: `add ${declaration.kind}` }] }));
        }
        // compare a kept declaration its kind compares
        else if (comparison !== undefined) {
            parts.push(() => comparison(previous, declaration));
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
            parts.push(() => ({ steps: [step] }));
        }
    }
    parts.push(() => Vocabulary.plan(history.vocabulary, declarations));

    return { from: history.release, steps: [...Plan.join(parts).steps] };
}

/** Address a declaration by its kind and name. */
function key(declaration: DeclarationDescription): string {
    return Address.join(declaration.kind, declaration.name);
}
