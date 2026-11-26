import { Condition, TABLE } from "@destack/db";
import { connectBunSqlite } from "@destack/db/bun";
import { type ObjectConnection } from "@destack/object/client";
import { ObjectClient } from "@destack/object/device";
import {
    CalendarLayout,
    type Layout,
    ObjectType,
    ObjectView,
    type ObjectQuery,
} from "@destack/object";
import { principal } from "@destack/access";
import { Scope } from "@destack/sync";
import { type JsonObject, JsonValue, nth, present, schema, type TimeScale } from "@destack/schema";
import {
    type Command,
    type CommandBody,
    CommandOption,
    type CommandCall,
} from "../command/command.ts";
import { Interrupt } from "../command/failure.ts";
import type { CommandContext } from "../command/context.ts";
import type { Live } from "../terminal/index.ts";

import { type FileFlag, FlagSet, INPUT_OPTION, LIST_OPTIONS } from "./flag.ts";
import type { CatalogEntry, Store } from "./catalog.ts";
import {
    Casing,
    type MethodKind,
    type ObjectFile,
    type PackageImplementationDescription,
} from "@destack/object";
import { Localization, SOURCE_LOCALE } from "@destack/locale";
import type { Action, ActionCall, ActionContext } from "@destack/object";
import { Frame } from "../terminal/frame.ts";
import { Pager } from "../terminal/pager.ts";
import { type Environment, FileSystemError, Pipe } from "@destack/environment";
import { MediaType } from "@destack/package/file";
import { basename } from "node:path";
import { CliError } from "../error/index.ts";

/** The lines a terminal is taken to have when it reports none: 24, the VT100's. */
const DEFAULT_ROWS = 24;
/** The word that calls a noun's create method beside the method's own name. */
const NEW = "new";

/** The usual name of a create method, which the word new stands for in help. */
const CREATE = "create";

/** The word for what a method of a kind did to its object, absent for kinds returning data. */
const RESULT_WORDS: Readonly<Record<string, string | undefined>> = {
    create: "Created",
    update: "Updated",
    delete: "Deleted",
    purge: "Purged",
    restore: "Restored",
    archive: "Archived",
    unarchive: "Unarchived",
    move: "Moved",
    revert: "Reverted",
    edit: "Edited",
};

/** The word that downloads an object's file. */
const DOWNLOAD = "download";

/** The ending of a file being saved, until it is whole. */
const PARTIAL = ".partial";

/** The option confirming an action that cannot be undone. */
const YES_OPTION = {
    yes: { type: "boolean", description: "Run without asking" },
} as const;

/** The method kinds the singular noun's commands run: getting one object and listing them. */
const NOUN_KINDS = new Set(["get", "list"]);

/** The language the command line writes titles in, the one messages are written in. */
const SOURCE = Localization.of(SOURCE_LOCALE, []);

/** The most identifiers completion offers for a noun, one terminal screen of candidates. */
const ID_PAGE = 50;

/** A listed object type opened: its entry with the object type and the actions calling it. */
export class CatalogType {
    /** The entry the type was opened from. */
    readonly entry: CatalogEntry;
    /** The object type. */
    readonly type: ObjectType;
    /** The actions the installation declares calling the type's methods. */
    readonly actions: readonly Action[];

    /** Keep a listed entry with its type and its actions. */
    constructor(entry: CatalogEntry, type: ObjectType, actions: readonly Action[]) {
        this.entry = entry;
        this.type = type;
        this.actions = actions;
    }

    /** Read the singular word. */
    get name(): string {
        return this.entry.name;
    }

    /** Read the plural word, which lists. */
    get plural(): string {
        return this.entry.plural;
    }

    /** Read the qualifier. */
    get qualifier(): string {
        return this.entry.qualifier;
    }

    /** The store running the installation. */
    get store(): Store {
        return this.entry.store;
    }

    /** The package listing the type. */
    get package(): PackageImplementationDescription {
        return this.entry.package;
    }

    /** Open a client of the type's objects in the scope the terminal uses for them. */
    client(context: CommandContext): Promise<ObjectConnection> {
        return this.entry.client(context);
    }

