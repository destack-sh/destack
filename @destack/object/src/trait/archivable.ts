import type { schema } from "@destack/schema";
import { integer } from "@destack/db";
import { ObjectError } from "../error/error.ts";
import type { MethodContext } from "../method/context.ts";
import { defineMethod, type Method } from "../method/method.ts";
import type {
    MethodProcedure,
    ObjectSchema,
    Procedure,
    ReplayFields,
    RowSchema,
    TargetFields,
} from "../method/procedure.ts";
import { Step } from "../method/step.ts";
import type { ObjectType } from "../object/object.ts";
import type { TraitTable } from "../object/table.ts";
import type { TraitBehavior, TraitPermission } from "./trait.ts";
import { callableTrait } from "./trait.ts";
import { t } from "@destack/locale";

/** An archivable object type's table, with its archive column. */
type ArchivableTable = TraitTable<{ readonly archivable: TraitPermission }>;

/** The signature of archiving or unarchiving an object. */
type ArchiveConfiguration<Permission extends string> = {
    kind: "archive" | "unarchive";
    permission: Permission;
    mutates: true;
};

/** The columns of an archivable object. */
export type ArchiveBuilderMap<Options> = Options extends undefined ? {} : ArchiveColumns;

/** The methods archivable objects take. */
export type ArchivableMethodMap<Archive> = [Archive] extends [string]
    ? {
          readonly archive: Method<ArchiveConfiguration<Archive> & { kind: "archive" }>;
          readonly unarchive: Method<ArchiveConfiguration<Archive> & { kind: "unarchive" }>;
      }
    : {};

/** The procedures of the archive and unarchive methods. */
export type ArchivableProcedures<Object extends ObjectType> = {
    archive: Procedure<
        schema.Object<TargetFields<Object> & ReplayFields<Object>>,
        RowSchema<Object>
    >;
    unarchive: Procedure<
        schema.Object<TargetFields<Object> & ReplayFields<Object>>,
        RowSchema<Object>
    >;
};

/** Objects archived out of everyday lists, kept read-only until unarchived. */
const archivableTrait: TraitBehavior<TraitPermission> = {
    key: "archivable",
    storages: ["durable"],
    options: (definition) => definition.archivable,
    columns: () => Archivable.columns(),
    constraints: () => [],
    policy: () => ({ attributes: { "archived-at": "number" } }),
    methods: (options) => ({
        archive: Archivable.method("archive", options.by),
        unarchive: Archivable.method("unarchive", options.by),
    }),
};

/** Archive objects out of everyday lists, read-only until unarchived. */
export const archivable = callableTrait("archivable", archivableTrait);

/** The columns, methods and changes between an object in use and an archived one. */
export const Archivable = {
    /** Build the column recording when an object was archived. */
    columns: archiveColumns,

    /** Declare the method archiving or unarchiving an object, undone by the other. */
    method<const Permission extends string>(
        kind: "archive" | "unarchive",
        permission: Permission,
    ): Method<ArchiveConfiguration<Permission>> {
        const inverse = kind === "archive" ? "unarchive" : "archive";

        return defineMethod<ArchiveConfiguration<Permission> & { table: ArchivableTable }>({
            title: kind === "archive" ? t`Archive` : t`Unarchive`,
            kind,
            permission,
            mutates: true,
            target: "object",
            isPredicted: true,
            result: "object",
            procedure: (_name: string, schemas: ObjectSchema): MethodProcedure => ({
                route: { method: "POST", path: `/{id}/${kind}` },
                input: schemas.target.extend(schemas.replay),
                output: schemas.row,
            }),
            handler: (context) => Archivable.change(context, kind),
            inverse: (step: Step) => {
                // archive or unarchive the object again
                const call = Step.call(step, inverse, Step.target(step));

                return call === undefined ? undefined : [call];
            },
        });
    },

    /** Archive or unarchive a call's target, refusing one already in that state. */
    async change(
        context: MethodContext<{ readonly table: ArchivableTable }, unknown>,
        kind: "archive" | "unarchive",
    ) {
        // refuse archiving an archived object, and unarchiving one in use
        const { object } = context;
        const isArchived = context.target.archivedAt !== null;
        if (kind === "archive" && isArchived) {
            throw ObjectError.of(
                "ARCHIVE_BLOCKED",
                `cannot archive ${object.name}: it is already archived`,
            );
        } else if (kind === "unarchive" && !isArchived) {
            throw ObjectError.of(
                "ARCHIVE_BLOCKED",
                `cannot unarchive ${object.name}: it is not archived`,
            );
        }

        // record the time it was archived, or clear it
        return context.update({ archivedAt: kind === "archive" ? context.now : null });
    },
};

/** Build the column recording when an object was archived. */
function archiveColumns() {
    return {
        /** The time the object was archived, null while in use. */
        archivedAt: integer("archived_at"),
    };
}

/** The column of an archivable object. */
export type ArchiveColumns = ReturnType<typeof archiveColumns>;
