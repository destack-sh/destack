import { principal } from "@destack/access";
import type { schema } from "@destack/schema";
import type { Procedure } from "../method/procedure.ts";
import { traitPermissions, type Gated, type Trait } from "./trait.ts";

import { bind, type ConsumerMethod, consumerMethod } from "../method/bindable.ts";
/** The relation of an object's consumers. */
export const CONSUMER = "consumer";

/** The permission binding consumers to the objects. */
const BIND = "bind";

/** The methods of bindable objects: binding them where their rows live, and one at a time. */
export type BindableMethodMap<Bindable> = [Bindable] extends [undefined]
    ? {}
    : [Bindable] extends [Gated<infer Permission>]
      ? {
            readonly bind: typeof bind;
            readonly bindConsumer: ConsumerMethod<Permission>;
            readonly releaseConsumer: ConsumerMethod<Permission>;
        }
      : { readonly bind: typeof bind };

/** The procedure binding derives, which stays out of the routes people call. */
export type BindableProcedures = {
    bind: Procedure<
        schema.Object<Readonly<Record<string, schema.Schema>>>,
        schema.Object<Readonly<Record<string, schema.Schema>>>
    >;
};

/** Objects that installations and machine services bind to and consume, such as secrets. */
export const bindable: Trait<true | Gated> = {
    key: "bindable",
    isDurable: true,
    options: (definition) => definition.bindable,
    columns: () => ({}),
    constraints: () => [],
    policy: (_options, _object, permissions) => ({
        // derive the bind permission unless the type declares its own, as gated types do
        ...traitPermissions(permissions, [BIND]),
        relations: {
            [CONSUMER]: {
                subjects: [principal.installation, principal.machine],
                grantedBy: BIND,
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
