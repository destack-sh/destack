import type {
    DropGlue,
    DropHook,
    DropMetadata,
} from "../../../_generated/mir/metadata/drop.js";
import type { LocalNodeId } from "../../../_generated/mir/tree/node.js";
import { getLocalNodeValue, localNodeIdMatches } from "../node.js";

export const DropMetadataImpl = {
    /** Return full drop glue for a type. */
    dropGlue(metadata: DropMetadata, ty: LocalNodeId): DropGlue | undefined {
        return getLocalNodeValue(metadata.glueByType, ty);
    },

    /** Return the user-authored drop hook for a type. */
    dropHook(metadata: DropMetadata, ty: LocalNodeId): DropHook | undefined {
        return getLocalNodeValue(metadata.hooksByType, ty);
    },
};

export const DropGlueImpl = {
    /** Return the concrete function backing this glue. */
    functionId(glue: DropGlue): LocalNodeId | undefined {
        return glue.kind === "generated" ? glue.function : undefined;
    },

    /** Return whether this glue is generated for the given function. */
    isGeneratedFunction(glue: DropGlue, target: LocalNodeId): boolean {
        return glue.kind === "generated" && localNodeIdMatches(glue.function, target);
    },
};

export const DropHookImpl = {
    /** Return whether this hook calls the given function. */
    isFunction(hook: DropHook, target: LocalNodeId): boolean {
        return localNodeIdMatches(hook.function, target);
    },
};
