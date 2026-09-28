import { ServiceConnectionDescription, type ServiceConnection } from "../declare/connection.ts";

/** Describe a service connection. */
export function describeServiceConnection(
    connection: ServiceConnection,
): ServiceConnectionDescription {
    return { packageId: connection.package.id, name: connection.name, service: connection.service };
}

export { ServiceConnectionDescription };