    /** Build the commands of its objects: create, and per object get, methods and actions. */
    command(): CommandBody {
        return objectCommand(this);
    }

    /** Build the command that lists its rows, once or live. */
    list(): CommandBody {
        return listCommand(this);
    }

    /** Find the name of the type's method of a method kind, refusing a type without one. */
    method(kind: MethodKind): string {
        // find the one method of the kind, refusing none and several
        const methods = Object.entries(this.type.describeMethods());
        const [found, other] = methods.filter(([, method]) => method.kind === kind);
        if (found === undefined) {
            throw CliError.of("METHOD_NOT_FOUND", `${kind} method of '${this.plural}' not found`);
        } else if (other !== undefined) {
            throw CliError.of(
                "METHOD_INVALID",
                `${kind} method of '${this.plural}' is invalid: several match`,
            );
        }

        return found[0];
    }

    /** Refuse a key the type cannot find: one that is no identifier of a type without keys. */
    requireFindable(key: string): void {
        if (!this.type.identifies(key) && this.type.keyed === undefined) {
            throw CliError.of(
                "USAGE_INVALID",
                `identifier '${key}' is invalid: '${this.plural}' have no key, so give an identifier, or list them with '${this.plural}'`,
            );
        }
    }

    /** Find an object's identifier by its identifier or key, refusing keys of unkeyed types. */
    async identify(key: string, context: CommandContext): Promise<string> {
        // take an identifier as given, and require a declared key for anything else
        this.requireFindable(key);
        if (this.type.identifies(key)) {
            return key;
        }

        // get the object by its key, which its get method resolves
        const remote = await this.client(context);
        const row = await remote.run(this.type, this.method("get"), { id: key });

        return schema.looseObject({ id: schema.string() }).parse(row).id;
    }
}

/** Build the command that lists a noun's rows, once or live. */
function listCommand(noun: CatalogType): CommandBody {
    return {
        isJson: true,
        get options() {
            return { ...FlagSet.rows(noun.type).options(), ...LIST_OPTIONS };
        },
        async run(call, context) {
            // read the keys the flags give as identifiers
            const { isJson: json } = call;
            const flags = FlagSet.rows(noun.type);
            const options = await identifyFlags(context, flags, call.options);

            // build the condition, the order and the limit
            const filter = flags.filter(
                await context.me(),
                context.environment.clock.now(),
                context.environment.timeZone,
            );
            const where = flags.condition(options, filter);
            const orderBy = flags.order(options);
            const limit = CommandOption.count(options["limit"], "limit");
            const query = {
                ...(where === undefined ? {} : { where: Condition.schema.parse(where) }),
                ...(orderBy === undefined ? {} : { orderBy }),
                ...(limit === undefined ? {} : { limit }),
            };

            // shape a view: a table's columns as --fields names them, its query and its window bound to now
            const named = flags.columns(options);
            const shape = (view: ObjectView): Shaped => {
                // draw a table by the named columns, else the view's layout
                const layout =
                    view.layout.kind === "table" && named !== undefined
                        ? { kind: "table" as const, fields: named }
                        : view.layout;
                const now = context.environment.clock.now();
                const { timeZone } = context.environment;
                const window =
                    layout.kind === "calendar" ? CalendarLayout.month(now, timeZone) : undefined;
                const laidOut = ObjectView.query(
                    { ...view, layout },
                    query,
                    { now, timeZone },
                    window,
                );

                return { view, layout, window, query: laidOut };
            };
            const shaped = shape(chosenView(noun, CommandOption.string(options["view"], "view")));
            const draw = async (rows: readonly JsonObject[]) =>
                json
                    ? JSON.stringify(rows)
                    : (await Frame.of(context, noun, rows)).draw(
                          rows,
                          shaped.layout,
                          noun.type,
                          shaped.window,
                      );

            // page the rows under the keys on a terminal, else print them after every change
            const remote = await noun.client(context);
            const { terminal } = context;
            if (options["watch"] === true) {
                const live = terminal.live(json);
                using interrupted = new Interrupt(context.environment.process);
                if (terminal.isTerminal && terminal.process.stdin.isTerminal && !json) {
                    await page({
                        shaped,
                        shape,
                        noun,
                        remote,
                        live,
                        context,
                        signal: interrupted.signal,
                    });
                } else {
                    for await (const rows of watch(
                        noun,
                        remote,
                        shaped.query,
                        interrupted.signal,
                        context,
                    )) {
                        await live.show(await draw(rows));
                    }
                }
                interrupted.require();
            }
            // print the rows once, nothing for a program when there are none
            else {
                const drawn = await draw(await remote.list(noun.type, shaped.query));
                if (drawn !== "") {
                    await terminal.print(drawn);
                }
            }
        },
    };
}

