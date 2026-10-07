import { principal } from "@destack/access";
import type { schema } from "@destack/schema";
import type { Procedure } from "../method/procedure.ts";
import type { Gated, Trait } from "./trait.ts";

import { bind, type ConsumerMethod, consumerMethod } from "../method/bindable.ts";
/** The relation of the consumers of an object: the installations whose live deployments captured it, and the cell services reading it on their behalf, such as connections reading their credentials. */
export const CONSUMER = "consumer";

/** The methods bindable objects take: binding them to installations where their rows live, and one consumer at a time by a permission's holders. */
export type BindableMethodMap<Bindable> = [Bindable] extends [undefined]
    ? {}
    : [Bindable] extends [Gated<infer Permission>]
      ? {
            readonly bind: typeof bind;
            readonly bindConsumer: ConsumerMethod<Permission>;
            readonly releaseConsumer: ConsumerMethod<Permission>;
        }
      : { readonly bind: typeof bind };

/** The procedure binding derives, which only the system calls. */
export type BindableProcedures = {
    bind: Procedure<
        schema.Object<Readonly<Record<string, schema.Schema>>>,
        schema.Object<Readonly<Record<string, schema.Schema>>>
    >;
};

/** Objects installations and cell services bind to: resources, secrets, connections and the like, used by their consumers. */
export const bindable: Trait<true | Gated> = {
    key: "bindable",
    isDurable: true,
    options: (definition) => definition.bindable,
    columns: () => ({}),
    constraints: () => [],
    policy: () => ({
        relations: {
            [CONSUMER]: {
                subjects: [principal.installation, principal.cell, principal.workload],
                grantedBy: null,
            },
        },
    }),
    methods: (options) =>
        options === true
            ? { bind }
            : {
                  bind,
                  bindConsumer: consumerMethod(options.by, "bind"),
                  releaseConsumer: consumerMethod(options.by, "release"),
              },
};
