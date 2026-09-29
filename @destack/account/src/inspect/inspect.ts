import { describeRouter } from "@destack/service/inspect";
import { accountService } from "../service/index.ts";

/** Describe the account procedures. */
export function inspect() {
    return describeRouter(accountService.name, accountService.router);
}
