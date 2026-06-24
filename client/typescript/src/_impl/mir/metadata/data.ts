import type { DataLayout } from "../../../_generated/mir/metadata/data.js";

export const DataLayoutImpl = {
    /** Return the pointer width in bits. */
    pointerBits(layout: DataLayout): number {
        return layout.pointerBytes * 8;
    },
};
