import type {
    DispatchMetadata,
    DispatchSlot,
    DynamicShape,
    DynamicTable,
    Vtable,
} from "../../../_generated/mir/metadata/dispatch.js";
import type { LocalNodeId } from "../../../_generated/mir/tree/node.js";
import { getLocalNodeValue, localNodeIdMatches } from "../node.js";

export const DispatchSlotImpl = {
    /** Return the zero-based slot index. */
    index(slot: DispatchSlot): number {
        return slot;
    },
};

export const DispatchMetadataImpl = {
    /** Return the vtable for a type when present. */
    vtable(metadata: DispatchMetadata, ty: LocalNodeId): Vtable | undefined {
        return metadata.vtables.find((table) => localNodeIdMatches(table.ty, ty));
    },

    /** Return the dynamic table for a concrete type and constraint. */
    dynamicTable(
        metadata: DispatchMetadata,
        concrete: LocalNodeId,
        constraint: LocalNodeId,
    ): DynamicTable | undefined {
        return metadata.dynamicTables.find((table) => {
            const isConcrete = localNodeIdMatches(table.concrete, concrete);
            const isConstraint = localNodeIdMatches(table.constraint, constraint);

            return isConcrete && isConstraint;
        });
    },

    /** Return dynamic shape metadata for a constraint type id. */
    dynamicShape(metadata: DispatchMetadata, constraint: LocalNodeId): DynamicShape | undefined {
        return getLocalNodeValue(metadata.dynamicShapes, constraint);
    },
};
