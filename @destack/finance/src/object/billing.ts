import { through } from "@destack/access";

/** The permissions of an account's billing objects. */
export const BILLING_PERMISSIONS = {
    read: through("account", "read"),
    bill: through("account", "bill"),
    provide: through("account", "serve"),
};
