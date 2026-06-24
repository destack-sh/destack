import type {
    CallMetadata,
    CallSite,
    FunctionMetadata,
    FunctionMetadataTable,
} from "../../../_generated/mir/metadata/function.js";
import type { LocalNodeId } from "../../../_generated/mir/tree/node.js";
import { getLocalNodeValue, localNodeIdMatches } from "../node.js";

export const FunctionMetadataTableImpl = {
    /** Return metadata for one function. */
    function(table: FunctionMetadataTable, function_: LocalNodeId): FunctionMetadata | undefined {
        return getLocalNodeValue(table.functions, function_);
    },

    /** Return metadata for one callsite. */
    call(table: FunctionMetadataTable, callsite: CallSite): CallMetadata | undefined {
        for (const [candidate, metadata] of table.calls) {
            // compare instruction callsites structurally
            if (candidate.kind === "instruction" && callsite.kind === "instruction") {
                if (localNodeIdMatches(candidate.instruction, callsite.instruction)) {
                    return metadata;
                }
            }

            // compare terminator callsites structurally
            else if (candidate.kind === "terminator" && callsite.kind === "terminator") {
                if (localNodeIdMatches(candidate.terminator, callsite.terminator)) {
                    return metadata;
                }
            }
        }

        return undefined;
    },
};
