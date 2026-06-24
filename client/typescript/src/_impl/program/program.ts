import type { Program } from "../../_generated/program/program.js";

export const ProgramImpl = {
    /** Return the pointer width in bytes. */
    pointerBytes(program: Program): number {
        return program.header.pointerBytes;
    },

    /** Return whether the program contains native execution material. */
    hasNative(program: Program): boolean {
        return program.native !== undefined;
    },
};
