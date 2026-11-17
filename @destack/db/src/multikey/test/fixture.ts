import { schema } from "@destack/schema";
import { defineTable, index, json, text } from "../../index.ts";

/** Playlists indexed by their tracks' titles, which triggers copy into an entry table. */
export const playlist = defineTable(
    "playlist",
    {
        /** The playlist's identifier. */
        id: text("id").primaryKey(),
        /** The playlist's tracks in order. */
        tracks: json("tracks", schema.array(schema.object({ title: schema.string() }))).notNull(),
    },
    {
        constraints: (columns) => [
            index("playlist_track").on(columns.tracks.element("title", "text")),
        ],
    },
);
