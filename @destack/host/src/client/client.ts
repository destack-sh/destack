import { type ClientOptions, createClient } from "@destack/service/client";
import { hostService } from "../service/service.ts";

/** Connect to host administration in the global tier. */
export function connect(options: ClientOptions) {
    return createClient(hostService.router, options);
}