/** A view shaped for a list: its layout, its calendar window and the query reading its rows. */
interface Shaped {
    /** The view. */
    readonly view: ObjectView;
    /** The layout drawn, a table's columns as --fields names them. */
    readonly layout: Layout;
    /** The month a calendar shows, absent for other layouts. */
    readonly window: TimeScale | undefined;
    /** The query reading the view's rows. */
    readonly query: Pick<ObjectQuery, "where" | "orderBy" | "limit">;
}

/** Page followed rows under a terminal's keys: move, filter, switch views and run actions. */
async function page(paging: {
    readonly shaped: Shaped;
    readonly shape: (view: ObjectView) => Shaped;
    readonly noun: CatalogType;
    readonly remote: ObjectConnection;
    readonly live: Live;
    readonly context: CommandContext;
    readonly signal: AbortSignal;
}): Promise<void> {
    // follow the view's rows and read the keys at once
    const { shape, noun, remote, live, context, signal } = paging;
    const { terminal } = context;
    const { type } = noun;
    const keys = terminal.keys();
    let shaped = paging.shaped;
    let followed = watch(noun, remote, shaped.query, signal, context);
    let rows: readonly JsonObject[] = [];
    let frame = await Frame.of(context, noun, rows);
    let pager = Pager.start(type, noun.actions);
    let nextRows = followed.next();
    let nextKey = keys.next();
    try {
        for (;;) {
            // wait for the next rows or key
            const event = await Promise.race([
                nextRows.then((read) => ({ kind: "rows", read }) as const),
                nextKey.then((read) => ({ kind: "key", read }) as const),
            ]);
            const height = terminal.rows ?? DEFAULT_ROWS;

            // take new rows and their frame, ending with them
            if (event.kind === "rows") {
                if (event.read.done === true) {
                    return;
                }
                rows = event.read.value;
                frame = await Frame.of(context, noun, rows);
                nextRows = followed.next();
            }
            // end with the keys, else answer the key
            else if (event.read.done === true) {
                return;
            } else {
                nextKey = keys.next();
                const drawn = frame.page(
                    frame.titled(rows, type, pager.filter),
                    shaped.layout,
                    type,
                    shaped.window,
                );
                const answered = pager.press(event.read.value, drawn, shaped.view, frame, height);
                pager = answered.pager;
                const { step } = answered;

                // quit, follow another view's rows, or run a command on its targets
                if (step.kind === "quit") {
                    return;
                } else if (step.kind === "view") {
                    await followed.return(undefined);
                    shaped = shape(step.view);
                    followed = watch(noun, remote, shaped.query, signal, context);
                    nextRows = followed.next();
                } else if (step.kind === "run") {
                    pager = pager.told(
                        await ran(step.action, step.targets, step.input, remote, context),
                    );
                }
            }

            // draw the page under the pager
            const drawn = frame.page(
                frame.titled(rows, type, pager.filter),
                shaped.layout,
                type,
                shaped.window,
            );
            await live.show(pager.draw(drawn, frame, height));
        }
    } finally {
        await keys.return(undefined);
        await followed.return(undefined);
    }
}

