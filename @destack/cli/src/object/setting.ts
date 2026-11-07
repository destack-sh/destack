import { type JsonValue, present, schema, toJsonSchema, Identifier } from "@destack/schema";
import { DeviceValue, type Setting, SettingResolution, type SettingSource } from "@destack/setting";
import { deviceSetting, setting } from "@destack/setting/object";
import { type Verb, type Group, type Options, OptionValue } from "../command/command.ts";

import type { Context } from "../context/context.ts";
import type { Terminal } from "../output/index.ts";

import { listCommand } from "./command.ts";
import { LIST_OPTIONS } from "./flag.ts";
import type { DynamicObjectClient, ObjectClient } from "@destack/object/client";
import { Noun } from "./catalog.ts";
import { Render } from "./render.ts";
import { CliError } from "../error/index.ts";

/** The scopes of `--scope`. */
const SCOPES = ["account", "space", "device"] as const;

/** The option with the scope of a value. */
const SCOPE_OPTION = {
    scope: {
        type: "string",
        description: "Where the value is placed: account, space or device",
        argument: "scope",
        choices: SCOPES,
    },
} as const;

/** The columns a device's settings show. */
const DEVICE_COLUMNS = ["PACKAGE-ID", "NAME", "VALUE"];

/** A value as the commands read it at a scope. */
const Row = schema.looseObject({
    id: schema.string(),
    revision: schema.number(),
    value: schema.json(),
});

/** A value as the commands read it at a scope. */
type Row = schema.Infer<typeof Row>;

/** The values of one scope. */
interface SettingValues {
    /** Read the value of a setting at the scope, without overrides. */
    readonly current: (declared: Setting) => Promise<Row | undefined>;
    /** Set a value, replacing the one before. */
    readonly set: (declared: Setting, value: JsonValue, current: Row | undefined) => Promise<void>;
    /** Remove the value, which leaves the default. */
    readonly reset: (current: Row) => Promise<void>;
}

/** A scope of setting values: the person's account, the space, or this device. */
class SettingScope {
    /** The scope's name, as --scope takes it. */
    readonly name: (typeof SCOPES)[number];

    /** Take a scope by its name. */
    constructor(name: (typeof SCOPES)[number]) {
        this.name = name;
    }

    /** Read the scope of --scope, refusing a command line without one. */
    static read(options: Options): SettingScope {
        // require one of the scopes
        const named = OptionValue.string(options["scope"], "scope");
        const name = SCOPES.find((candidate) => candidate === named);
        if (name === undefined) {
            throw CliError.of("USAGE_INVALID", `set the scope with --scope ${SCOPES.join(", ")}`);
        }

        return new SettingScope(name);
    }

    /** Find the settings noun storing the scope's values: the universe's or the space's. */
    async noun(context: Context): Promise<Noun> {
        // refuse the device, whose values are its local objects
        if (this.name === "device") {
            throw CliError.of("USAGE_INVALID", "device settings are this computer's local objects");
        }

        // take the settings of the space's store for a space, else the universe's
        const store = this.name === "space" ? "space" : "universe";
        const found = (await context.catalog()).nouns.find(
            (noun) => noun.store === store && noun.type.isCopyOf(setting),
        );
        if (found === undefined) {
            throw CliError.of(
                "SPACE_REQUIRED",
                `${this.name} has no settings here: pick a space with --space`,
            );
        }

        return found;
    }

    /** Open the scope's values until a stack closes. */
    async open(context: Context, stack: AsyncDisposableStack): Promise<SettingValues> {
        // open the device's local values
        if (this.name === "device") {
            return DeviceSettingValues.open(context, stack);
        }

        // open the settings storing the scope's values
        const noun = await this.noun(context);

        return new StoreSettingValues(await noun.client(context));
    }
}

/** Build the commands of a setting by name. */
export function settingCommand(context: Context): Group {
    return {
        description: "Get, set or reset a setting at the scope --scope selects",
        commands: {},
        argument: {
            name: "name",
            complete: async () => (await context.settings()).map((declared) => declared.name),
            command: (name) => ({
                description: `Get, set or reset the setting ${name}`,
                commands: {
                    get: getCommand(context, name),
                    set: setCommand(context, name),
                    reset: resetCommand(context, name),
                },
                default: getCommand(context, name),
            }),
        },
    };
}

