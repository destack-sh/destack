import type { ObjectServer } from "@destack/object/server";
import type { AccountCall } from "../object/index.ts";

/** Run work for an account as the system in one transaction of the server's database. */
export function asAccount<Result>(
    server: ObjectServer,
    scope: string,
    work: (call: AccountCall) => Promise<Result>,
): Promise<Result> {
    const now = server.clock();

    return server.database.transaction((database) =>
        work({ database, scope, now, invoke: server.invoker({ database, scope, now }) }),
    );
}
