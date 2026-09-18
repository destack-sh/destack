import { defineSchema, schema } from "@destack/schema";
import { Digest } from "@destack/package/manifest";
import { DeclarationReference } from "@destack/package/inspect";
import {
    ATTR_DESTACK_BUILD_MANIFEST,
    ATTR_DESTACK_CODE_DECLARATION,
    ATTR_DESTACK_CODE_MODULE,
} from "../convention/source.ts";

/** The package build and declaration that emitted a telemetry signal. */
export const SourceDescription = defineSchema(schema.object({
    /** The digest of the emitting build's manifest. */
    manifest: Digest,
    /** The named declaration in that build's inspection. */
    declaration: DeclarationReference,
}));
/** Build-qualified source attribution for a telemetry signal. */
export type SourceDescription = schema.Infer<typeof SourceDescription>;

/** Read source attribution, rejecting incomplete or invalid references. */
export function describeSource(
    attributes: Readonly<Record<string, unknown>>,
): SourceDescription | undefined {
    const manifest = attributes[ATTR_DESTACK_BUILD_MANIFEST];
    const module = attributes[ATTR_DESTACK_CODE_MODULE];
    const name = attributes[ATTR_DESTACK_CODE_DECLARATION];
    if (manifest === undefined && module === undefined && name === undefined) return undefined;

    return SourceDescription.parse({ manifest, declaration: { module, name } });
}
