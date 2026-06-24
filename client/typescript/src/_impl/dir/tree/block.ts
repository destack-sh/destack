import type { Block } from "../../../_generated/dir/tree/block.js";
import type { LocalNodeId } from "../../../_generated/dir/tree/node.js";

export const BlockImpl = {
    /** Return whether the block has explicit block syntax. */
    isExplicit(block: Block): boolean {
        return block.form === "explicit" || block.form === "do";
    },

    /** Return whether the block has no expressions. */
    isEmpty(block: Block): boolean {
        return block.leadingExpressions.length === 0 && block.tailExpression === undefined;
    },

    /** Return the number of expressions in the block. */
    len(block: Block): number {
        return block.leadingExpressions.length + (block.tailExpression === undefined ? 0 : 1);
    },

    /** Return the first expression in evaluation order. */
    firstExpression(block: Block): LocalNodeId | undefined {
        return block.leadingExpressions[0] ?? block.tailExpression;
    },

    /** Return the last expression in evaluation order. */
    lastExpression(block: Block): LocalNodeId | undefined {
        return block.tailExpression ?? block.leadingExpressions.at(-1);
    },
};
