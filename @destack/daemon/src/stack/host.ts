import { check, identifier, integer, json, type Select, sql, table, text } from "@destack/db";
import { HostEnrollment } from "../service/host.ts";

/** Host identity and optional device enrollment. */
export const host = table(
    "host",
    {
        /** Singleton row key. */
        id: integer("id").primaryKey().notNull(),
        /** Device identifier registered during sign-in. */
        deviceId: identifier("device_id", "device").unique(),
        /** Host identifier registered during sign-in. */
        hostId: identifier("host_id", "host").notNull().unique(),
        /** Verified universe enrollment, absent before registration. */
        enrollment: json("enrollment", HostEnrollment),
        /** Secure OS credential reference for the enrolled host. */
        credential: text("credential"),
        /** Persisted execution admission, independent of daemon process lifetime. */
        execution: text("execution", { enum: ["enabled", "draining", "disabled"] })
            .notNull()
            .default("disabled"),
        /** Local display name. */
        name: text("name").notNull(),
        /** Installation creation time in UTC milliseconds. */
        createdAt: integer("created_at").notNull(),
    },
    (host) => [check("host_singleton", sql`${host.id} = 1`)],
);

/** The local daemon's persistent host configuration. */
export type Host = Select<typeof host>;
