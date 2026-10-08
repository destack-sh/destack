import { schema } from "@destack/schema";
import { isServiceError, ServiceError } from "@destack/service/error";

/** A scope that moved, and the machine it answers at now. */
export interface Moved {
    /** The moved scope. */
    readonly scope: string;
    /** The machine now serving it. */
    readonly machine: string;
}

/** The answer a machine gives for a scope that moved to another machine. */
export const Moved = {
    /** The schema of a moved scope. */
    schema: schema.object({
        /** The moved scope. */
        scope: schema.string().min(1),
        /** The machine now serving it. */
        machine: schema.string().min(1),
    }),

    /** Build the failure that points to the machine a scope moved to. */
    error(moved: Moved): ServiceError<"MISDIRECTED_REQUEST", Moved> {
        return new ServiceError("MISDIRECTED_REQUEST", {
            message: `${moved.scope} moves to ${moved.machine}`,
            data: moved,
        });
    },

    /** Read the machine a misdirected failure points to, absent for other failures. */
    of(error: unknown): Moved | undefined {
        return isServiceError(error) && error.code === "MISDIRECTED_REQUEST"
            ? Moved.schema.parse(error.data)
            : undefined;
    },

    /** Read the machine a misdirected response points to, absent for other responses such as another origin's 421. */
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
