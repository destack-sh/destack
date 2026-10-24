import { defineErrorFamily, type FamilyFailure, type ReasonOf } from "@destack/error";
import type {} from "@destack/package/import-meta";

/** The shader package's failures, each reason with the call error code it answers. */
export const ShaderError = defineErrorFamily(import.meta.destack.package, {
    // bad request
    INVALID_COLOR: { code: "BAD_REQUEST" },
    INVALID_UNIFORM: { code: "BAD_REQUEST" },

    // precondition failed
    CANVAS_UNSUPPORTED: { code: "PRECONDITION_FAILED" },
    CONTEXT_LOST: { code: "PRECONDITION_FAILED" },
    IMAGE_NOT_LOADED: { code: "PRECONDITION_FAILED" },
    WEBGL_UNSUPPORTED: { code: "PRECONDITION_FAILED" },

    // unprocessable content
    COMPILE_FAILED: { code: "UNPROCESSABLE_CONTENT" },
    LINK_FAILED: { code: "UNPROCESSABLE_CONTENT" },
});

/** A stable cause of shader failures. */
export type ShaderReason = ReasonOf<typeof ShaderError>;
/** A shader failing to draw, with its reason and message. */
export type ShaderFailure = FamilyFailure<ShaderReason>;
