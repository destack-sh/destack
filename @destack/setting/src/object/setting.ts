import { SettingError } from "../error/index.ts";
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
import { SETTING_MODES } from "../setting/mode.ts";
import type { Clock } from "@destack/environment";

/** A setting value placed in a scope. */
export const setting = defineObject({
    name: "setting",
    plural: "settings",
    // hand every value down to the scopes inside its scope
    inherited: {},
    // let a stack write a declared value against the release of the build declaring its setting
    supplied: ["release"],
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

/** The values a scope's chain places for some settings, nearest first, and a device's values. */
export interface SettingChain {
    /** The scope and the scopes above it, nearest first. */
    readonly chain: string[];
    /** The values the chain places. */
    readonly values: SettingValue[];
    /** The device's values, none without a device. */
    readonly device: DeviceSettingValue[];
}

/** Reads of the setting values placed in a database. */
export const SettingValue = {
    /** Read the retention a package's setting of days gives a scope. */
    retention(
        database: DatabaseConnection,
    ): (described: object, owner: Package, scope: string) => Promise<number> {
        return async (described, owner, scope) => {
            // rebuild the setting from the definition its kind's description stores, refusing anything but days
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

            // resolve it in a scope of its scope type, and give its default where none can be placed, such as a machine
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
        const chain = await SettingValue.chain(reader, selection.scope);
        const values = await reader.read(setting.table).findMany({
            where: {
                AND: [declared.condition(selection), { scope: { in: chain } }],
            },
        });

        return declared.resolve(selection, values, chain).value;
    },

    /** Read a scope and the scopes above it, nearest first, refusing a scope this store lacks. */
    async chain(reader: Pick<Snapshot, "read">, scope: string): Promise<string[]> {
        const chain = (await Scope.chain(reader, scope)).map((link) => link.object.id);
        if (chain.length === 0) {
            throw SettingError.of("UNKNOWN_SCOPE", `unknown scope ${scope}`);
        }

        return chain;
    },

    /** List the values a scope and the scopes above it place for some settings, nearest first. */
    async list(
        reader: Pick<Snapshot, "read">,
        settings: readonly DeclarationReference[],
        scope: string,
    ): Promise<Omit<SettingChain, "device">> {
        const chain = await SettingValue.chain(reader, scope);

        return { chain, values: await SettingValue.placed(reader, settings, chain) };
    },

    /** Read the values a chain of scopes places for some settings. */
    async placed(
        reader: Pick<Snapshot, "read">,
        settings: readonly DeclarationReference[],
        chain: readonly string[],
    ): Promise<SettingValue[]> {
        return settings.length === 0 || chain.length === 0
            ? []
            : await reader.read(setting.table).findMany({
                  where: {
                      AND: [
                          { scope: { in: [...chain] } },
                          {
                              OR: settings.map((reference) => ({
                                  packageId: reference.packageId,
                                  name: reference.name,
                              })),
                          },
                      ],
                  },
              });
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
    ): AsyncGenerator<SettingChain> {
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
                    `setting values of ${scope} cannot move, but moved to ${machine}`,
                );
            },
        });

        // record the first failure, which stops following
        const failed = Promise.withResolvers<never>();
        failed.promise.catch(() => undefined);
        const stopping = new AbortController();
        const following = AbortSignal.any([signal, stopping.signal]);

        // run the client and subscribe to the values
        const running = client.run(following, (error) => {
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

            // fail with the replica's failure, which alone stops following before the caller does
            if (stopping.signal.aborted) {
                await failed.promise;
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
    ): AsyncGenerator<SettingChain> {
        // read the values along the chain and the device's own
        const { scope, device } = at;
        const named = settings.map((reference) => ({
            packageId: reference.packageId,
            name: reference.name,
        }));
        const read = async (): Promise<SettingChain> => {
            // read the chain as it stands, none before its scope replicates, with its values and the device's values
            const snapshot = Snapshot.live(database);
            const chain = (await Scope.chain(snapshot, scope)).map((link) => link.object.id);
            const values = await SettingValue.placed(snapshot, settings, chain);
            const devices =
                device === undefined
                    ? []
                    : await snapshot
                          .read(deviceSetting.table)
                          .findMany({ where: { scope: device, OR: named } });

            return { chain, values, device: devices };
        };

        // yield them now from the log's position
        const log = new Log(database);
        let { sequence } = await log.position();
        let current = await read();
        yield current;

        // yield them after each page changing them, following the chain again once it changes
        const tables = [
            setting.table,
            Scope.table,
            ...(device === undefined ? [] : [deviceSetting.table]),
        ];
        for (;;) {
            const scopes = device === undefined ? current.chain : [...current.chain, device];
            const pages = log.follow({ tables, after: sequence, scopes }, signal);
            const changes = pages[Symbol.asyncIterator]();
            try {
                // read again after each page until the follow ends or the chain changes
                let page = await changes.next();
                while (page.done !== true) {
                    sequence = page.value.sequence;
                    const next = await read();
                    yield next;
                    const isMoved = next.chain.join(" ") !== current.chain.join(" ");
                    current = next;
                    if (isMoved) {
                        break;
                    }
                    page = await changes.next();
                }

                // end once the follow ends, else follow the new chain
                if (page.done === true) {
                    return;
                }
            } finally {
                // stop following once the caller stops reading or the chain changes
                await changes.return?.(undefined);
            }
        }
    },
};
