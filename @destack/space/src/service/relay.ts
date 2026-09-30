import { LogPosition } from "@destack/db/log";
import { Chunk, CopyStage } from "@destack/resource";
import { identifier, schema } from "@destack/schema";
import { defineProcedure, eventIterator } from "@destack/service";
import { QueryPage } from "@destack/sync";
import { SpaceKey } from "./key.ts";

/** Where a source stood once fenced: the log position the target's copy reaches, and the digest of the space's rows at it. */
export const Fence = schema.object({
    /** The source's log position once fenced. */
    position: LogPosition,
    /** The SHA-256 digest of the space's rows at the position, as hexadecimal. */
    digest: schema.string().min(1),
});
/** Where a source stood once fenced. */
export type Fence = schema.Infer<typeof Fence>;

/** One record of a resource's export: a chunk of its content, or the end once every chunk was sent. */
export const ExportRecord = schema.union([
    schema.object({
        /** The next chunk. */
        chunk: Chunk,
    }),
    schema.object({
        /** Whether the export sent every chunk. */
        end: schema.literal(true),
    }),
]);
/** One record of a resource's export. */
export type ExportRecord = schema.Infer<typeof ExportRecord>;

/** The relay of a space's zone from a transfer's source to its target. */
export const relay = {
    /** Follow the space's rows from a position, or from a snapshot without one. */
    watch: defineProcedure({ authentication: "identity", permission: null, audit: false })
        .route({ method: "POST", path: "/spaces/{spaceId}/relay/watch" })
        .input(
            SpaceKey.extend({
                /** The log position the target's copy reached, absent before its first snapshot. */
                after: LogPosition.optional(),
            }),
        )
        .output(eventIterator(QueryPage)),
    /** Fence the space's reads and writes, answering the source's position. */
    fence: defineProcedure({ authentication: "identity", permission: null, audit: "activity" })
        .route({ method: "POST", path: "/spaces/{spaceId}/relay/fence" })
        .input(SpaceKey)
        .output(Fence),
    /** Read a resource's content after a cursor, sealing secrets to the target's recipient key, and mark the end. */
    export: defineProcedure({ authentication: "identity", permission: null, audit: false })
        .route({ method: "POST", path: "/spaces/{spaceId}/relay/resources/{resourceId}/export" })
        .input(
            SpaceKey.extend({
                /** The resource. */
                resourceId: identifier("resource"),
                /** How far the copy goes. */
                stage: CopyStage,
                /** The cursor of the last chunk the target imported, absent at the start. */
                after: schema.string().min(1).optional(),
                /** The target's recipient key for this copy, as base64 of its P-256 point. */
                recipient: schema.base64(),
            }),
        )
        .output(eventIterator(ExportRecord)),
};
