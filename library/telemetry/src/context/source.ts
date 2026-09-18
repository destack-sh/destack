import { SourceDescription } from "../inspect/source.ts";
import type { Attributes } from "@opentelemetry/api";
import {
    ATTR_DESTACK_BUILD_MANIFEST,
    ATTR_DESTACK_CODE_DECLARATION,
    ATTR_DESTACK_CODE_MODULE,
} from "../convention/source.ts";

/** Attribute a span or log to an exact build and source declaration. */
export function sourceAttributes(source: SourceDescription): Attributes {
    const reference = SourceDescription.parse(source);

    return {
        [ATTR_DESTACK_BUILD_MANIFEST]: reference.manifest,
        [ATTR_DESTACK_CODE_MODULE]: reference.declaration.module,
        [ATTR_DESTACK_CODE_DECLARATION]: reference.declaration.name,
    };
}
