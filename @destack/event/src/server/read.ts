import { type JsonCondition, Snapshot } from "@destack/db";
import { type JsonValue, schema } from "@destack/schema";
import { ServiceError } from "@destack/service";
import type { ServiceContext } from "@destack/service/server";
import { type ObjectReference, Scope } from "@destack/sync";
import { Event, type EventKind } from "../kind/kind.ts";
import type { EventStore } from "../store/store.ts";

/** The value readers see in place of a personal value. */
const MASK = "****";

/** A read that showed a kind's personal values unmasked: who read which kind of which object how. */
export interface UnmaskedRead {
    /** The scope read. */
    readonly scope: string;
    /** The object whose grants decided the read: the scope's own, or the one the read narrowed to. */
    readonly object: ObjectReference;
    /** The kind read. */
    readonly kind: EventKind;
    /** The read, such as query or tail. */
    readonly operation: string;
}

/** An event as the event service answers it. */
export type ShownEvent = schema.Infer<typeof Event>;

/** What a read of a store's events names: the kind, the scope and object it reads, and the operation. */
export interface ReadRequest {
    /** The store read. */
    readonly store: EventStore;
    /** The kind read. */
    readonly kind: EventKind;
    /** The scope read, narrowed to an object when named, its own events unless it reads within. */
    readonly selection: {
        readonly scope: string;
        readonly object?: string | undefined;
        readonly within?: boolean | undefined;
    };
    /** The operation, such as query or tail. */
    readonly operation: string;
    /** Record a read that showed personal values unmasked, absent where nobody unmasks them. */
    readonly unmasked?:
        | ((context: ServiceContext, read: UnmaskedRead) => Promise<void>)
        | undefined;
}

/** Require the caller to read a kind of a scope, answering the read's condition and redaction. */
export async function authorizeRead(
    context: ServiceContext,
    request: ReadRequest,
): Promise<{ readonly where: JsonCondition; readonly redact: (event: Event) => ShownEvent }> {
    // find the object whose grants decide, refusing a kind nobody reads here
    const { kind, selection } = request;
    const { access } = kind;
    if (access === undefined) {
        throw new ServiceError("NOT_FOUND", { message: `no readable event kind ${kind.name}` });
    }
    const object = await governing(request);

    // require the read permission on it
    const authorization = context.requireAuthorization();
    await authorization.require(access.read, object);

    // narrow to the object and to the scope's own events unless the read reaches within
    const where: JsonCondition = {
        ...(selection.object === undefined || access.object === undefined
            ? {}
            : { [access.object.key]: selection.object }),
        ...(selection.within === true ? {} : { source: selection.scope }),
    };

    // show personal values to a caller who may unmask them, recording the read
    const record = request.unmasked;
    const isUnmasked =
        record !== undefined &&
        access.unmask !== undefined &&
        (await authorization.check(access.unmask, object)).isAllowed;
    if (!isUnmasked) {
        return { where, redact: (event) => Event.parse(masked(kind, event)) };
    }
    await record(context, {
        scope: selection.scope,
        object,
        kind,
        operation: request.operation,
    });

    return { where, redact: (event) => Event.parse(event) };
}

/** Find the object deciding a read: the one it narrows to, or the scope's own object. */
async function governing(request: ReadRequest): Promise<ObjectReference> {
    // decide on the object a read narrows to
    const { kind, selection } = request;
    const narrowed = kind.access?.object;
    if (selection.object !== undefined) {
        if (narrowed === undefined) {
            throw new ServiceError("BAD_REQUEST", {
                message: `event kind ${kind.name} narrows to no object`,
            });
        }

        return narrowed.reference(selection.scope, selection.object);
    }

    // decide on the scope's own object
    const [link] = await Scope.chain(Snapshot.live(request.store.database), selection.scope);
    if (link === undefined) {
        throw new ServiceError("NOT_FOUND", { message: `no scope ${selection.scope}` });
    }

    return link.object;
}

/** Replace each personal value of an event's data with the mask, entry by entry. */
function masked(kind: EventKind, event: Event): Event {
    return {
        ...event,
        data: kind.mapSensitive(event.data, (value, sensitivity) =>
            sensitivity === "personal" ? maskOf(value) : value,
        ),
    };
}

/** Mask a value, a record or a list entry by entry. */
function maskOf(value: JsonValue): JsonValue {
    if (Array.isArray(value)) {
        return value.map(maskOf);
    } else if (typeof value === "object" && value !== null) {
        return Object.fromEntries(Object.keys(value).map((key) => [key, MASK]));
    }

    return MASK;
}
