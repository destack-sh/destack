import { ServiceConnectionDescription, type ServiceConnection } from "../declare/connection.ts";

/** Describe a service dependency without serializing its executable router. */
export function describeServiceConnection(
    connection: ServiceConnection,
): ServiceConnectionDescription {
    return { packageId: connection.package.id, name: connection.name, service: connection.service };
}

export { ServiceConnectionDescription };
