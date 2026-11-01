import { channel } from "node:diagnostics_channel";
import type * as Cloudflare from "@cloudflare/workers-types";
import { type Identifier, schema } from "@destack/schema";
import type { WorkloadStart } from "@destack/object/workload";
import type { TraceCount } from "../gateway/index.ts";

/** The diagnostics channel an invocation publishes the space it works for on, in its trace. */
export const ATTRIBUTION_CHANNEL = "destack.attribution";

/** The path of a space's object that takes the work its traces measured. */
export const TRACES_PATH = "/.destack/traces";

/** The space an invocation works for, with the installation it runs. */
const attributionSchema = schema.object({
    /** The space. */
    scope: schema.identifier("space"),
    /** The installation, absent for work the space does itself. */
    installation: schema.identifier("installation").exactOptional(),
});
/** The space an invocation works for, with the installation it runs. */
export type Attribution = schema.Infer<typeof attributionSchema>;

/** The fields of a Workers trace the tail reads. */
export type Trace = Pick<Cloudflare.TraceItem, "cpuTime" | "diagnosticsChannelEvents">;

/** The channel the attributions go out on. */
const attributions = channel(ATTRIBUTION_CHANNEL);

/** The space an invocation works for, published in its trace and read by the machine's tail. */
export const Attribution = Object.assign(attributionSchema, {
    /** Read a workload start's space, which every worker serves, and its installation. */
    of(start: Pick<WorkloadStart, "scope" | "installation">): Attribution {
        return {
            scope: schema.identifier("space").parse(start.scope),
            installation: start.installation,
        };
    },

    /** Publish the space and installation the current invocation works for in its trace. */
    publish(attribution: Attribution): void {
        attributions.publish(attribution);
    },

    /** Add up each space's CPU time from traces by their attribution. */
    read(traces: readonly Trace[]): Map<Identifier<"space">, TraceCount[]> {
        const spaces = new Map<Identifier<"space">, Map<string, TraceCount>>();
        for (const trace of traces) {
            // leave out a trace attributed to no space, such as the tail's own posts
            const published = trace.diagnosticsChannelEvents.find(
                (event) => event.channel === ATTRIBUTION_CHANNEL,
            );
            if (published === undefined) {
                continue;
            }

            // add the trace's CPU time
            const { scope, installation } = attributionSchema.parse(published.message);
            const counts = spaces.get(scope) ?? new Map<string, TraceCount>();
            const counted = counts.get(installation ?? scope);
            counts.set(installation ?? scope, {
                ...(installation === undefined ? {} : { installation }),
                cpuMilliseconds: (counted?.cpuMilliseconds ?? 0) + trace.cpuTime,
            });
            spaces.set(scope, counts);
        }

        return new Map([...spaces].map(([scope, counts]) => [scope, [...counts.values()]]));
    },
});
