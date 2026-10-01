import { type ClientOptions, createClient } from "@destack/service/client";
import { registryService } from "../service/service.ts";

/** Connect to registry administration. */
export function connect(options: ClientOptions) {
    return createClient(registryService, options);
}
