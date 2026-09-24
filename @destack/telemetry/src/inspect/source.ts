import { defineSchema, schema } from "@destack/schema";
import { Digest } from "@destack/package/file";
import { SymbolLocation } from "@destack/package/code";
import {
    ATTR_DESTACK_BUILD_MANIFEST,
    ATTR_DESTACK_CODE_MODULE,
    ATTR_DESTACK_CODE_SYMBOL,
} from "../convention/source.ts";

/** The package build and symbol that emitted a telemetry signal. */
export const SourceDescription = defineSchema(
    schema.object({
        /** The digest of the emitting build's manifest. */
        manifest: Digest,
        /** The named symbol in that build's inspection. */
        symbol: SymbolLocation,
    }),
);
/** Build-qualified source attribution for a telemetry signal. */
export type SourceDescription = schema.Infer<typeof SourceDescription>;

/** Read source attribution, rejecting incomplete or invalid references. */
export function describeSource(
    attributes: Readonly<Record<string, unknown>>,
): SourceDescription | undefined {
    // read the attribution attributes, absent when none is set
    const manifest = attributes[ATTR_DESTACK_BUILD_MANIFEST];
    const module = attributes[ATTR_DESTACK_CODE_MODULE];
    const name = attributes[ATTR_DESTACK_CODE_SYMBOL];
    if (manifest === undefined && module === undefined && name === undefined) {
        return undefined;
    }

    return SourceDescription.parse({ manifest, symbol: { module, name } });
}
