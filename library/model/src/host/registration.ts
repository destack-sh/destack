import { check, identifier, integer, type Select, sql, table, text } from "@destack/db";
import { serverConnection } from "./server.ts";

/** A space registered with the local daemon for local or remote access. */
export const spaceRegistration = table("space_registration", {
    /** The space identifier; the authoritative record can be remote. */
    spaceId: identifier("space_id", "space").primaryKey().notNull(),
    /** The remote authority, absent for locally administered spaces. */
    serverConnectionId: identifier("server_connection_id", "server-connection").references(
        () => serverConnection.id,
        {
            onDelete: "restrict",
        },
    ),
    /** The last revision received from the authoritative service. */
    remoteRevision: integer("remote_revision"),
    /** The directory containing host-managed files, absent for remote-only access. */
    directory: text("directory").unique(),
    /** The last successful refresh of the remote registration. */
    synchronizedAt: integer("synchronized_at"),
}, (spaceRegistration) => [
    check(
        "space_registration_revision",
        sql`${spaceRegistration.remoteRevision} IS NULL OR (${spaceRegistration.serverConnectionId} IS NOT NULL AND ${spaceRegistration.remoteRevision} >= 1)`,
    ),
]);

/** A local space registration. */
export type SpaceRegistration = Select<typeof spaceRegistration>;
