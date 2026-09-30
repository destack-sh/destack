import { defineSchema, schema, Version } from "@destack/schema";
import { Digest } from "@destack/package/file";
import type { DeclarationDescription } from "@destack/package/inspect";
import { PlanError } from "../error/error.ts";
import { Address } from "./address.ts";
import type { Plan, Step } from "./plan.ts";

/** The releases that added and removed a term, and the digest of its last definition. */
export const VocabularyEntry = defineSchema(
    schema.object({
        /** The release that added the term. */
        introduced: Version,
        /** The release that removed the term, absent while declared. */
        removed: Version.optional(),
        /** The digest of the term's last declared definition. */
        digest: Digest,
    }),
);
/** The releases that added and removed a term, and the digest of its last definition. */
export type VocabularyEntry = schema.Infer<typeof VocabularyEntry>;

/** The terms of a package's releases, such as `object/note/relation/editor`. */
export const Vocabulary = Object.assign(defineSchema(schema.record(Address, VocabularyEntry)), {
    plan,
    advance,
});
/** The terms of a package's releases. */
export type Vocabulary = schema.Infer<typeof Vocabulary>;

/** Plan a release's terms, refusing a removed term under another definition. */
function plan(
    vocabulary: Vocabulary,
    declarations: readonly Pick<DeclarationDescription, "vocabulary">[],
): Plan {
    // compare each declared term with its entry
    const declared = terms(declarations);
    const steps: Step[] = [];
    const problems: PlanError["problems"][number][] = [];
    for (const [term, digest] of declared) {
        const entry = vocabulary[term];
        // restore a removed term under its last definition
        if (entry?.removed !== undefined && entry.digest === digest) {
            const detail = `restore after its removal in ${entry.removed}: data stored under it applies again`;
            steps.push({ action: "restore", target: term, risk: "data-dependent", detail });
        }
        // refuse a removed term under another definition
        else if (entry?.removed !== undefined) {
            const detail = `removed in ${entry.removed} with another definition; choose a new name`;
            problems.push({ target: term, detail });
        }
    }
    if (problems.length > 0) {
        throw new PlanError(problems);
    }

    // remove the terms the release no longer declares
    for (const [term, entry] of Object.entries(vocabulary)) {
        if (entry.removed === undefined && !declared.has(term)) {
            const detail = "remove: data stored under it no longer applies";
            steps.push({ action: "delete", target: term, risk: "backward-incompatible", detail });
        }
    }

    return { steps };
}

/** Advance a vocabulary to a release's terms. */
function advance(
    vocabulary: Vocabulary,
    declarations: readonly Pick<DeclarationDescription, "vocabulary">[],
    release: Version,
): Vocabulary {
    // keep each declared term under its current definition, restoring removed ones
    const declared = terms(declarations);
    const advanced: Vocabulary = {};
    for (const [term, digest] of declared) {
        advanced[term] = { introduced: vocabulary[term]?.introduced ?? release, digest };
    }

    // keep removed terms, marking those the release drops
    for (const [term, entry] of Object.entries(vocabulary)) {
        if (!declared.has(term)) {
            advanced[term] = { ...entry, removed: entry.removed ?? release };
        }
    }

    return advanced;
}

/** Collect the terms of a release's declarations with their definitions' digests. */
function terms(
    declarations: readonly Pick<DeclarationDescription, "vocabulary">[],
): Map<string, Digest> {
    return new Map(
        declarations.flatMap((declaration) => Object.entries(declaration.vocabulary ?? {})),
    );
}
