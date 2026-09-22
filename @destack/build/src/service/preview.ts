import { schema } from "@destack/schema";
import { OperationError } from "@destack/service/operation";
import { defineProcedure, eventIterator } from "@destack/service";

/** Fields shared by every preview state. */
const fields = schema.object({
    /** Identifier assigned by the build service. */
    id: schema.uuid(),
    /** Editable source reference resolved and authorized by the host. */
    source: schema.string().min(1),
    /** Named application selected from the source configuration. */
    application: schema.string().min(1),
    /** Creation time in UTC milliseconds. */
    createdAt: schema.number().int(),
    /** Last lifecycle change in UTC milliseconds. */
    updatedAt: schema.number().int(),
});

/** A running application updated from an editable package. */
export const Preview = schema.discriminatedUnion("state", [
    fields.extend({ state: schema.literal("starting") }),
    fields.extend({ state: schema.literal("running"), url: schema.url() }),
    fields.extend({ state: schema.literal("stopping") }),
    fields.extend({ state: schema.literal("stopped"), stoppedAt: schema.number().int() }),
    fields.extend({
        state: schema.literal("failed"),
        stoppedAt: schema.number().int(),
        error: OperationError,
    }),
]);

/** Observable preview state. */
export type Preview = schema.Infer<typeof Preview>;

/** Select an editable source and its named application settings. */
export const PreviewRequest = schema.object({
    /** Editable source reference resolved by the host. */
    source: schema.string().min(1),
    /** Named application declared in the host's source configuration. */
    application: schema.string().min(1),
});

/** Preview selection supplied by a client. */
export type PreviewRequest = schema.Infer<typeof PreviewRequest>;

/** Preview identifier shared by lifecycle requests. */
const key = schema.object({ id: schema.uuid() });

/** Manage running application previews under host authorization. */
export const preview = {
    start: access("start")
        .route({ method: "POST", path: "/previews" })
        .input(PreviewRequest)
        .output(Preview),
    get: access("read").route({ method: "GET", path: "/previews/{id}" }).input(key).output(Preview),
    list: access("read").route({ method: "GET", path: "/previews" }).output(schema.array(Preview)),
    watch: access("read")
        .route({ method: "GET", path: "/previews/{id}/watch" })
        .input(key)
        .output(eventIterator(Preview)),
    stop: access("stop")
        .route({ method: "POST", path: "/previews/{id}/stop" })
        .input(key)
        .output(Preview),
};

/** Declare preview permissions and audit lifecycle changes. */
function access(action: "read" | "start" | "stop") {
    return defineProcedure({
        authentication: "identity",
        permission: { resource: "destack.preview", action },
        audit: action !== "read",
    });
}
