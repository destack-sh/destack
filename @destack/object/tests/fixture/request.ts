import type { RequestContext } from "@destack/runtime/server";
import type { Call } from "@destack/sync";
import { RequestId } from "@destack/runtime/request";
import type { CallableName, MethodInput, MethodOutput, ObjectType } from "../../src/index.ts";
import type { ObjectStore } from "../../src/server/index.ts";

/** A request's input: the method's input under a request or, for ephemeral objects, a writer. */
export type RequestInput<Type extends ObjectType, Name extends CallableName<Type>> = MethodInput<
    Type,
    Name
> & { readonly requestId?: string; readonly writerId?: string };

/** Run one method in a scope as a request sends it: a query, else a one-call mutation. */
export async function request<Type extends ObjectType, Name extends CallableName<Type>>(
    store: ObjectStore,
    object: Type,
    name: Name,
    scope: string,
    input: NoInfer<RequestInput<Type, Name>>,
    context: RequestContext,
) {
    // place the call in the scope, sending the request and writer apart from its input
    const { requestId, writerId, ...rest } = input;
    const call: Call<MethodOutput<Type, Name>> = {
        type: object.typeReference,
        method: name,
        input: { ...rest, ...object.in(scope) },
        release: object.package.version,
    };

    // read through a query
    if (!object.method(name).mutates) {
        return store.query(context.access(), call);
    }

    // write through a one-call mutation under the request or the writer, and report the store's watermark
    const id = requestId ?? RequestId.create(store.clock.now());
    const options = writerId === undefined ? {} : { writerId };
    const [written] = await store.mutate(context.access(), { id, calls: [call] }, options);
    if (object.route.field !== undefined) {
        context.observed.observe(await store.watermark());
    }

    return written;
}
