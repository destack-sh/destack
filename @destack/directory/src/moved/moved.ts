import { schema } from "@destack/schema";
import { ServiceError } from "@destack/service/error";

/** A scope that moved, and the cell it answers at now. */
export interface Moved {
    /** The moved scope. */
    readonly scope: string;
    /** The cell now serving it. */
    readonly cell: string;
}

/** The answer a cell gives for a scope that moved to another cell. */
export const Moved = {
    /** The schema of a moved scope. */
    schema: schema.object({
        /** The moved scope. */
        scope: schema.string().min(1),
        /** The cell now serving it. */
        cell: schema.string().min(1),
    }),

    /** Build the 421 failure that points to the cell a scope moved to. */
    error(moved: Moved): Error {
        return new ServiceError("MOVED", {
            status: 421,
            message: `${moved.scope} moves to ${moved.cell}`,
            data: moved,
        });
    },

    /** Read the cell a MOVED failure points to, absent for other failures. */
    of(error: unknown): Moved | undefined {
        return error instanceof ServiceError && error.code === "MOVED"
            ? Moved.schema.parse(error.data)
            : undefined;
    },
};
