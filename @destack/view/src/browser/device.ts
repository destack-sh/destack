import type { Environment } from "@destack/environment";
import type { BrowserCapability } from "@destack/environment/browser";
import { broadcastChannel, connectWasmSqlite, WasmClient } from "@destack/db/browser";
import { BrowserDevice, type DeviceWorkerScope } from "@destack/object/device";
import { ObjectClient, ObjectConnection } from "@destack/object/client";
import type { ObjectType } from "@destack/object";
import { OriginPath, type PackageVersion } from "@destack/package";
import { TelemetryScope } from "@destack/telemetry";
import {
    type CaptureScope,
    reportToDevtools,
    startWorkerTelemetry,
} from "@destack/telemetry/browser";
import { OtlpExporter } from "@destack/telemetry/otlp";
import type {} from "@destack/package/import-meta";
import { schema } from "@destack/schema";
import { Subject } from "@destack/sync";
import { account, space, user } from "@destack/account/object";
import { deviceSetting, setting } from "@destack/setting/object";
import { ScopePath } from "../declare/context.ts";

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
    objects: readonly ObjectType[],
    scope: DeviceWorkerScope & CaptureScope,
    message: unknown,
    source: PackageVersion,
    loading: { readonly start: number; readonly end: number },
    environment: Environment & Environment<"broadcast" | BrowserCapability>,
): void {
    BrowserDevice.answer(
        scope,
        async (started) => {
            // export the device's telemetry and uncaught failures to the package's origin, as its pages do
            const request = DeviceRequest.parse(started);
            const exporter = OtlpExporter.http(
                {
                    endpoint: `${request.root}${OriginPath.telemetry}`,
                    isKeepalive: false,
                    fetch: (fetched) => environment.network.fetch(fetched),
                },
                reportToDevtools,
            );
            const telemetry = await startWorkerTelemetry(
                exporter.options(source),
                environment,
                scope,
            );
            const traced = { ...environment, telemetry };
            const instruments = TelemetryScope.of(import.meta.destack.package, telemetry);

            // record the worker loading its modules, timed before its telemetry started
            instruments.span.record("destack.device.load", {}, loading);

            // open the device in a span, each of its steps in its own
            return await instruments.span("destack.device.open", {}, async () => {
                // open the device's database file with the tables of the object types it stores
                const { name } = request;
                const stored = storedObjects(objects, request);
                const database = await instruments.span("destack.device.database", {}, async () =>
                    connectWasmSqlite(
                        await WasmClient.opfs(name, traced),
                        ObjectClient.tables(stored),
                        {
                            name,
                            environment: traced,
                        },
                    ),
                );

                // follow and write every scope over one connection to the package's origin, connecting to enclosing homes below its home path
                const connection = ObjectConnection.open({
                    scope: request.caller.id,
                    url: ObjectConnection.url(`${request.root}${ScopePath.of(request.space)}`),
                    homes: (home: string) =>
                        ObjectConnection.open({
                            scope: home,
                            url: ObjectConnection.url(`${request.root}${ScopePath.of(home)}`),
                            environment: traced,
                        }),
                    environment: traced,
                });
                const publisher = (followed: string) => connection.at(followed);

                // serve the person's device in the person's scope
                return await instruments.span("destack.device.client", {}, () =>
                    BrowserDevice.serve({
                        database,
                        channel: broadcastChannel(`destack:${name}`, traced),
                        objects: stored,
                        scope: request.caller.id,
                        caller: request.caller,
                        ...(request.device === undefined ? {} : { device: request.device }),
                        remote: connection,
                        publisher,
                        environment: traced,
                    }),
                );
            });
        },
        message,
    );
}

/** List the object types a device stores, the person's own only on their device. */
export function storedObjects(
    objects: readonly ObjectType[],
    request: Pick<DeviceRequest, "device">,
): Readonly<Record<string, ObjectType>> {
    return deviceObjects(objects, DeviceRole.played(request.device));
}

/** List the object types a scope's roles open among some types, keyed by type. */
export function deviceObjects(
    objects: readonly ObjectType[],
    roles: readonly DeviceRole[],
): Readonly<Record<string, ObjectType>> {
    return Object.fromEntries(
        roles.flatMap((role) =>
            Object.entries(
                role === "user"
                    ? ObjectClient.keyed([setting, deviceSetting, user])
                    : ObjectClient.keyed(
                          objects.filter((object) =>
                              object.livesInType(role === "account" ? account : space),
                          ),
                      ),
            ),
        ),
    );
}
