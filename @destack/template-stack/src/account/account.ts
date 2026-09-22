import { defineAccount } from "@destack/model/declare";

/** Environments shared by the account's spaces. */
export const account = defineAccount({
    environments: {
        development: {},
        production: {},
    },
});
