import type { Environment } from "@destack/environment";
import type { BrowserCapability } from "@destack/environment/browser";
import { broadcastChannel, connectWasmSqlite, WasmClient } from "@destack/db/browser";
import { BrowserDevice, type DeviceWorkerScope } from "@destack/object/browser";
import { ObjectClient, ProcedureRemote } from "@destack/object/client";
import { PACKAGE_PATH, type ObjectType } from "@destack/object";
import { schema } from "@destack/schema";
import { Subject } from "@destack/sync";
import { account, space, user } from "@destack/account/object";
import { deviceSetting, setting } from "@destack/setting/object";
import { HOME_PATH } from "../declare/context.ts";
import type { View } from "../declare/view.ts";

/** Every part a scope plays for a page, whose object types the person's device stores together. */
const DEVICE_ROLES = ["user", "account", "space"] as const;

/** The parts a scope plays for a page, each opening its own object types. */
const DeviceRole = schema.enum(["user", "account", "space"]);
/** The part a scope plays for a page. */
export type DeviceRole = schema.Infer<typeof DeviceRole>;

/** What a page's tab asks its device worker for: the person's objects of the view. */
export const DeviceRequest = schema.object({
    /** The person signed in, whose one device every tab of the origin shares. */
    caller: Subject,
    /** The browser's device storing device-scoped local objects. */
    device: Subject.exactOptional(),
    /** The page's origin with its base, which serves the package and the homes. */
    root: schema.string().min(1),
});
/** What a page's tab asks its device worker for. */
export type DeviceRequest = schema.Infer<typeof DeviceRequest>;

/** Serve a view's device from its worker, over the person's database file. */
export function serveDevice(
    view: View,
    scope: DeviceWorkerScope,
    environment: Environment & Environment<"broadcast" | BrowserCapability>,
): void {
    BrowserDevice.listen(scope, async (message) => {
        // open the person's database file with the tables of every object type the view opens
        const request = DeviceRequest.parse(message);
        const name = request.caller.id;
        const objects = deviceObjects(view, DEVICE_ROLES);
        const file = await WasmClient.opfs(name, environment);
        const database = connectWasmSqlite(file, ObjectClient.tables(objects), {
            name,
            environment,
        });

        // follow and write each scope through the package's origin, once per scope, calling enclosing homes below its home path
        const url = `${request.root}${PACKAGE_PATH}`;
        const remotes = new Map<string, ProcedureRemote>();
        const publisher = (followed: string) => {
            const remote =
                remotes.get(followed) ??
                ProcedureRemote.at({
                    scope: followed,
                    endpoint: { url },
                    reconnect: () => ({ url }),
                    homes: (home: string) => ({ url: `${request.root}${HOME_PATH}/${home}` }),
                    environment,
                });
            remotes.set(followed, remote);

            return remote;
        };

        // serve the person's device in the person's scope
        return BrowserDevice.serve({
            database,
            channel: broadcastChannel(`destack:${name}`, environment),
            objects,
            scope: name,
            caller: request.caller,
            ...(request.device === undefined ? {} : { device: request.device }),
            remote: publisher(name),
            publisher,
            environment,
        });
    });
}

/** List the object types a scope's roles open for a view, keyed by type. */
export function deviceObjects(
    view: View,
    roles: readonly DeviceRole[],
): Readonly<Record<string, ObjectType>> {
    return Object.fromEntries(
        roles.flatMap((role) =>
            Object.entries(
                role === "user"
                    ? ObjectClient.keyed([setting, deviceSetting, user])
                    : ObjectClient.keyed(view.opens(role === "account" ? account : space)),
            ),
        ),
    );
}
