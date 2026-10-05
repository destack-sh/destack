import { principal } from "@destack/access";
import type { schema } from "@destack/schema";
import type { Procedure } from "../method/procedure.ts";
import type { Trait } from "./trait.ts";

import { bind } from "../method/bindable.ts";
/** The relation of the installations whose live deployments captured an object, and so use it. */
export const CONSUMER = "consumer";

/** The methods bindable objects take: binding them to installations where their rows live. */
export type BindableMethodMap<Bindable> = [Bindable] extends [undefined]
    ? {}
    : { readonly bind: typeof bind };

/** The procedure binding derives, which only the system calls. */
export type BindableProcedures = {
    bind: Procedure<
        schema.Object<Readonly<Record<string, schema.Schema>>>,
        schema.Object<Readonly<Record<string, schema.Schema>>>
    >;
};

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
    methods: () => ({ bind }),
};
