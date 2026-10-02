import { schema } from "@destack/schema";
import { ServiceError } from "@destack/service/error";

/** The HTTP status of a scope that moved: 421 Misdirected Request. */
const MOVED_STATUS = 421;

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

    /** Build the failure that points to the cell a scope moved to. */
    error(moved: Moved): ServiceError<"MOVED", Moved> {
        return new ServiceError("MOVED", {
            status: MOVED_STATUS,
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

    /** Read the cell a MOVED response points to, absent for other responses. */
    async read(response: Response): Promise<Moved | undefined> {
        // skip other statuses without reading their body
        if (response.status !== MOVED_STATUS) {
            return undefined;
        }

        // read the moved scope of a MOVED answer
        const body = schema
            .object({
                defined: schema.boolean(),
                code: schema.string(),
                status: schema.number().int(),
                message: schema.string(),
                data: schema.json().exactOptional(),
            })
            .parse(await response.clone().json());

        return body.code === "MOVED" ? Moved.schema.parse(body.data) : undefined;
    },
};
