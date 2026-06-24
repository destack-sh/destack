import type { FloatValue } from "../../../_generated/mir/tree/attribute.js";

export const FloatValueImpl = {
    /** Return the f64 value represented by the stored bits. */
    toFloat(value: FloatValue): number {
        const bytes = new ArrayBuffer(8);
        new DataView(bytes).setBigUint64(0, value.bits, true);

        return new DataView(bytes).getFloat64(0, true);
    },
};
