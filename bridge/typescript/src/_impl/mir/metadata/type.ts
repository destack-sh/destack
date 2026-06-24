import type { StringId } from "../../../_generated/core/string.js";
import type { TypeLineage, TypeMetadata } from "../../../_generated/mir/metadata/type.js";
import type { LocalNodeId } from "../../../_generated/mir/tree/node.js";
import { getLocalNodeValue } from "../node.js";

export const TypeMetadataImpl = {
    /** Return lineage metadata for a type when present. */
    lineage(metadata: TypeMetadata, ty: LocalNodeId): TypeLineage | undefined {
        return getLocalNodeValue(metadata.lineageByType, ty);
    },

    /** Return the display name for a type when present. */
    displayName(metadata: TypeMetadata, ty: LocalNodeId): StringId | undefined {
        return getLocalNodeValue(metadata.displayNameByType, ty);
    },

    /** Return the runtime type descriptor global for a type when present. */
    descriptorGlobal(metadata: TypeMetadata, ty: LocalNodeId): LocalNodeId | undefined {
        return getLocalNodeValue(metadata.descriptorByType, ty);
    },
};
