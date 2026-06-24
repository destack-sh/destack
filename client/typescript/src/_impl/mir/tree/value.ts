import type { Value, ValueSlice } from "../../../_generated/mir/tree/value.js";

export const ValueSliceImpl = {
    /** Return whether the slice has no values. */
    isEmpty(slice: ValueSlice): boolean {
        return slice.count === 0;
    },

    /** Return the number of values in the slice. */
    len(slice: ValueSlice): number {
        return slice.count;
    },
};

export const ValueImpl = {
    /** Return the numeric MIR value identifier. */
    id(value: Value): number {
        return value;
    },
};
