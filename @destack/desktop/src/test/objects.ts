import { principal } from "@destack/access";
import { connectBunSqlite } from "@destack/db/bun";
import type { ObjectType } from "@destack/object";
import { ObjectClient } from "@destack/object/client";
import { Identifier } from "@destack/schema";
import { Scope } from "@destack/sync";
import { onTestFinished } from "@destack/test";
import { type DESKTOP_OBJECTS, window } from "../object/index.ts";
import type { Environment } from "@destack/environment";

/** Store a native device's local objects of a package in an in-memory database. */
export async function localObjects<const Objects extends Readonly<Record<string, ObjectType>>>(
    objects: Objects,
    environment: Environment,
    id: Identifier<"device"> = Identifier.create("device", environment),
): Promise<ObjectClient<Objects>> {
    // store the objects in memory as the native device, on the test's environment
    const database = await connectBunSqlite(":memory:", [], { environment: environment });
    onTestFinished(() => database.close());
    const device = principal.device.reference(Scope.universe.id, id);
    const opened = await ObjectClient.open({
        environment,
        database,
        objects,
        scope: device.id,
        caller: device,
        device,
        endpoint: {
            url: "http://local.invalid/",
            fetch: async () => {
                throw new TypeError("local objects cannot call a server");
            },
        },
        reconnect: () => {
            throw new TypeError("local objects cannot move to a machine");
        },
    });
    onTestFinished(() => opened.close());

    return opened;
}

/** Store a native device's windows in an in-memory database, on the test's environment. */
export function desktopObjects(
    environment: Environment,
): Promise<ObjectClient<typeof DESKTOP_OBJECTS>> {
    return localObjects({ window }, environment);
}