/** Run an action's call on each target, telling what it did or why it failed. */
async function ran(
    action: Action,
    targets: readonly JsonObject[],
    input: JsonObject,
    remote: ObjectConnection,
    context: CommandContext,
): Promise<string> {
    // call the method once per target, stopping at the first refusal
    const title = context.terminal.localization.render(action.title);
    const acting = await contextOf(context, remote);
    const call = present(action.call, `the call of action '${action.name}'`);
    try {
        for (const target of targets) {
            const id = schema.string().parse(target["id"]);
            await invoke(call, remote, { ...input, ...targetOf(call, remote, id) }, acting);
        }
    } catch (error) {
        return `${title} failed: ${error instanceof Error ? error.message : String(error)}`;
    }

    return `${title}: ${targets.length} ${targets.length === 1 ? call.object.name : call.object.plural}`;
}

/** Read the scope a connection calls in, the universe without one. */
function scopeOf(remote: ObjectConnection): string {
    return remote.scope ?? Scope.universe.id;
}

/** Read the value of the object an action's call acts on, by its identifier. */
function targetOf(call: ActionCall, remote: ObjectConnection, id: string): JsonObject {
    return call.target === undefined
        ? {}
        : { [call.target]: call.object.policy.reference(scopeOf(remote), id) };
}

/** Run an action's call on values, reading the rows it needs, and answer its last result. */
async function invoke(
    call: ActionCall,
    remote: ObjectConnection,
    values: JsonObject,
    acting: ActionContext,
): Promise<JsonValue> {
    // require the revision given, and take each object and person by identifier as its reference
    const { revision } = values;
    const required = revision === undefined ? {} : { revision };
    const referred = Object.fromEntries(
        call.inputs.map(({ name, value: takes }) => {
            const value = values[name];
            if (typeof value === "string" && takes.kind === "person") {
                return [name, principal.user.reference(Scope.universe.id, value)];
            } else if (typeof value === "string" && takes.kind === "object") {
                return [name, takes.type.policy.reference(scopeOf(remote), value)];
            }

            return [name, value];
        }),
    );

    // call the method for each target, as the person, reading the rows it needs
    const invocations = await call.invocations(
        { values: JsonValue.object(referred) },
        {
            context: acting,
            read: async (type, { id }) => {
                const [row] = await remote.list(type, { where: { id: { in: [id] } }, limit: 1 });

                return row;
            },
        },
    );
    let result: JsonValue = null;
    for (const { input } of invocations) {
        result = await remote.run(call.object, call.method, { ...input, ...required });
    }

    return result;
}

/** Read what a command's action reads: the person signed in on this machine, its scope, now. */
async function contextOf(
    context: CommandContext,
    remote: ObjectConnection,
): Promise<ActionContext> {
    const userId = await context.me();

    return {
        subject:
            userId === undefined ? undefined : principal.user.reference(Scope.universe.id, userId),
        moment: {
            now: context.environment.clock.now(),
            timeZone: context.terminal.timeZone,
        },
        scope: scopeOf(remote),
        places: [],
    };
}

/** Pick the view a list asks for by name, else the type's default. */
function chosenView(noun: CatalogType, name: string | undefined): ObjectView {
    // take the view by its name, else the default
    const offered = noun.type.views;
    const chosen = name === undefined ? noun.type.view : offered.find((view) => view.name === name);

    // refuse a view the type does not offer, listing those it does
    if (chosen === undefined) {
        const names = offered.map((view) => view.name).join(", ");
        throw CliError.of(
            "USAGE_INVALID",
            `view '${name}' is invalid: '${noun.plural}' offer '${names}'`,
        );
    }

    return chosen;
}

