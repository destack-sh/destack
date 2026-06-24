import type {
    MemoryAccessMetadata,
    MemoryMetadata,
} from "../../../_generated/mir/metadata/memory.js";
import type { LocalNodeId } from "../../../_generated/mir/tree/node.js";
import { getLocalNodeValue } from "../node.js";

export const MemoryMetadataImpl = {
    /** Return memory accesses for an instruction when available. */
    memoryAccesses(
        metadata: MemoryMetadata,
        instruction: LocalNodeId,
    ): ReadonlyArray<MemoryAccessMetadata> {
        return getLocalNodeValue(metadata.memoryAccessesByInstructionId, instruction) ?? [];
    },
};
