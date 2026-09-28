import { defineAccount } from "@destack/account/declare";

/** Environments shared by the account's spaces. */
export const account = defineAccount({
    environments: {
        development: {},
        production: {},
    },
});