/** Build the commands of a noun's objects: create, and per object get, methods and actions. */
function objectCommand(noun: CatalogType): CommandBody {
    // call the methods of the scope without an object, such as the noun's create
    const methods = Object.entries(noun.type.describeMethods()).filter(
        ([, method]) => !NOUN_KINDS.has(method.kind),
    );
    const creates = methods.filter(([, method]) => method.target === "scope");

    // split the actions by whether they call on an object or on the scope
    const onObject = noun.actions.filter(({ call }) => call?.target !== undefined);
    const onScope = noun.actions.filter(
        ({ call }) => call !== undefined && call.target === undefined,
    );

    // call the type's create method by the word new too, unless a method or an action has the word
    const created = creates.find(([, method]) => method.kind === "create");
    const isTaken =
        creates.some(([name]) => name === NEW) || onScope.some((action) => action.verb === NEW);
    const added: [string, Command][] =
        created === undefined || isTaken ? [] : [[NEW, methodCommand(noun, created[0], undefined)]];
    const called = methods.filter(([, method]) => method.target === "object");

    return {
        commands: Object.fromEntries([
            // list a method named create under the word new alone, taking both words
            ...creates.map(([name]): [string, Command] => [
                name,
                {
                    ...methodCommand(noun, name, undefined),
                    ...(name === CREATE && added.length > 0 ? { isHidden: true } : {}),
                },
            ]),
            ...added,
            ...onScope.map((action): [string, Command] => [
                action.verb,
                actionCommand(noun, action, undefined),
            ]),
        ]),
        // call the methods and actions of one object, getting it without either
        target: {
            name: "id",
            // offer the identifiers of the first page of objects
            complete: async (context) => {
                const rows = await (await noun.client(context)).list(noun.type, { limit: ID_PAGE });

                return rows.map((row) => schema.string().parse(row["id"]));
            },
            command: (id) => ({
                ...getCommand(noun, id),
                commands: Object.fromEntries([
                    ...(fileFields(noun.type).length === 0
                        ? []
                        : [[DOWNLOAD, downloadCommand(noun, id)] satisfies [string, Command]]),
                    ...called.map(([name]): [string, Command] => [
                        name,
                        methodCommand(noun, name, id),
                    ]),
                    ...onObject.map((action): [string, Command] => [
                        action.verb,
                        actionCommand(noun, action, id),
                    ]),
                ]),
            }),
        },
    };
}

/** List a type's file fields. */
function fileFields(type: ObjectType): string[] {
    return Object.entries(type.fields)
        .filter(([, field]) => field.limits !== undefined)
        .map(([name]) => name);
}

/** Build the command downloading an object's file to this machine. */
function downloadCommand(noun: CatalogType, id: string): Command {
    // take a field by its flag's name, the only one when there is one
    const fields = new Map(fileFields(noun.type).map((name) => [Casing.kebab(name), name]));
    const names = [...fields.keys()];
    const [only] = names;

    return {
        description: "Download a file",
        isJson: true,
        arguments: [
            {
                name: "field",
                usage: names.join("|"),
                ...(only === undefined || names.length > 1 ? {} : { default: only }),
                complete: async () => names,
            },
        ],
        options: {
            output: {
                type: "string",
                description: "The path to save it to, its own name here by default",
                argument: "path",
            },
        },
        async run({ arguments: given, options, isJson: json }, context) {
            // read the object, refusing a field without a file
            const named = given["field"] ?? "";
            const field = fields.get(named);
            if (field === undefined) {
                throw CliError.of(
                    "USAGE_INVALID",
                    `file '${named}' is invalid: '${noun.name}' has '${names.join(", ")}'`,
                );
            }
            const remote = await noun.client(context);
            const identified = await noun.identify(id, context);
            const row = schema
                .jsonObject()
                .parse(await remote.run(noun.type, noun.method("get"), { id: identified }));
            const found = noun.type.fields[field]?.file(row[field]);
            if (found === undefined) {
                throw CliError.of(
                    "USAGE_INVALID",
                    `${noun.name} '${id}' is invalid: its '${named}' has no file`,
                );
            }

            // save it under the given path, else its own name
            const { file } = found;
            const output = CommandOption.string(options["output"], "output");
            const path = output ?? file.name;
            const client = await remote.files();
            const reference = noun.type.reference(schema.string().parse(row["scope"]), identified);
            const body = await client.open(reference, field);
            await saveFile(context, path, body, output === undefined);
            await context.terminal.print(
                json
                    ? JSON.stringify({ path, size: file.size, type: file.type })
                    : context.terminal.event("Saved", "done", path),
            );
        },
    };
}

