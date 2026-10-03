import {
    type Context,
    context,
    propagation,
    ROOT_CONTEXT,
    type TextMapGetter,
    type TextMapPropagator,
    type TextMapSetter,
} from "@opentelemetry/api";

/** The setter writing trace fields into HTTP headers. */
const HEADERS_SETTER: TextMapSetter<Headers> = {
    set: (carrier, name, value) => carrier.set(name, value),
};

/** The getter reading trace fields from HTTP headers. */
const HEADERS_GETTER: TextMapGetter<Headers> = {
    keys: (carrier) => [...carrier.keys()],
    get: (carrier, name) => carrier.get(name) ?? undefined,
};

/** Inject the active trace into an outgoing HTTP request. */
export function injectContext(
    headers: Headers,
    parent: Context = context.active(),
    propagator: Pick<TextMapPropagator, "inject"> = propagation,
): void {
    propagator.inject(parent, headers, HEADERS_SETTER);
}

/** Extract a remote trace without inheriting another request's active context. */
export function extractContext(
    headers: Headers,
    propagator: Pick<TextMapPropagator, "extract"> = propagation,
): Context {
    return propagator.extract(ROOT_CONTEXT, headers, HEADERS_GETTER);
}
