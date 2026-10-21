import type { MethodContextOf, NextOf, ResultOf } from "@destack/object";
import { IdentityOperation, type PublicKey } from "@destack/identity";
import { CallError } from "@destack/object";
import { Identity, operation, type Operation } from "../object/identity.ts";

/** How long a higher-priority rotation key may nullify operations a lower one signed. */
export const RECOVERY_MILLISECONDS = 72 * 60 * 60 * 1000;

/** Implement identity operations. */
export function implementOperation(): typeof operation {
    return operation.implement({
        methods: { create },
    });
}

/** Append a signed operation after one of the identity's current operations. */
async function create(
    context: MethodContextOf<typeof operation, "create">,
    next: NextOf<typeof operation, "create">,
): Promise<ResultOf<typeof operation, "create">> {
    // read the operation and the identity's log
    const subject = context.requireParent().id;
    const signed = context.input.operation;
    const claims = IdentityOperation.read(signed);
    const log = await Identity.log(context, subject);
    if (claims.subject !== subject) {
        throw new CallError("BAD_REQUEST", {
            message: `the operation changes ${claims.subject}, not ${subject}`,
        });
    }

    // sign the first by its own rotation keys, a later one by the keys of the operation it follows
    const current = log.filter((entry) => !entry.isNullified);
    const previous = current.find((entry) => entry.digest === claims.previous);
    const priority = await signer(
        signed,
        log.length === 0 && claims.previous === null
            ? claims.rotationKeys
            : following(previous, subject).rotationKeys,
    );

    // nullify the operations after the one it follows, within their recovery window
    const later =
        previous === undefined ? [] : current.filter((entry) => entry.sequence > previous.sequence);
    const [first] = later;
    if (first !== undefined && !isRecoverable(first, priority, context.now)) {
        throw new CallError("CONFLICT", {
            message: `the operation cannot nullify the later operations of ${subject}`,
        });
    }
    await context.mutate(
        context.caller,
        later.map((entry) =>
            operation.call("nullify", { ...operation.target(entry), isNullified: true }),
        ),
    );

    // append it to the log
    return next(
        context.with({
            input: {
                ...context.input,
                sequence: log.length,
                digest: await IdentityOperation.digest(signed),
                priority,
                appliedAt: context.now,
                isNullified: false,
            },
        }),
    );
}

/** Read the keys of the current operation an operation follows, refusing none. */
function following(previous: Operation | undefined, subject: string) {
    if (previous === undefined) {
        throw new CallError("CONFLICT", {
            message: `the operation follows no current operation of ${subject}`,
        });
    }

    return IdentityOperation.read(previous.operation);
}

/** Find the priority of the rotation key that signed an operation, refusing any other signer. */
async function signer(signed: string, rotationKeys: readonly PublicKey[]): Promise<number> {
    const priority = await IdentityOperation.verify(signed, rotationKeys);
    if (priority === undefined) {
        throw new CallError("UNAUTHORIZED", {
            message: "no rotation key of the identity signed the operation",
        });
    }

    return priority;
}

/** Report whether a higher-priority key may still nullify an operation at a time. */
function isRecoverable(entry: Operation, priority: number, now: number): boolean {
    return priority < entry.priority && entry.appliedAt + RECOVERY_MILLISECONDS >= now;
}
