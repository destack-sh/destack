import { through, union } from "@destack/access";
import { sql, uniqueIndex, type Select } from "@destack/db";
import { defineObject, field, method } from "@destack/object";
import { DeclarationName, PackageId } from "@destack/package";
import { CONCURRENCIES, ScheduleTiming } from "@destack/service/schedule";
import { Call } from "@destack/sync";
import { installation } from "./installation.ts";
import { space } from "./space.ts";

/** A schedule of an installation: a timing and the object method call each occurrence runs, declared by its package or created by it. */
export const schedule = defineObject({
    name: "schedule",
    plural: "schedules",
    scope: space,
    fields: {
        /** The installation whose calls run. */
        installation: field.reference(installation, { delete: "cascade" }),
        /** The package declaring the schedule, absent for one the installation created. */
        packageId: field.string(PackageId).optional(),
        /** The schedule's name within its installation. */
        name: field.string(DeclarationName),

        // what runs when
        /** When the occurrences fall. */
        timing: field.json(ScheduleTiming),
        /** The object method call each occurrence runs, in the installation's space. */
        call: field.json(Call),
        /** Whether occurrences may overlap. */
        concurrency: field.enum(CONCURRENCIES),
        /** How late an occurrence may start, in milliseconds. */
        deadline: field.integer(),
        /** Whether occurrences pause. */
        isPaused: field.boolean().default(false),
    },
    constraints: (entry) => [
        // name declared schedules apart from the ones the installation created
        uniqueIndex("schedule_name").on(
            entry.installation,
            sql`coalesce(${entry.packageId}, '')`,
            entry.name,
        ),
    ],
    permissions: {
        read: through("installation", "read"),
        write: through("installation", "run"),
        pause: union(through("installation", "run"), through("installation", "update")),
    },
    reserved: ["write"],
    methods: {
        get: method.get("read"),
        list: method.list("read"),
        create: method.create("write", {
            fields: ["installation", "name", "timing", "call", "concurrency", "deadline"],
        }),
        update: method.update("write", { fields: ["timing", "call"] }),
        pause: method.update("pause", { fields: ["isPaused"] }),
        delete: method.delete("write"),
        declare: method.create(null, { isSystem: true }),
        redeclare: method.update(null, { isSystem: true }),
        retire: method.delete(null, { isSystem: true }),
    },
});

/** A schedule of an installation. */
export type Schedule = Select<typeof schedule.table>;
