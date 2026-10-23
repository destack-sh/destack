import { Caller, principal } from "@destack/access";
import type { MethodContextOf, NextOf, ResultOf } from "@destack/object";
import { IdentityOperation } from "@destack/identity";
import { schema } from "@destack/schema";
import { Scope, Subject } from "@destack/sync";
import { identity, Identity, operation } from "../object/identity.ts";
import { placement } from "../object/placement.ts";
import type {} from "@destack/package/import-meta";
import { AccountError } from "../error/error.ts";

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
    context: MethodContextOf<typeof identity, "create">,
    next: NextOf<typeof identity, "create">,
): Promise<ResultOf<typeof identity, "create">> {
    // require the identity's first operation
    const subject = context.id;
    const claims = IdentityOperation.read(context.input.operation);
    if (claims.subject !== subject || claims.previous !== null) {
        throw AccountError.of(
            "IDENTITY_NOT_STARTED",
            `the operation does not start an identity of ${subject}`,
        );
    }

    // require the universe for itself, else the machine serving the space
    const sender = Caller.sender(context.caller);
    const isUniverse =
        subject === Scope.universe.id && Subject.same(sender, Caller.principal(context.self));
    const spaceId = schema.identifier("space").safeParse(subject);
    const served = spaceId.success
        ? await context.read(placement).findFirst({ where: { id: spaceId.data } })
        : undefined;
    const isServing =
        served !== undefined &&
        sender !== undefined &&
        principal.machine.is(sender) &&
        sender.id === served.machineId;
    if (!isUniverse && !isServing) {
        throw AccountError.of(
            "FOREIGN_IDENTITY_START",
            `only the machine serving ${subject} can start its identity`,
        );
    }

    // create the identity and append its first operation
    const created = await next();
    await context.mutate(context.caller, [
        operation.call("create", {
            ...operation.in(Scope.universe.id),
            parentId: subject,
            operation: context.input.operation,
        }),
    ]);

    return created;
}
