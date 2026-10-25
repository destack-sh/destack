import { defineAccount } from "@destack/account/declare";

/** Stages shared by the account's spaces. */
export const account = defineAccount({
    stages: {
        dev: {},
        production: {},
    },
});
