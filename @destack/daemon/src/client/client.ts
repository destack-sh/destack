import { type ClientOptions, createClient } from "@destack/service/client";
import { daemonService } from "../service/index.ts";

/** Connect to an authenticated host administration endpoint. */
export function connect(options: ClientOptions) {
    return createClient(daemonService, options);
}
