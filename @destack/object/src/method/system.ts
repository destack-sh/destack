import type { Select, Table } from "@destack/db";
import { schema, type JsonObject } from "@destack/schema";

/** Build system calls. */
export const SystemCall = {
    /** Call on an existing object's row, in its scope. */
    of<Row extends Select<Table>, Input extends Readonly<Record<string, unknown>>>(
        row: Row,
        input?: Input,
    ): SystemCall<Row, Input> {
        return {
            scope: schema.string().parse(row["scope"]),
            target: row,
            ...(input === undefined ? {} : { input }),
        };
    },
};

/** One call the system makes. */
export interface SystemCall<Row extends Select<Table> = Select<Table>, Input = JsonObject> {
    /** The scope the call acts in. */
    readonly scope: string;
    /** The object the call acts on, as its table selects it, absent for a creation or a call on the collection. */
    readonly target?: Row;
    /** The identifier a creation takes, a fresh one when absent. */
    readonly id?: string;
    /** The method's input, without the target's identifier. */
    readonly input?: Input;
    /** The request the call answers once, a repeat replaying its recorded result, after Stripe's idempotency keys. */
    readonly requestId?: string;
}
