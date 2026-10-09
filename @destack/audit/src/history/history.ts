import { type EventStore, EventTime } from "@destack/event";
import type { PackageId } from "@destack/package";
import { type Identifier, present } from "@destack/schema";
import { AuditError } from "../error/index.ts";
import { AuditActor, AuditCaller } from "../record/actor.ts";
import { AuditCall, type JournalEntry } from "../record/call.ts";
import { call } from "../record/event.ts";
import { AuditBatch } from "../service/batch.ts";

/** The audited calls of each scope, kept as events of the call kind once each ends, which the event service reads. */
export class AuditHistory {
    /** The events the history keeps its calls in. */
    readonly store: EventStore;

    /** Keep the history in an event store keeping the call kind. */
    constructor(store: EventStore) {
        this.store = store;
    }

    /** Store a batch of ended calls once each and return the stored count. */
    async ingest(value: AuditBatch): Promise<number> {
        // parse the batch of ended calls
        const batch = AuditBatch.parse(value);

        // append each call without its input and result value, once
        await this.store.append(
            call,
            batch.calls.map((ended) => AuditHistory.#event(ended)),
        );

        return batch.calls.length;
    }

    /** Store a batch of an instance's journal its machine relays, recording the instance as each call's provenance. */
    async relay(value: unknown, provenance: AuditProvenance): Promise<number> {
        // parse the batch, refusing one no journal writes
        const parsed = AuditBatch.safeParse(value);
        if (!parsed.success) {
            throw new AuditError("INVALID_EVENT", "invalid journal batch", { cause: parsed.error });
        }

        // refuse calls of another space or package, and calls with a provenance of their own
        const batch = parsed.data;
        for (const relayed of batch.calls) {
            AuditHistory.#requireOrigin(relayed, provenance);
        }

        // record the installation and the instance on each call
        const { installationId, instanceId } = provenance;
        const calls = batch.calls.map((relayed) => {
            const context = { ...relayed.execution.context, installationId, instanceId };

            return { ...relayed, execution: { ...relayed.execution, context } };
        });

        return this.ingest({ calls });
    }

    /** Copy a call without its input and a success's value, running or ended. */
    static kept<Entry extends JournalEntry>(entry: Entry): Entry {
        const outcome = entry.execution.outcome;

        return {
            ...entry,
            input: {},
            execution: {
                ...entry.execution,
                ...(outcome?.kind === "success" ? { outcome: { kind: "success" } } : {}),
            },
        };
    }

    /** Write an ended call as an event of its scope, keyed by its actor, method, package, category, outcome and target. */
    static #event(ended: AuditCall) {
        const { execution } = ended;
        const { target } = execution;

        return {
            scope: execution.context.scope,
            id: execution.id,
            time: EventTime.of(execution.startedAt),
            keys: {
                actor: AuditActor.key(AuditCaller.actor(execution.context.caller)),
                method: ended.method,
                packageId: execution.context.package.id,
                category: execution.category,
                outcome: present(execution.outcome, "an ended call's outcome").kind,
                target: target.id,
                targetType: target.type,
            },
            data: AuditHistory.kept(ended),
        };
    }

    /** Require a relayed call to run in the instance's space and package, without a provenance, which only its machine records. */
    static #requireOrigin(relayed: AuditCall, provenance: AuditProvenance): void {
        // refuse a call of another space or package
        const { context } = relayed.execution;
        if (context.scope !== provenance.scope || context.package.id !== provenance.packageId) {
            throw new AuditError(
                "FORBIDDEN",
                `installation ${provenance.installationId} records no calls of ${context.package.id} in ${context.scope}`,
            );
        }

        // refuse a call that states where it ran
        const isProvenanced =
            context.installationId !== undefined ||
            context.instanceId !== undefined ||
            context.machineId !== undefined;
        if (isProvenanced) {
            throw new AuditError("FORBIDDEN", "only the machine records a call's provenance");
        }
    }
}

/** The instance whose journal a machine relays, as the machine verified it from the instance's secret. */
export interface AuditProvenance {
    /** The space the instance serves. */
    readonly scope: Identifier<"space">;
    /** The package the instance runs. */
    readonly packageId: PackageId;
    /** The installation the instance runs. */
    readonly installationId: Identifier<"installation">;
    /** The instance. */
    readonly instanceId: Identifier<"instance">;
}
