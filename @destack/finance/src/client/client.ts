import { type ClientOptions, createClient } from "@destack/service/client";
import { financeService } from "../service/service.ts";

/** Connect to the finance service. */
export function connect(options: ClientOptions) {
    return createClient(financeService, options);
}
