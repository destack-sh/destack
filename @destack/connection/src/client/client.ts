import { type ClientOptions, createClient } from "@destack/service/client";
import { connectionService } from "../service/service.ts";

/** Connect to a space's connection service. */
export function connect(options: ClientOptions) {
    return createClient(connectionService, options);
}
