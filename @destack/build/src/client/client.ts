import { createClient, type ClientOptions } from "@destack/service/client";
import { buildService } from "../service/index.ts";

/** Connect to builds and previews using the shared authenticated HTTP transport. */
export function connect(options: ClientOptions) {
    return createClient(buildService, options);
}
