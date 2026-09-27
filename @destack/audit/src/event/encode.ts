import { canonicalize } from "@destack/schema/json";
import { AuditEvent } from "./event.ts";
import { AuditError } from "../error/index.ts";

/** Maximum encoded event size accepted by persistence and delivery. */
const MAX_EVENT_BYTES = 65536;

/** Encode a validated, bounded event for durable storage. */
export function encodeEvent(value: AuditEvent): { event: AuditEvent; content: string } {
    // validate the event
    const event = AuditEvent.parse(value);

    // require a distinct occurrence reference before persistence
    if (event.attemptId && (event.result.stage === "attempt" || event.attemptId === event.id)) {
        throw new AuditError("INVALID_EVENT", "only a result can refer to a distinct attempt");
    }

    // bound both database records and individual delivery payloads
    const content = canonicalize(event);
    if (new TextEncoder().encode(content).byteLength > MAX_EVENT_BYTES) {
        throw new AuditError("INVALID_EVENT", `audit event exceeds ${MAX_EVENT_BYTES} bytes`);
    }

    return { event, content };
}
