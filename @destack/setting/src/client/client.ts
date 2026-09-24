import { createClient, type ClientOptions } from "@destack/service/client";
import { settingService } from "../service/index.ts";

/** Connect to a host's setting service using the shared authenticated transport. */
export function createSettingClient(options: ClientOptions) {
    return createClient(settingService.router, options);
}