/** Save a file's bytes as they arrive, replacing a file only at a path the person wrote. */
async function saveFile(
    { environment: { files } }: CommandContext,
    path: string,
    body: ReadableStream<Uint8Array>,
    isNew: boolean,
): Promise<void> {
    // refuse to replace a file that another person's name picked
    if (isNew && (await files.exists(path))) {
        await body.cancel();
        throw CliError.of(
            "USAGE_INVALID",
            `file '${path}' is invalid: it exists already, so give another --output`,
        );
    }

    // write it beside its place, and put it there once it is whole
    const partial = `${path}${PARTIAL}`;
    try {
        await using file = await files.open(partial, { write: true, createNew: true });
        await Pipe.to(body, file.writable());
    } catch (error) {
        await files.remove(partial, { force: true });
        throw error;
    }
    await files.rename(partial, path);
}

/** Build the command getting one object, once or live. */
function getCommand(noun: CatalogType, id: string): Command {
    return {
        description: `Show one of the ${noun.plural}`,
        isJson: true,
        options: { watch: LIST_OPTIONS.watch },
        async run({ options, isJson: json }, context) {
            // open the object's noun, showing it as fields or JSON
            const remote = await noun.client(context);
            const show = async (row: JsonObject) =>
                json
                    ? JSON.stringify(row)
                    : (await Frame.of(context, noun, [row])).record(row, noun.type);

            // follow the object as it changes until interrupted
            if (options["watch"] === true) {
                const live = context.terminal.live(json);
                const query = { where: { id: await noun.identify(id, context) } };
                using interrupted = new Interrupt(context.environment.process);
                for await (const rows of watch(noun, remote, query, interrupted.signal, context)) {
                    const [row] = rows;
                    await live.show(row === undefined ? `No ${noun.name} ${id}.` : await show(row));
                }
                interrupted.require();
            }
            // read it once through the noun's get method
            else {
                noun.requireFindable(id);
                const row = await remote.run(noun.type, noun.method("get"), { id });
                await context.terminal.print(await show(schema.jsonObject().parse(row)));
            }
        },
    };
}

/** Build the command that calls one method, on an object or on the collection. */
function methodCommand(noun: CatalogType, method: string, id: string | undefined): Command {
    return {
        description: `Call ${noun.name}.${method}`,
        isJson: true,
        get options() {
            return {
                ...FlagSet.input(noun.type, method).options(),
                ...INPUT_OPTION,
                ...(noun.type.isDestructive(method) ? YES_OPTION : {}),
            };
        },
        async run(call, context) {
            // ask before a method that removes its target for good
            const { isJson: json, options } = call;
            if (noun.type.isDestructive(method)) {
                await requireConfirmation(method, noun, id, options, context);
            }

            // read the flags, whose references the method resolves by key
            const flags = FlagSet.input(noun.type, method);

            // build the input from the JSON option, the flags and the object
            const written = await methodInput(noun, flags, options, context);
            const input = id === undefined ? written : { ...written, id };

            // call the method and show its result
            const remote = await noun.client(context);
            const result = await remote.run(noun.type, method, input);
            await context.terminal.print(
                json ? JSON.stringify(result) : await renderResult(context, noun, method, result),
            );
        },
    };
}

/** Ask a person on a terminal before a verb that cannot be undone, unless they passed --yes. */
async function requireConfirmation(
    verb: string,
    noun: CatalogType,
    id: string | undefined,
    options: Readonly<Record<string, unknown>>,
    context: CommandContext,
): Promise<void> {
    // take --yes, else the person's answer on a terminal
    const target = id ?? noun.plural;
    const word = `${verb.charAt(0).toUpperCase()}${verb.slice(1)}`;
    const subject = id === undefined ? `'${noun.plural}'` : `${noun.name} '${id}'`;
    const isConfirmed =
        options["yes"] === true ||
        (context.environment.process.stdin.isTerminal &&
            (await context.terminal.prompt.confirm(`${word} ${subject}? This cannot be undone.`)));
    if (!isConfirmed) {
        throw CliError.of(
            "COMMAND_BLOCKED",
            `cannot run '${verb}' on '${target}': it cannot be undone, so confirm it with --yes`,
        );
    }
}

