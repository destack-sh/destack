import type {
    FunctionBehavior,
    MemoryEffect,
} from "../../../_generated/mir/metadata/effect.js";

export const MemoryEffectImpl = {
    /** Return whether this effect does not access memory. */
    isNone(effect: MemoryEffect): boolean {
        return !effect.reads && !effect.writes;
    },

    /** Return whether this effect only reads memory. */
    isReadOnly(effect: MemoryEffect): boolean {
        return effect.reads && !effect.writes;
    },

    /** Return whether this effect only writes memory. */
    isWriteOnly(effect: MemoryEffect): boolean {
        return !effect.reads && effect.writes;
    },

    /** Return whether this effect may both read and write memory. */
    isReadWrite(effect: MemoryEffect): boolean {
        return effect.reads && effect.writes;
    },
};

export const FunctionBehaviorImpl = {
    /** Return whether the behavior may unwind execution. */
    mayUnwind(behavior: FunctionBehavior): boolean {
        return behavior.panic === "mayPanic" || behavior.suspend === "maySuspend";
    },
};
