import type { Environment } from "@destack/environment";
import type { BrowserCapability } from "@destack/environment/browser";
import { broadcastChannel, connectWasmSqlite, WasmClient } from "@destack/db/browser";
import { BrowserDevice, type DeviceWorkerScope } from "@destack/object/device/browser";
import { ObjectConnection } from "@destack/object/client";
import { ObjectClient } from "@destack/object/device";
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
import { ScopePath } from "../declare/context.ts";
import { DeviceRequest, DeviceRole } from "../browser/request.ts";

export { BrowserEnvironment } from "@destack/environment/browser";

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
                const stored = DeviceRole.objects(objects, DeviceRole.played(request.device));
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
