import { defineErrorFamily, type FamilyFailure, type ReasonOf } from "@destack/error";
import type {} from "@destack/package/import-meta";

/** The shader package's failures, each reason with the call error code it answers. */
export const ShaderError = defineErrorFamily({
    reasons: {
        // invalid
        COLOR_INVALID: {},
        COMPILE_INVALID: {},
        LINK_INVALID: {},
        UNIFORM_INVALID: {},

        // blocked
        CONTEXT_LOST_BLOCKED: {},
        IMAGE_BLOCKED: {},

        // unsupported
        CANVAS_UNSUPPORTED: {},
        WEBGL_UNSUPPORTED: {},
    },
});

/** A stable cause of shader failures. */
export type ShaderReason = ReasonOf<typeof ShaderError>;
/** A shader failing to draw, with its reason and message. */
export type ShaderFailure = FamilyFailure<ShaderReason>;
