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
    error(moved: Moved): ServiceError<"MOVED", Moved> {
        return new ServiceError("MOVED", {
            message: `${moved.scope} moves to ${moved.cell}`,
            data: moved,
        });
    },

    /** Read the cell a MOVED failure points to, absent for other failures. */
    of(error: unknown): Moved | undefined {
        return isServiceError(error) && error.code === "MOVED"
            ? Moved.schema.parse(error.data)
            : undefined;
    },

    /** Read the cell a MOVED response points to, absent for other responses such as another origin's 421. */
    async read(response: Response): Promise<Moved | undefined> {
        // skip other statuses without reading their body
        if (response.status !== ServiceError.status("MOVED")) {
            return undefined;
        }

        // skip a body that is no JSON
        let body: unknown;
        try {
            body = await response.clone().json();
        } catch {
            return undefined;
        }

        // read the moved scope of a MOVED answer, skipping an answer of another shape
        const answer = schema
            .looseObject({ code: schema.literal("MOVED"), data: Moved.schema })
            .safeParse(body);

        return answer.success ? answer.data.data : undefined;
    },
};