/** Build the command listing the values at the scope of --scope. */
export function settingsCommand(context: Context): Verb {
    return {
        description: "List the setting values at the scope --scope selects",
        json: true,
        options: { ...SCOPE_OPTION, ...LIST_OPTIONS },
        async run(call) {
            // read the scope and the list options beside it
            const scope = SettingScope.read(call.options);
            const options = Object.fromEntries(
                Object.entries(call.options).filter(([name]) => name !== "scope"),
            );
            const given = Object.keys(options);

            // refuse list options for the device's values, which take none
            if (scope.name === "device" && given.length > 0) {
                throw CliError.of(
                    "USAGE_INVALID",
                    `device settings do not accept --${given.join(", --")}`,
                );
            }
            // print the device's values
            else if (scope.name === "device") {
                await using stack = new AsyncDisposableStack();
                const values = await DeviceSettingValues.open(context, stack);
                await values.print(context, call.json);
            }
            // list the values of the settings storing the scope's
            else {
                await listCommand(context, await scope.noun(context)).run({
                    ...call,
                    options,
                });
            }
        },
    };
}

/** Build the command showing a setting's effective value, or its value at --scope. */
function getCommand(context: Context, name: string): Verb {
    return {
        description: `Show the value of ${name}, where it comes from, or its value at --scope`,
        options: SCOPE_OPTION,
        json: true,
        async run({ options, json }) {
            // show the effective value and its sources without --scope
            const declared = await context.setting(name);
            if (options["scope"] === undefined) {
                await showResolution(await resolve(context, declared), json, context.terminal);
            }
            // show the value at the scope, else the default
            else {
                const scope = SettingScope.read(options);
                await using stack = new AsyncDisposableStack();
                const values = await scope.open(context, stack);
                const current = await values.current(declared);
                await show(declared, scope, current, json, context.terminal);
            }
        },
    };
}

/** Resolve a setting's effective value, a device's on the device, a space's in the space. */
async function resolve(context: Context, declared: Setting): Promise<SettingResolution> {
    // read the device's values of the setting, which only the device stores
    await using stack = new AsyncDisposableStack();
    const settings = await context.objects({ deviceSetting }, stack);
    const rows = await settings.query.deviceSetting.findMany({ where: declared.reference });
    const device = rows.map(({ id, revision, packageId, name, value, release }) =>
        DeviceValue.parse({ id, revision, packageId, name, value, release }),
    );

    // resolve a device's setting here, from the device's values and the default
    const { scope } = declared.definition;
    const user = await context.me();
    if (scope === "device") {
        const id = (await context.native()).id;

        return SettingResolution.parse(declared.resolve({ scope: id }, [], [id], device));
    }
    // resolve a person's setting here for nobody signed in, from the device's values and the default
    else if (scope === "user" && user === undefined) {
        return SettingResolution.parse(declared.resolve({ scope: null }, [], [], device));
    }
    // resolve a person's setting in their account, and a space's in the space, beside the device's values
    else {
        const stored = new SettingScope(scope === "user" ? "account" : "space");
        const noun = await stored.noun(context);
        const remote = await noun.client(context);
        const resolved = await remote.call(setting.name, "resolve", {
            ...declared.reference,
            selection: {
                scope:
                    scope === "user"
                        ? present(user, "the person signed in")
                        : (await context.locate()).placement.space,
            },
            device,
        });

        return SettingResolution.parse(resolved);
    }
}

/** Print an effective value, and on a terminal the source deciding it and the ones it overrides. */
async function showResolution(
    resolution: SettingResolution,
    json: boolean,
    terminal: Terminal,
): Promise<void> {
    // print the resolution as JSON
    const { value, sources, overridden } = resolution;
    const text = typeof value === "string" ? value : JSON.stringify(value);
    if (json) {
        await terminal.print(JSON.stringify(resolution));
    }
    // print the value with where it comes from on a terminal
    else if (terminal.isTerminal) {
        const lines = [
            text,
            ...sources.map((source) => `  from  ${origin(source)}`),
            ...overridden.toReversed().map((source) => `  over  ${origin(source)}`),
        ];
        await terminal.print(lines.join("\n"));
    }
    // print the bare value for a program
    else {
        await terminal.print(text);
    }
}

