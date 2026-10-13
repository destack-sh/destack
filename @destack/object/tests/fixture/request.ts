import type { RequestContext } from "@destack/runtime/server";
import { RequestId } from "@destack/runtime/request";
import type { CallableName, CallOutput, ObjectType } from "../../src/index.ts";
import type { ObjectStore } from "../../src/server/index.ts";

/** Run one method as a request carries it: a query for a reading method, else a one-call mutation under the request's identifier or writer. */
export async function request<Type extends ObjectType, Name extends CallableName<Type>>(
    store: ObjectStore,
    object: Type,
    name: Name,
    input: Readonly<Record<string, unknown>>,
    context: RequestContext,
): Promise<CallOutput<Type, Name>>;
/**
 * Run one method as a request carries it.
 *
 * @construct the result is the method's result, which CallOutput reads from the same procedure.
 */
export async function request(
    store: ObjectStore,
    object: ObjectType,
    name: string,
    input: Readonly<Record<string, unknown>>,
    context: RequestContext,
): Promise<unknown> {
    // read through a query
    const { requestId, writerId, ...rest } = input;
    const call = object.call(name, rest);
    if (!object.method(name).mutates) {
        return store.query(context.caller(), call);
    }

    // write through a one-call mutation under the request or the writer, and report the scope's watermark
    const id = typeof requestId === "string" ? requestId : RequestId.create(store.clock.now());
    const options = typeof writerId === "string" ? { writerId } : {};
    const [written] = await store.mutate(context.caller(), { id, calls: [call] }, options);
    const field = object.route.field;
    const scope = field === undefined ? undefined : rest[field];
    if (typeof scope === "string") {
        context.observed.observe(await store.watermark(scope));
    }

    return written;
}
