import { accountService } from "@destack/account/service";
import { defineServiceBinding } from "@destack/service/declare";

/** The universe's account service, which the installation copies its residency's accounts from. */
export const accounts = defineServiceBinding("accounts", accountService, {
    address: "@destack/account",
});
