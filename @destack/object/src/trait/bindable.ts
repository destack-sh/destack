import { principal } from "@destack/access";
import type { Trait } from "./trait.ts";

/** The relation of the installations whose live deployments captured an object, and so use it. */
export const CONSUMER = "consumer";

/** Objects installations bind to: resources, secrets and the like, used by the installations capturing them. */
export const bindable: Trait<true> = {
    key: "bindable",
    isDurable: true,
    options: (definition) => definition.bindable,
    columns: () => ({}),
    constraints: () => [],
    policy: () => ({
        relations: { [CONSUMER]: { subjects: [principal.installation], grantedBy: null } },
    }),
    methods: () => ({}),
};
