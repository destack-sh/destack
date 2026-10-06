import { schema } from "@destack/schema";
import { isServiceError, ServiceError } from "@destack/service/error";

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
    error(moved: Moved): ServiceError<"MISDIRECTED_REQUEST", Moved> {
        return new ServiceError("MISDIRECTED_REQUEST", {
            message: `${moved.scope} moves to ${moved.cell}`,
            data: moved,
        });
    },

    /** Read the cell a misdirected failure points to, absent for other failures. */
    of(error: unknown): Moved | undefined {
        return isServiceError(error) && error.code === "MISDIRECTED_REQUEST"
            ? Moved.schema.parse(error.data)
            : undefined;
    },

    /** Read the cell a misdirected response points to, absent for other responses such as another origin's 421. */
    async read(response: Response): Promise<Moved | undefined> {
        // skip other statuses without reading their body
        if (response.status !== ServiceError.status("MISDIRECTED_REQUEST")) {
            return undefined;
        }

        // skip a body that is no JSON
        let body: unknown;
        try {
            body = await response.clone().json();
        } catch {
            return undefined;
        }

        // read the moved scope of a misdirected answer
        const answer = schema
            .looseObject({ code: schema.literal("MISDIRECTED_REQUEST"), data: Moved.schema })
            .safeParse(body);

        return answer.success ? answer.data.data : undefined;
    },
};