/** Name where a setting's value comes from: the default, the device, or the scope placing it. */
function origin(source: SettingSource): string {
    // show the default
    if (source.kind === "default") {
        return "default";
    }
    // show the device
    else if (source.kind === "device") {
        return "device";
    }
    // show a value the declaration rejects by where it is placed
    else if (source.kind === "invalid") {
        return `${origin(source.source)} (invalid, skipped)`;
    }
    // show a placed value by its scope's identity and the overrides it applies to
    else {
        const kind = Identifier.prefix(schema.anyIdentifier().parse(source.scope));
        const overrides = [source.package, source.space, source.installation].filter(
            (override) => override !== undefined,
        );

        return [kind, ...overrides].join(" ");
    }
}

/** Build the command that sets a value at a scope. */
function setCommand(context: Context, name: string): Verb {
    return {
        description: `Set the value of ${name}`,
        arguments: [
            {
                name: "value",
                // offer the values an enumerated or boolean setting accepts
                complete: async () => offered(await context.setting(name)),
            },
        ],
        options: SCOPE_OPTION,
        json: true,
        async run({ arguments: { value: text }, options, json }) {
            // read the value by the declared schema
            const scope = SettingScope.read(options);
            const declared = await context.setting(name);
            const value = parse(declared, present(text, "the value argument"));

            // replace the value at the scope, or set the first there
            await using stack = new AsyncDisposableStack();
            const values = await scope.open(context, stack);
            await values.set(declared, value, await values.current(declared));
            await show(declared, scope, { value }, json, context.terminal);
        },
    };
}

/** Build the command removing the value at a scope, which leaves the default. */
function resetCommand(context: Context, name: string): Verb {
    return {
        description: `Reset ${name} to its default`,
        options: SCOPE_OPTION,
        json: true,
        async run({ options, json }) {
            // remove the value at the scope
            const scope = SettingScope.read(options);
            const declared = await context.setting(name);
            await using stack = new AsyncDisposableStack();
            const values = await scope.open(context, stack);
            const current = await values.current(declared);
            if (current !== undefined) {
                await values.reset(current);
            }
            await show(declared, scope, undefined, json, context.terminal);
        },
    };
}

/** List the values a setting offers on a command line: an enumeration's, or a boolean's. */
function offered(declared: Setting): string[] {
    // read the schema's enumeration or type as JSON Schema writes them
    const written = schema
        .looseObject({
            enum: schema.array(schema.json()).exactOptional(),
            type: schema.json().exactOptional(),
        })
        .parse(toJsonSchema(declared.definition.schema));
    const values = written.enum ?? (written.type === "boolean" ? [true, false] : []);

    return values.map((value) => (typeof value === "string" ? value : JSON.stringify(value)));
}

/** Read a value's text as JSON when the setting's schema accepts that, else as text. */
function parse(declared: Setting, text: string): JsonValue {
    // read the text as JSON, else take it as text
    let candidates: JsonValue[];
    try {
        candidates = [schema.json().parse(JSON.parse(text)), text];
    } catch {
        candidates = [text];
    }

    // take the first reading the schema accepts
    const accepted = candidates.find(
        (candidate) => declared.definition.schema.safeParse(candidate).success,
    );
    if (accepted === undefined) {
        throw CliError.of("USAGE_INVALID", `${declared.name} does not accept ${text}`);
    }

    return accepted;
}

/** Print a setting's value at a scope, the default when the scope has none. */
async function show(
    declared: Setting,
    scope: SettingScope,
    current: Pick<Row, "value"> | undefined,
    json: boolean,
    terminal: Terminal,
): Promise<void> {
    // show the placed value, else the default, else nothing for a setting without one
    const { definition } = declared;
    const isDefaulted = "default" in definition;
    const state = current !== undefined ? "set" : isDefaulted ? "default" : "unset";
    const shown = {
        package: declared.package.name,
        name: declared.name,
        scope: scope.name,
        value:
            current !== undefined
                ? current.value
                : isDefaulted
                  ? schema.json().parse(definition.default)
                  : null,
        state,
    };

    // print JSON, or the value as text with strings bare, marking a default and an unset setting
    const text = typeof shown.value === "string" ? shown.value : JSON.stringify(shown.value);
    const marks = { set: text, default: `${text} (default)`, unset: "(unset)" };
    await terminal.print(json ? JSON.stringify(shown) : marks[state]);
}

