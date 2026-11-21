import { Condition, TABLE } from "@destack/db";
import { connectBunSqlite } from "@destack/db/bun";
import { ObjectClient, type ObjectConnection } from "@destack/object/client";
import {
    CalendarLayout,
    type ObjectType,
    type Layout,
    Presentation,
    ObjectView,
    type ObjectQuery,
} from "@destack/object";
import { principal } from "@destack/access";
import { Scope } from "@destack/sync";
import { type JsonObject, type JsonValue, present, schema, type TimeScale } from "@destack/schema";
import {
    type Command,
    type CommandBody,
    CommandOption,
    type CommandCall,
} from "../command/command.ts";
import { Interrupt } from "../command/failure.ts";
import type { CommandContext } from "../command/context.ts";
import type { Live } from "../output/index.ts";

import { FlagSet, INPUT_OPTION, LIST_OPTIONS } from "../object/flag.ts";
import { CatalogType, Catalog } from "../object/catalog.ts";
import { Localization, SOURCE_LOCALE } from "@destack/locale";
import { type Action, ActionCall } from "@destack/object";
import { Frame } from "../object/frame.ts";
import { Pager } from "../object/pager.ts";
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

/** Build the command of a word that names a noun. */
export function nounCommand(catalog: Catalog, word: string): Command | undefined {
    // find the noun, refusing a word several nouns claim
    const entry = catalog.find(word);

    // leave a word no noun claims to the other commands
    if (entry === undefined) {
        return undefined;
    }
    // list a noun's objects by its plural, taking the singular's subcommands too, as kubectl's plurals do
    else if (entry.lists(word)) {
        return {
            description: `List ${entry.plural} (${entry.qualifier})`,
            load: async () => {
                const noun = await entry.open();

                return { ...objectCommand(noun), ...listCommand(noun) };
            },
        };
    }
    // get, create and call a noun's objects by its singular
    else {
        return {
            description: `Get, create and change ${entry.plural} (${entry.qualifier})`,
            load: async () => objectCommand(await entry.open()),
        };
    }
}

/** Build the command that lists a noun's rows, once or live. */
export function listCommand(noun: CatalogType): CommandBody {
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
                    titled(rows, type, pager.filter),
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
                        await ran(step.action, step.targets, step.input, remote, frame),
                    );
                }
            }

            // draw the page under the pager
            const drawn = frame.page(
                titled(rows, type, pager.filter),
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

/** Filter rows to those whose title contains a text, ignoring case. */
function titled(rows: readonly JsonObject[], type: ObjectType, filter: string): JsonObject[] {
    return rows.filter((row) =>
        Presentation.title(type.presentation, row).toLowerCase().includes(filter.toLowerCase()),
    );
}

/** Run an action's call on each target, telling what it did or why it failed. */
async function ran(
    action: Action,
    targets: readonly JsonObject[],
    input: JsonObject,
    remote: ObjectConnection,
    frame: Frame,
): Promise<string> {
    // call the method once per target, stopping at the first refusal
    const title = frame.terminal.localization.render(action.title);
    const call = present(action.call, `the call of action '${action.name}'`);
    try {
        for (const target of targets) {
            await remote.run(
                call.object,
                call.method,
                call.input({ ...input, id: target["id"] ?? null }),
            );
        }
    } catch (error) {
        return `${title} failed: ${error instanceof Error ? error.message : String(error)}`;
    }

    return `${title}: ${targets.length} ${targets.length === 1 ? call.object.name : call.object.plural}`;
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

    // call the type's create method by the word new too, unless a method or an action has the word
    const created = creates.find(([, method]) => method.kind === "create");
    const isTaken =
        creates.some(([name]) => name === NEW) ||
        noun.actions.some((action) => action.call?.target === "scope" && action.name === NEW);
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
            ...noun.actions
                .filter((action) => action.call?.target === "scope")
                .map((action): [string, Command] => [
                    action.name,
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
                    ...called.map(([name]): [string, Command] => [
                        name,
                        methodCommand(noun, name, id),
                    ]),
                    ...noun.actions
                        .filter((action) => action.call?.target === "object")
                        .map((action): [string, Command] => [
                            action.name,
                            actionCommand(noun, action, id),
                        ]),
                ]),
            }),
        },
    };
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
                ...(ActionCall.isDestructive(noun.type, method) ? YES_OPTION : {}),
            };
        },
        async run(call, context) {
            // ask before a method that removes its target for good
            const { isJson: json, options } = call;
            if (ActionCall.isDestructive(noun.type, method)) {
                await requireConfirmation(method, noun, id, options, context);
            }

            // read the flags, whose references the method resolves by key
            const flags = FlagSet.input(noun.type, method);

            // build the input from the JSON option, the flags and the object
            const written = await methodInput(flags, options, context);
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
    const fields = Object.keys(call.preset);

    return {
        description: SOURCE.render(action.title),
        isJson: true,
        get options() {
            return {
                ...FlagSet.input(noun.type, call.method).without(fields).options(),
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
                        `cannot run '${action.name}' on ${noun.name} '${id}': it does not apply`,
                    );
                }
            }

            // ask before an action that removes its target for good
            if (call.isDestructive) {
                await requireConfirmation(action.name, noun, id, given.options, context);
            }

            // build the input from the JSON option, the flags and the object, beside the preset fields
            const flags = FlagSet.input(noun.type, call.method).without(fields);
            const input = await methodInput(flags, given.options, context);

            // call the method and show its result
            const result = await remote.run(
                noun.type,
                call.method,
                call.input(target === undefined ? input : { ...input, id: target }),
            );
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

        return (await noun.open()).identify(value, context);
    });
}

/** Read a method's input: the JSON of --input, with the flags written over it. */
async function methodInput(
    flags: FlagSet,
    options: CommandCall["options"],
    context: CommandContext,
): Promise<JsonObject> {
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

    return flags.input(options, given, filter);
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
    const title = noun.type.presentation?.title;
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
    const objects = ObjectClient.keyed([noun.type]);
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