/** Build the command running a declared action's call on a matching object, or on its scope. */
function actionCommand(noun: CatalogType, action: Action, id: string | undefined): Command {
    const call = present(action.call, `the call of action '${action.name}'`);
    const flags = () => FlagSet.action(noun.type, call.inputs, call.target);

    return {
        description: SOURCE.render(action.title),
        isJson: true,
        get options() {
            return {
                ...flags().options(),
                ...INPUT_OPTION,
                ...(call.isDestructive ? YES_OPTION : {}),
            };
        },
        async run(given, context) {
            // refuse an object the action does not apply to, or which the person may not call it on
            const remote = await noun.client(context);
            const target = id === undefined ? undefined : await noun.identify(id, context);
            if (target !== undefined) {
                const [applies] = await remote.list(noun.type, {
                    ...call.query([target]),
                    limit: 1,
                });
                if (applies === undefined) {
                    throw CliError.of(
                        "COMMAND_BLOCKED",
                        `cannot run '${action.verb}' on ${noun.name} '${id}': it does not apply`,
                    );
                }
            }

            // ask before an action that removes its target for good
            if (call.isDestructive) {
                await requireConfirmation(action.verb, noun, id, given.options, context);
            }

            // fill the inputs from the JSON option, the flags and the object, refusing a missing one
            const values = {
                ...(await methodInput(noun, flags(), given.options, context)),
                ...(target === undefined ? {} : targetOf(call, remote, target)),
            };
            const missing = call.inputs.find(
                (each) => each.presence === "required" && !(each.name in values),
            );
            if (missing !== undefined) {
                throw CliError.of(
                    "USAGE_INVALID",
                    `input '${missing.name}' of action '${action.verb}' is invalid: it must be given`,
                );
            }

            // call the method and show its result
            const result = await invoke(call, remote, values, await contextOf(context, remote));
            await context.terminal.print(
                given.isJson
                    ? JSON.stringify(result)
                    : await renderResult(context, noun, call.method, result),
            );
        },
    };
}

/** Read the keys given to flags of identifiers as the identifiers of their objects. */
async function identifyFlags(
    context: CommandContext,
    flags: FlagSet,
    options: CommandCall["options"],
): Promise<CommandCall["options"]> {
    return flags.identify(options, async (identity, value) => {
        // find the noun listing objects of the identity
        const noun = (await context.catalog()).identity(identity);
        if (noun === undefined) {
            throw CliError.of(
                "USAGE_INVALID",
                `'${value}' is invalid: nothing lists '${identity}'s here, so give its identifier`,
            );
        }

        return (await noun.open(context)).identify(value, context);
    });
}

/** Read a method's input: the JSON of --input, with the flags written over it. */
async function methodInput(
    noun: CatalogType,
    flags: FlagSet,
    options: CommandCall["options"],
    context: CommandContext,
): Promise<JsonObject> {
    // read the JSON, with the flags as the person writes them
    const filter = flags.filter(
        await context.me(),
        context.environment.clock.now(),
        context.environment.timeZone,
    );
    const given = await CommandOption.json(
        options["input"],
        "input",
        context.environment,
        context.terminal,
    );

    // send the files the flags give paths of, over the input
    const sent: Record<string, JsonValue> = {};
    for (const { flag, path } of flags.files(options)) {
        sent[nth(flag.path, 0)] =
            path === undefined ? null : await sendFile(noun, flag, path, context);
    }

    // refuse a key of a listed type without keys, leaving the server to resolve the others
    const checked = await flags.identify(options, async (identity, value) => {
        const listed = (await context.catalog()).identity(identity);
        await listed?.open(context).then((found) => found.requireFindable(value));

        return value;
    });

    // address the people the subject flags give
    const people = await flags.address(checked, (named) => context.subject(named));

    return { ...flags.input(checked, given, filter), ...people, ...sent };
}

