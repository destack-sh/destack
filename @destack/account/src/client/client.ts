import { type ClientOptions, createClient } from "@destack/service/client";
import { accountService } from "../service/service.ts";

/** Connect to account administration. */
export function connect(options: ClientOptions) {
    return createClient(accountService, options);
}
