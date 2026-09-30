import { createClient, type ClientOptions } from "@destack/service/client";
import { settingService } from "../service/index.ts";

/** Connect to a host's setting service. */
export function connect(options: ClientOptions) {
    return createClient(settingService, options);
}
