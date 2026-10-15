import { Caller, principal } from "@destack/access";
import type { CallOf, NextOf, ResultOf } from "@destack/object";
import { IdentityOperation } from "@destack/identity";
import { schema } from "@destack/schema";
import { CallError } from "@destack/runtime/error";
import { Scope, Subject } from "@destack/sync";
import { identity, Identity, operation } from "../object/identity.ts";
import { placement } from "../object/placement.ts";

/** Implement identities. */
export function implementIdentity(): typeof identity {
    return identity.implement({
        methods: {
            create,
            keys: (call) => Identity.keys(call, call.target.id),
        },
    });
}

/** Start an identity as the machine serving its space, or as the universe for itself. */
async function create(
    call: CallOf<typeof identity, "create">,
    next: NextOf<typeof identity, "create">,
): Promise<ResultOf<typeof identity, "create">> {
    // require the identity's first operation
    const subject = call.requireId();
    const claims = IdentityOperation.read(call.input.operation);
    if (claims.subject !== subject || claims.previous !== null) {
        throw new CallError("BAD_REQUEST", {
            message: `the operation starts no identity of ${subject}`,
        });
    }

    // require the universe for itself, else the machine serving the space
    const sender = Caller.sender(call.caller);
    const isUniverse =
        subject === Scope.universe.id &&
        sender !== undefined &&
        Subject.same(sender, Caller.requirePrincipal(call.self));
    const spaceId = schema.identifier("space").safeParse(subject);
    const served = spaceId.success
        ? await call.read(placement).findFirst({ where: { id: spaceId.data } })
        : undefined;
    const isServing =
        served !== undefined &&
        sender !== undefined &&
        principal.machine.is(sender) &&
        sender.id === served.machineId;
    if (!isUniverse && !isServing) {
        throw new CallError("FORBIDDEN", {
            message: `only the machine serving ${subject} starts its identity`,
        });
    }

    // create the identity and append its first operation
    const created = await next();
    await call.mutate(call.caller, [
        operation.call("create", {
            ...operation.in(Scope.universe.id),
            parentId: subject,
            operation: call.input.operation,
        }),
    ]);

    return created;
}
