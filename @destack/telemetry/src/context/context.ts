import {
    type Context,
    context,
    propagation,
    ROOT_CONTEXT,
    type TextMapPropagator,
} from "@opentelemetry/api";

/** Inject the active trace into an outgoing HTTP request. */
export function injectContext(
    headers: Headers,
    parent: Context = context.active(),
    propagator: Pick<TextMapPropagator, "inject"> = propagation,
): void {
    propagator.inject(parent, headers, {
        set: (headers, name, value) => headers.set(name, value),
    });
}

/** Extract a remote trace without inheriting another request's active context. */
export function extractContext(
    headers: Headers,
    propagator: Pick<TextMapPropagator, "extract"> = propagation,
): Context {
    return propagator.extract(ROOT_CONTEXT, headers, {
        keys: (headers) => [...headers.keys()],
        get: (headers, name) => headers.get(name) ?? undefined,
    });
}
