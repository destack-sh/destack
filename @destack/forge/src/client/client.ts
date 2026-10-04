import { type ClientOptions, createClient } from "@destack/service/client";
import { forgeService } from "../service/service.ts";

/** Connect to forge administration. */
export function connect(options: ClientOptions) {
    return createClient(forgeService, options);
}
