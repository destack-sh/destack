import { AuditEvent } from "./event.ts";
import { AuditError } from "../error/index.ts";

/** Maximum encoded event size accepted by persistence and delivery. */
const MAX_EVENT_BYTES = 65536;

/** Encode a validated, bounded event for durable storage. */
export function encodeEvent(value: AuditEvent): { event: AuditEvent; content: string } {
    const event = AuditEvent.parse(value);

    // require complete scope and occurrence references before persistence
    if (event.context.spaceId && !event.context.accountId) {
        throw new AuditError("INVALID_EVENT", "space audit requires an account");
    }
    if (event.attemptId && (event.result.stage === "attempt" || event.attemptId === event.id)) {
        throw new AuditError("INVALID_EVENT", "only a result can refer to a distinct attempt");
    }

    // bound both database records and individual delivery payloads
    const content = canonical(event);
    if (new TextEncoder().encode(content).byteLength > MAX_EVENT_BYTES) {
        throw new AuditError("INVALID_EVENT", "audit event exceeds 65536 bytes");
    }

    return { event, content };
}

/** Serialize JSON with stable object-key ordering for exact duplicate detection. */
export function canonical(value: unknown): string {
    if (value === null || typeof value !== "object") {
        return JSON.stringify(value);
    } else if (Array.isArray(value)) {
        return `[${value.map(canonical).join(",")}]`;
    } else {
        const fields = Object.entries(value)
            .filter(([, value]) => value !== undefined)
            .sort(([left], [right]) => (left < right ? -1 : left > right ? 1 : 0));

        return `{${fields.map(([key, value]) => `${JSON.stringify(key)}:${canonical(value)}`).join(",")}}`;
    }
}
