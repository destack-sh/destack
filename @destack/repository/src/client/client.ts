import { type ClientOptions, createClient } from "@destack/service/client";
import { repositoryService } from "../service/service.ts";

/** Connect to repository administration. */
export function connect(options: ClientOptions) {
    return createClient(repositoryService, options);
}
