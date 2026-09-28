import { schema } from "@destack/schema";
import { ServiceError } from "@destack/service/error";

/** The holder a moved scope answers at now. */
export interface ScopeHolder {
    /** The moved scope. */
    readonly scope: string;
    /** The host or region now holding it. */
    readonly holder: string;
}

/** Name the holder a moved scope answers at. */
export const ScopeHolder = {
    /** The schema of a scope's holder. */
    schema: schema.object({
        /** The moved scope. */
        scope: schema.string().min(1),
        /** The host or region now holding it. */
        holder: schema.string().min(1),
    }),

    /** Build the 421 failure naming a moved scope's holder. */
    error(moved: ScopeHolder): Error {
        return new ServiceError("MOVED", {
            status: 421,
            message: `${moved.scope} moves to ${moved.holder}`,
            data: moved,
        });
    },

    /** Read the holder a MOVED failure names, absent for other failures. */
    of(error: unknown): ScopeHolder | undefined {
        return error instanceof ServiceError && error.code === "MOVED"
            ? ScopeHolder.schema.parse(error.data)
            : undefined;
    },
};
