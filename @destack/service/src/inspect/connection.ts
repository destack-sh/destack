import { ServiceConnectionDeclaration, type ServiceConnection } from "../declare/connection.ts";

/** Describe a service dependency without serializing its executable router. */
export function describeServiceConnection(
    connection: ServiceConnection,
): ServiceConnectionDeclaration {
    return { packageId: connection.packageId, name: connection.name, service: connection.service };
}

export { ServiceConnectionDeclaration };
