import { type ClientOptions, createClient } from "@destack/service/client";
import { spaceService } from "../service/service.ts";

/** Connect to space administration. */
export function connect(options: ClientOptions) {
    return createClient(spaceService, options);
}
