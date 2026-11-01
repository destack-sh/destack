import { type DurableObjectDatabaseBindings, Jurisdiction } from "@destack/db/cloudflare";
import type { Identifier } from "@destack/schema";
import type { TraceCount } from "../gateway/index.ts";
import { Attribution, TRACES_PATH, type Trace } from "./attribution.ts";
import { PartitionName } from "./runtime/index.ts";
import type {} from "@destack/package/import-meta";
import { MachineError } from "../error/index.ts";

/** The cloud machine's Tail Worker, posting each space's traced work to the space's object. */
export class MachineTail {
    /** Post the CPU time of the machine's traces to their spaces. */
    async tail(traces: Trace[], bindings: DurableObjectDatabaseBindings): Promise<void> {
        await Promise.all(
            [...Attribution.read(traces)].map(([scope, counts]) =>
                MachineTail.#post(bindings, scope, counts),
            ),
        );
    }

    /** Post a space's traced work to the object of the space's own partition. */
    static async #post(
        bindings: DurableObjectDatabaseBindings,
        scope: Identifier<"space">,
        counts: readonly TraceCount[],
    ): Promise<void> {
        // address the space's object in its jurisdiction
        const namespace = Jurisdiction.narrow(bindings.PARTITION, bindings.DESTACK_JURISDICTION);
        const object = namespace.get(namespace.idFromName(PartitionName.of({ space: scope })));

        // post the counts, refusing a failed answer
        const response = await object.fetch(
            new Request(`https://space${TRACES_PATH}`, {
                method: "POST",
                body: JSON.stringify(counts),
            }),
        );
        const answer = await response.text();
        if (!response.ok) {
            throw MachineError.of(
                "TRACE_REFUSED",
                `space ${scope} rejected traced work: ${response.status} ${answer}`,
            );
        }
    }
}

/** The cloud machine's Tail Worker, one per isolate. */
export default new MachineTail();
