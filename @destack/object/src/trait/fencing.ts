import { type Method } from "../method/method.ts";
import { method } from "../method/record.ts";
import type { Gated, Trait } from "./trait.ts";
import { Execution } from "../method/execution.ts";
import { schema } from "@destack/schema";
import type { Table } from "@destack/db";
import { Scope } from "@destack/sync";

/** The input fencing an object's scope: the machine the scope moves to. */
const FenceInput = schema.object({
    /** The machine the scope moves to. */
    machine: schema.identifier("machine"),
});

/** The method fencing an object's scope for its move to a machine. */
type FenceScopeMethod<Fence extends string> = Method<{
    kind: "custom";
    permission: Fence;
    input: NoInfer<typeof FenceInput>;
    output: never;
    prepared: never;
    mutates: true;
    table: Table;
    internal: true;
    target: "object";
}>;

/** The method lifting the fence of an object's scope. */
type UnfenceScopeMethod<Fence extends string> = Method<{
    kind: "custom";
    permission: Fence;
    input: never;
    output: never;
    prepared: never;
    mutates: true;
    table: Table;
    internal: true;
    target: "object";
}>;

/** The methods of objects fencing their scope while it moves. */
export type FencingMethodMap<Fence> = [Fence] extends [string]
    ? {
          readonly fenceScope: FenceScopeMethod<Fence>;
          readonly unfenceScope: UnfenceScopeMethod<Fence>;
      }
    : {};

/** Objects fencing the scope they live in while it moves between machines, such as a transfer. */
export const fencing: Trait<Gated> & {
    /** Declare the method fencing an object's scope for its move to a machine. */
    readonly fenceScope: <const Permission extends string>(
        permission: Permission,
    ) => FenceScopeMethod<Permission>;
    /** Declare the method lifting the fence of an object's scope. */
    readonly unfenceScope: <const Permission extends string>(
        permission: Permission,
    ) => UnfenceScopeMethod<Permission>;
} = {
    key: "fencing",
    storages: ["durable"],
    options: (definition) => definition.fencing,
    columns: () => ({}),
    constraints: () => [],
    methods: (options) => ({
        fenceScope: fencing.fenceScope(options.by),
        unfenceScope: fencing.unfenceScope(options.by),
    }),
    validate: (_options, object, definition) => {
        // require a type living in a scope it can fence
        if (definition.isScope === true || object.scope === Scope.universe.id) {
            throw new TypeError(`object ${object.name} fences no enclosing scope`);
        }
    },
    fenceScope: (permission) =>
        Execution.handle(
            method.mutation({
                permission,
                input: FenceInput,
                isInternal: true,
                isPredicted: false,
            }),
            async (call, execution) => {
                // fence the scope the object lives in for the machine it moves to
                const { authorization } = Execution.store(execution, call);
                const scope = await Scope.object(Execution.snapshot(execution), call.scope);
                await authorization.fence(scope, call.input.machine);

                return call.target;
            },
        ),
    unfenceScope: (permission) =>
        Execution.handle(
            method.mutation({ permission, isInternal: true, isPredicted: false }),
            async (call, execution) => {
                // lift the fence of the scope the object lives in
                const { authorization } = Execution.store(execution, call);
                const scope = await Scope.object(Execution.snapshot(execution), call.scope);
                await authorization.unfence(scope);

                return call.target;
            },
        ),
};
