import type { SpaceSet } from "../../../_generated/mir/tree/memory.js";
import type { Space } from "../../../_generated/mir/tree/type.js";

/** The raw bit for each memory space. */
const SPACE_BITS: Record<Space, number> = {
    local: 1 << 0,
    shared: 1 << 1,
    frame: 1 << 2,
    static: 1 << 3,
};

export const SpaceSetImpl = {
    /** Return whether no space bits are set. */
    isEmpty(spaces: SpaceSet): boolean {
        return spaces === 0;
    },

    /** Return whether all space bits from another set are present. */
    contains(spaces: SpaceSet, space: SpaceSet | Space): boolean {
        const value = spaceValue(space);

        return (spaces & value) === value;
    },

    /** Return whether two space sets overlap. */
    intersects(left: SpaceSet, right: SpaceSet): boolean {
        return (left & right) !== 0;
    },

    /** Return whether two space sets do not overlap. */
    isDisjoint(left: SpaceSet, right: SpaceSet): boolean {
        return (left & right) === 0;
    },

    /** Return the shared space bits between two space sets. */
    intersection(left: SpaceSet, right: SpaceSet): SpaceSet {
        return left & right;
    },
};

/** Return the raw bits for one space value. */
function spaceValue(space: SpaceSet | Space): number {
    // space names map through the bit table
    if (typeof space === "string") {
        return SPACE_BITS[space];
    }

    // space sets already store raw bits
    else {
        return space;
    }
}
