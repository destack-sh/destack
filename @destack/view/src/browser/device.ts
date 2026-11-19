import type { Environment } from "@destack/environment";
import type { BrowserCapability } from "@destack/environment/browser";
import { broadcastChannel, connectWasmSqlite, WasmClient } from "@destack/db/browser";
import { BrowserDevice, type DeviceWorkerScope } from "@destack/object/browser";
import { ObjectClient, ObjectConnection } from "@destack/object/client";
import type { ObjectType } from "@destack/object";
import { schema } from "@destack/schema";
import { Subject } from "@destack/sync";
import { account, space, user } from "@destack/account/object";
import { deviceSetting, setting } from "@destack/setting/object";
import { ScopePath } from "../declare/context.ts";
import type { View } from "../declare/view.ts";

export { BrowserEnvironment } from "@destack/environment/browser";

/** Every part a scope plays for a page, whose object types the person's device stores together. */
const DEVICE_ROLES = ["user", "account", "space"] as const;

/** The parts a scope plays for a page, each opening its own object types. */
export const DeviceRole = Object.assign(schema.enum(DEVICE_ROLES), {
    /** List the parts a page's scopes may play: the person's own only on their device. */
    played(device: Subject | undefined): DeviceRole[] {
        return DEVICE_ROLES.filter((role) => role !== "user" || device !== undefined);
    },
});
/** The part a scope plays for a page. */
export type DeviceRole = schema.Infer<typeof DeviceRole>;

/** What a page's tab asks its device worker for: the person's objects of the package's views. */
export const DeviceRequest = schema.object({
    /** The device's name, one per installation and person, shared by the installation's tabs. */
    name: schema.string().min(1),
    /** The person signed in. */
    caller: Subject,
    /** The browser's device storing device-scoped local objects. */
    device: Subject.exactOptional(),
    /** The page's origin with its base, which serves the package and the homes. */
    root: schema.string().min(1),
    /** The space of the page's installation, whose store the device connects to. */
    space: schema.string().min(1),
});
/** What a page's tab asks its device worker for. */
export type DeviceRequest = schema.Infer<typeof DeviceRequest>;

/** Serve a package's device from its worker, over the person's database file. */
export function serveDevice(
    views: readonly View[],
    scope: DeviceWorkerScope,
    environment: Environment & Environment<"broadcast" | BrowserCapability>,
): void {
    BrowserDevice.listen(scope, async (message) => {
        // open the device's database file with the tables of the object types it stores
        const request = DeviceRequest.parse(message);
        const { name } = request;
        const objects = storedObjects(views, request);
        const file = await WasmClient.opfs(name, environment);
        const database = connectWasmSqlite(file, ObjectClient.tables(objects), {
            name,
            environment,
        });

        // follow and write every scope over one connection to the package's origin, connecting to enclosing homes below its home path
        const connection = ObjectConnection.open({
            scope: request.caller.id,
            url: ObjectConnection.url(`${request.root}${ScopePath.of(request.space)}`),
            homes: (home: string) =>
                ObjectConnection.open({
                    scope: home,
                    url: ObjectConnection.url(`${request.root}${ScopePath.of(home)}`),
                    environment,
                }),
            environment,
        });
        const publisher = (followed: string) => connection.at(followed);

        // serve the person's device in the person's scope
        return BrowserDevice.serve({
            database,
            channel: broadcastChannel(`destack:${name}`, environment),
            objects,
            scope: request.caller.id,
            caller: request.caller,
            ...(request.device === undefined ? {} : { device: request.device }),
            remote: connection,
            publisher,
            environment,
        });
    });
}

/** List the object types a device stores for its views, the person's own only on their device. */
export function storedObjects(
    views: readonly View[],
    request: Pick<DeviceRequest, "device">,
): Readonly<Record<string, ObjectType>> {
    const roles = DeviceRole.played(request.device);

    return Object.fromEntries(views.flatMap((view) => Object.entries(deviceObjects(view, roles))));
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
