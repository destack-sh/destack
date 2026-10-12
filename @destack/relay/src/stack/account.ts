import { accountService } from "@destack/account/service";
import { defineServiceBinding } from "@destack/service/declare";

/** The universe's account service: the copies of the residency's accounts, and the directory. */
export const accounts = defineServiceBinding("accounts", accountService, {
    address: "@destack/account",
});
