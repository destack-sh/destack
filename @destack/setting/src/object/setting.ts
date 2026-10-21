import { DeclarationReference } from "@destack/package/declare";
import { type DatabaseConnection, Log, Snapshot, type Select } from "@destack/db";
import { deviceSetting, type DeviceSettingValue } from "./device-setting.ts";
import { Scope, type Subject } from "@destack/sync";
import { ObjectClient } from "@destack/object/client";
import type { ClientOptions } from "@destack/object/client";
import { Setting, SettingMetadata, SettingScope } from "../setting/setting.ts";
import type { SettingSelection } from "../setting/placement.ts";
import { defineObject, field } from "@destack/object";
import { Duration, type Identifier, schema, Version } from "@destack/schema";
import { DeclarationName, type Package, PackageId } from "@destack/package";
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

/** The values a scope's chain places for some settings, nearest first, and a device's own. */
export interface SettingPlacement {
    /** The scope and the scopes above it, nearest first. */
    readonly chain: string[];
    /** The values the chain places. */
    readonly values: SettingValue[];
    /** The device's own values, none without a device. */
    readonly device: DeviceSettingValue[];
}

/** Reads of the setting values placed in a database. */
export const SettingValue = {
    /** Read the retention a package's setting of days gives a scope. */
    retention(
        database: DatabaseConnection,
    ): (setting: object, owner: Package, scope: string) => Promise<number> {
        return async (described, owner, scope) => {
            // rebuild the package's setting from its definition, refusing anything but a setting of days
            const definition = schema.record(schema.string(), schema.unknown()).parse(described);
            const {
                schema: value,
                default: fallback,
                scope: kind,
                overrides,
                ...metadata
            } = definition;
            if (!(value instanceof schema.Schema)) {
                throw new TypeError(
                    `setting ${String(metadata["name"])} of ${owner.name} has no value schema`,
                );
            }
            const declared = new Setting(owner, {
                ...SettingMetadata.parse(metadata),
                ...SettingScope.parse({ scope: kind, overrides }),
                schema: value,
                default: schema.number().int().positive().parse(fallback),
            });

            // resolve it in a scope of its scope type, and take its default in any other
            const isApplied = schema.identifier(declared.definition.scope).safeParse(scope).success;
            const days = isApplied
                ? await SettingValue.resolve(Snapshot.live(database), declared, { scope })
                : declared.definition.default;

            return Duration.milliseconds({ days: schema.number().int().positive().parse(days) });
        };
    },

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
            yield* SettingValue.follow(database, options.settings, { scope }, following);

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

    /** Yield settings' values along a scope's chain and on a device, now and after each change. */
    async *follow(
        database: DatabaseConnection,
        settings: readonly DeclarationReference[],
        at: {
            /** The scope whose chain places values. */
            readonly scope: string;
            /** The device whose own values come first, absent for none. */
            readonly device?: Identifier<"device">;
        },
        signal: AbortSignal,
    ): AsyncGenerator<SettingPlacement> {
        // read the values along the chain and the device's own
        const { scope, device } = at;
        const named = settings.map((reference) => ({
            packageId: reference.packageId,
            name: reference.name,
        }));
        const read = async (): Promise<SettingPlacement> => ({
            ...(await SettingValue.along(database, settings, scope)),
            device:
                device === undefined
                    ? []
                    : await Snapshot.live(database)
                          .read(deviceSetting.table)
                          .findMany({ where: { scope: device, OR: named } }),
        });

        // yield them now from the log's position, then after each page changing them
        const log = new Log(database);
        const { sequence } = await log.position();
        const current = await read();
        yield current;
        const pages = log.follow(
            {
                tables:
                    device === undefined ? [setting.table] : [setting.table, deviceSetting.table],
                after: sequence,
                scopes: device === undefined ? current.chain : [...current.chain, device],
            },
            signal,
        );
        const changes = pages[Symbol.asyncIterator]();
        try {
            while ((await changes.next()).done !== true) {
                yield await read();
            }
        } finally {
            // stop following once the caller stops reading
            await changes.return?.(undefined);
        }
    },
};