/** Send a file at a path to a type's store, refusing a path without one. */
async function sendFile(
    noun: CatalogType,
    flag: FileFlag,
    path: string,
    context: CommandContext,
): Promise<ObjectFile> {
    // find it and its size
    const { files } = context.environment;
    const status = await files.stat(path).catch((error: unknown) => {
        if (FileSystemError.is(error, "FILE_NOT_FOUND")) {
            return undefined;
        }
        throw error;
    });
    if (status?.kind !== "file") {
        throw CliError.of(
            "USAGE_INVALID",
            status === undefined
                ? `--'${flag.name}' file '${path}' is invalid: it does not exist`
                : `--'${flag.name}' file '${path}' is invalid: it is not a file`,
        );
    }

    // refuse one its field does not take
    const { size } = status;
    const type = MediaType.of(path);
    flag.require(path, type, size);

    // send each part as it is read, under the file's own name
    const slice = (start: number, end: number) => ({
        bytes: () => readRange(context.environment, path, start, end),
    });
    const remote = await noun.client(context);
    const client = await remote.files();

    return client.upload({ size, slice }, { name: basename(path), type }).done;
}

/** Read the bytes of a file from a start to an end, refusing a file that became shorter. */
async function readRange(
    { files }: Environment<"files">,
    path: string,
    start: number,
    end: number,
): Promise<Uint8Array<ArrayBuffer>> {
    // read until the range is whole
    await using file = await files.open(path, { read: true });
    await file.seek(start);
    const bytes = new Uint8Array(end - start);
    for (let read = 0; read < bytes.byteLength;) {
        const count = await file.read(bytes.subarray(read));
        if (count === null) {
            throw CliError.of("USAGE_INVALID", `file '${path}' is invalid: it became shorter`);
        }
        read += count;
    }

    return bytes;
}

/** Render a method's result: a changed object as one event, any other result as it is. */
async function renderResult(
    context: CommandContext,
    noun: CatalogType,
    method: string,
    result: JsonValue,
): Promise<string> {
    // show what a method without a changed object returned
    const row = schema.jsonObject().safeParse(result);
    const word = RESULT_WORDS[noun.type.method(method).kind];
    if (!row.success) {
        return JSON.stringify(result);
    } else if (word === undefined) {
        const frame = await Frame.of(context, noun, [row.data]);

        return frame.record(row.data, noun.type);
    }

    // say what happened to the object, by its title, with its identifier beside it
    const { id } = schema.looseObject({ id: schema.string().exactOptional() }).parse(row.data);
    const title = noun.type.presentation.title;
    const named = title === undefined ? undefined : row.data[title];
    const subject = typeof named === "string" ? `${noun.name} '${named}'` : noun.name;

    return context.terminal.event(word, "done", subject, id);
}

/** Follow a query's rows for the person through an in-memory replica until the signal aborts. */
async function* watch(
    noun: CatalogType,
    remote: ObjectConnection,
    query: Pick<ObjectQuery, "where" | "orderBy" | "limit">,
    signal: AbortSignal,
    context: CommandContext,
): AsyncGenerator<readonly JsonObject[]> {
    // replicate the type's rows in memory as the person signed in
    const { environment } = await context.native();
    const objects = ObjectType.keyed([noun.type]);
    const database = await connectBunSqlite(":memory:", ObjectClient.tables(objects), {
        environment,
    });
    const { userId } = (await context.launcher()).user;
    const client = await ObjectClient.open({
        database,
        objects,
        scope: present(remote.scope, `the scope of ${noun.plural}`),
        caller: principal.user.reference(Scope.universe.id, userId),
        remote,
        environment,
    });

    // follow the replica, ending the watch at its first refused follow
    const stopping = new AbortController();
    const stopped = AbortSignal.any([signal, stopping.signal]);
    let failure: Error | undefined;
    const stopRefusals = client.store.listenRefusals((error) => {
        failure ??= error instanceof Error ? error : new Error(String(error));
        stopping.abort();
    });
    const following = client.follow(stopped);
    try {
        // yield the rows as JSON once the replica has them, and after every change
        const { table } = noun.type;
        for await (const rows of client.queryType(noun.type).findMany(query).watch(stopped)) {
            yield rows.map((row) => table[TABLE].encode(row));
        }
        if (failure !== undefined) {
            throw failure;
        }
    } finally {
        stopRefusals();
        stopping.abort();
        await following;
        await client.close();
        await database.close();
    }
}
