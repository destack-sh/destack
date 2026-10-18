import { DeclarationReference } from "@destack/package/declare";
import { type DatabaseConnection, Log, Snapshot, type Select } from "@destack/db";
import { Scope, type Subject } from "@destack/sync";
import { ObjectClient } from "@destack/object/client";
import type { ClientOptions } from "@destack/runtime/client";
import type { Setting } from "../setting/setting.ts";
import type { SettingSelection } from "../setting/placement.ts";
import { defineObject, field } from "@destack/object";
import { schema, Version } from "@destack/schema";
import { PackageId, DeclarationName } from "@destack/package";
import { StackSetting } from "../declare/stack.ts";
import { SETTING_MODES } from "../setting/mode.ts";
import type { Clock } from "@destack/environment";

/** A setting value placed in a scope. */
export const setting = defineObject({
    name: "setting",
    plural: "settings",
    declarable: { schema: StackSetting },
    // hand every value down to the scopes inside its scope
    inherited: {},
    fields: {
        /** The package declaring the setting. */
        packageId: field.string(PackageId).immutable(),
        /** The setting's name in its declaring package. */
        name: field.string(DeclarationName).immutable(),

        // read the overrides from other databases as identifiers
        /** The consuming package the value applies to. */
        package: field.string(PackageId).optional().immutable(),
        /** The space the value applies in. */
        space: field.string(schema.identifier("space")).optional().immutable(),
        /** The installation the value applies to. */
        installation: field.string(schema.identifier("installation")).optional().immutable(),

        /** How the value applies. */
        mode: field.enum(SETTING_MODES),
        /** The value. */
        value: field.json(schema.json()),
        /** The release of the setting's package the value was written against. */
        release: field.string(Version),
    },
    permissions: ["read", "write"],
    adoptable: { by: "write" },
    constraints: {
        placement: {
            kind: "index",
            on: ["packageId", "name", "scope", "package", "space", "installation"],
            unique: true,
            nullsNotDistinct: true,
        },
        scope: { kind: "index", on: ["scope", "id"] },
        override: {
            kind: "check",
            condition: {
                AND: [
                    { OR: [{ installation: { isNull: true } }, { space: { isNull: true } }] },
                    {
                        OR: [
                            { mode: "set" },
                            { package: { isNull: true }, space: { isNull: true } },
                        ],
                    },
                ],
            },
        },
    },
});

/** A setting value as its row stores it. */
export type SettingValue = Select<typeof setting.table>;

/** Reads of the setting values placed in a database. */
export const SettingValue = {
    /** Resolve a setting for a selection from the values placed along its scope's chain. */
    async resolve<Value extends schema.Schema>(
        reader: Pick<Snapshot, "read">,
        declared: Setting<Value>,
        selection: SettingSelection & { readonly scope: string },
    ): Promise<schema.Infer<Value>> {
        // read the values the scope and the scopes above it set
        const chain = (await Scope.chain(reader, selection.scope)).map((link) => link.object.id);
        const values = await reader.read(setting.table).findMany({
            where: {
                AND: [declared.condition(selection), { scope: { in: chain } }],
            },
        });

        return declared.resolve(selection, values, chain).value;
    },

    /** Read the values a scope and the scopes above it place for some settings, nearest first. */
    async along(
        database: DatabaseConnection,
        settings: readonly DeclarationReference[],
        scope: string,
    ): Promise<{ readonly chain: string[]; readonly values: SettingValue[] }> {
        // read the values placed along the scope's chain
        const snapshot = Snapshot.live(database);
        const chain = (await Scope.chain(snapshot, scope)).map((link) => link.object.id);
        const values =
            settings.length === 0
                ? []
                : await snapshot.read(setting.table).findMany({
                      where: {
                          AND: [
                              { scope: { in: chain } },
                              {
                                  OR: settings.map((reference) => ({
                                      packageId: reference.packageId,
                                      name: reference.name,
                                  })),
                              },
                          ],
                      },
                  });

        return { chain, values };
    },

    /** Replicate a scope's values from their store into a database, yielding each change. */
    async *replicate(
        options: {
            /** The store serving the scope's setting values, such as the universe's. */
            readonly endpoint: ClientOptions;
            /** The scope whose values are replicated, such as a person. */
            readonly scope: string;
            /** The principal reading them. */
            readonly caller: Subject;
            /** The database with the replica, of `ObjectClient.tables({ setting })`. */
            readonly database: DatabaseConnection;
            /** The settings yielded. */
            readonly settings: readonly DeclarationReference[];
            /** The clock the replica runs at. */
            readonly clock: Clock;
        },
        signal: AbortSignal,
    ): AsyncGenerator<{ readonly chain: string[]; readonly values: SettingValue[] }> {
        // open a client of the scope's values over the database
        const { endpoint, scope, caller, database, clock } = options;
        const client = await ObjectClient.open({
            clock,
            database,
            objects: { setting },
            scope,
            caller,
            endpoint,
            reconnect: (machine) => {
                throw new TypeError(
                    `setting values of ${scope} never move, yet moved to ${machine}`,
                );
            },
        });

        // record the first failure, which stops following
        const failed = Promise.withResolvers<never>();
        failed.promise.catch(() => {});
        let failure: { readonly error: unknown } | undefined;
        const stopping = new AbortController();
        const following = AbortSignal.any([signal, stopping.signal]);

        // run the client and subscribe to the values
        const running = client.run(following, (error) => {
            failure = { error };
            failed.reject(error);
            stopping.abort();
        });
        const live = client.query.setting.findMany({}).subscribe();

        // yield once the values and the chain's access rows are replicated, and after each change
        try {
            await Promise.race([live.ready, failed.promise]);
            await Promise.race([
                database.log.until(
                    async () => (await client.access.position(database)) !== undefined,
                    following,
                ),
                failed.promise,
            ]);
            yield* SettingValue.follow(database, options.settings, scope, following);

            // fail with the replica's failure, ending quietly once stopped
            if (failure !== undefined) {
                throw failure.error;
            }
        } finally {
            stopping.abort();
            await live.close();
            await running;
            await client.close();
        }
    },

    /** Yield the values along a scope's chain for some settings, now and after each change. */
    async *follow(
        database: DatabaseConnection,
        settings: readonly DeclarationReference[],
        scope: string,
        signal: AbortSignal,
    ): AsyncGenerator<{ readonly chain: string[]; readonly values: SettingValue[] }> {
        // yield the values now from the log's position
        const log = new Log(database);
        const { sequence } = await log.position();
        const current = await SettingValue.along(database, settings, scope);
        yield current;

        // yield them again after each page changing setting values along the chain
        const pages = log.follow(
            { tables: [setting.table], after: sequence, scopes: current.chain },
            signal,
        );
        for await (const page of pages) {
            if (page.changes.length > 0) {
                yield await SettingValue.along(database, settings, scope);
            }
        }
    },
};
