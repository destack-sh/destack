import {
    type Context,
    ROOT_CONTEXT,
    type TextMapGetter,
    type TextMapSetter,
} from "@opentelemetry/api";
import type { Environment } from "@destack/environment";

/** The setter writing trace fields into HTTP headers. */
const HEADERS_SETTER: TextMapSetter<Headers> = {
    set: (carrier, name, value) => carrier.set(name, value),
};

/** The getter reading trace fields from HTTP headers. */
const HEADERS_GETTER: TextMapGetter<Headers> = {
    keys: (carrier) => [...carrier.keys()],
    get: (carrier, name) => carrier.get(name) ?? undefined,
};

/** Inject a trace, the telemetry's active one by default, into an outgoing HTTP request. */
export function injectContext(
    headers: Headers,
    telemetry: Pick<Environment["telemetry"], "propagator" | "context">,
    parent: Context = telemetry.context.active(),
): void {
    telemetry.propagator.inject(parent, headers, HEADERS_SETTER);
}

/** Extract a remote trace without inheriting another request's active context. */
export function extractContext(
    headers: Headers,
    telemetry: Pick<Environment["telemetry"], "propagator">,
): Context {
    return telemetry.propagator.extract(ROOT_CONTEXT, headers, HEADERS_GETTER);
}