/** The values a store's settings noun stores at its scope, without overrides. */
class StoreSettingValues implements SettingValues {
    /** The client of the store's settings. */
    readonly #remote: DynamicObjectClient;

    /** Read and write a store's settings through a client. */
    constructor(remote: DynamicObjectClient) {
        this.#remote = remote;
    }

    /** Read the value of a setting at the scope, refusing two values of one setting. */
    async current(declared: Setting): Promise<Row | undefined> {
        // list the scope's rows
        const rows = await this.#remote.list(setting.name, {
            where: {
                ...declared.reference,
                package: { isNull: true },
                space: { isNull: true },
                installation: { isNull: true },
            },
            limit: 2,
        });

        // refuse two values of one setting
        const [only, other] = rows;
        if (other !== undefined) {
            throw CliError.of("SETTING_AMBIGUOUS", `${declared.name} has two values at one scope`);
        }

        return only === undefined ? undefined : Row.parse(only);
    }

    /** Set a value against the release declaring the setting, replacing the one before. */
    async set(declared: Setting, value: JsonValue, current: Row | undefined): Promise<void> {
        const written = { mode: "set", value, release: declared.package.version };
        await (current === undefined
            ? this.#remote.call(setting.name, "create", { ...declared.reference, ...written })
            : this.#remote.call(setting.name, "update", {
                  id: current.id,
                  revision: current.revision,
                  ...written,
              }));
    }

    /** Remove the value, which leaves the default. */
    async reset(current: Row): Promise<void> {
        await this.#remote.call(setting.name, "delete", {
            id: current.id,
            revision: current.revision,
        });
    }
}

/** The values this computer's device stores as its local objects. */
class DeviceSettingValues implements SettingValues {
    /** The device's local setting values. */
    readonly #settings: ObjectClient<{ readonly deviceSetting: typeof deviceSetting }>;

    /** Read and write the device's local values. */
    constructor(settings: ObjectClient<{ readonly deviceSetting: typeof deviceSetting }>) {
        this.#settings = settings;
    }

    /** Open the device's local values until a stack closes. */
    static async open(context: Context, stack: AsyncDisposableStack): Promise<DeviceSettingValues> {
        return new DeviceSettingValues(await context.objects({ deviceSetting }, stack));
    }

    /** Read the value of a setting on the device. */
    async current(declared: Setting): Promise<Row | undefined> {
        const [row] = await this.#settings.query.deviceSetting.findMany({
            where: declared.reference,
        });

        return row === undefined ? undefined : Row.parse(row);
    }

    /** Set a value against the setting's release, refusing a setting the device cannot store. */
    async set(declared: Setting, value: JsonValue, current: Row | undefined): Promise<void> {
        // require a setting the device stores
        if (!declared.isDeviceStored()) {
            throw CliError.of("USAGE_INVALID", `${declared.name} does not accept a device value`);
        }

        // write against the release declaring the setting
        const written = { value, release: declared.package.version };
        const submitted =
            current === undefined
                ? this.#settings.mutate(deviceSetting).create({ ...declared.reference, ...written })
                : this.#settings.mutate(deviceSetting).update({ id: current.id, ...written });
        await submitted.confirmed;
    }

    /** Remove the value, which leaves the default. */
    async reset(current: Row): Promise<void> {
        await this.#settings.mutate(deviceSetting).delete({ id: current.id }).confirmed;
    }

    /** Print every value the device stores, as JSON or a table. */
    async print(context: Context, json: boolean): Promise<void> {
        const rows = await this.#settings.query.deviceSetting.findMany();
        const cells = rows.map((row) => [row.packageId, row.name, JSON.stringify(row.value)]);
        await context.terminal.print(
            json ? JSON.stringify(rows) : Render.grid(DEVICE_COLUMNS, cells, context.terminal),
        );
    }
}
