import {
    type Context,
    ROOT_CONTEXT,
    type TextMapGetter,
    type TextMapSetter,
} from "@opentelemetry/api";
import type { Telemetry } from "./telemetry.ts";

/** The setter writing trace fields into HTTP headers. */
const HEADERS_SETTER: TextMapSetter<Headers> = {
    set: (carrier, name, value) => carrier.set(name, value),
};

/** The getter reading trace fields from HTTP headers. */
const HEADERS_GETTER: TextMapGetter<Headers> = {
    keys: (carrier) => [...carrier.keys()],
    get: (carrier, name) => carrier.get(name) ?? undefined,
};

/** The trace context HTTP headers carry between processes (W3C Trace Context). */
export const TraceHeaders = {
    /** Write a trace, the telemetry's active one by default, into an outgoing request's headers. */
    inject(
        headers: Headers,
        telemetry: Pick<Telemetry, "propagator" | "context">,
        parent: Context = telemetry.context.active(),
    ): void {
        telemetry.propagator.inject(parent, headers, HEADERS_SETTER);
    },

    /** Read a remote trace from a request's headers, inheriting no active context. */
    extract(headers: Headers, telemetry: Pick<Telemetry, "propagator">): Context {
        return telemetry.propagator.extract(ROOT_CONTEXT, headers, HEADERS_GETTER);
    },
};
