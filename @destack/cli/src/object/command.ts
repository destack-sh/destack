import { Condition, TABLE } from "@destack/db";
import { connectBunSqlite } from "@destack/db/bun";
import { ObjectClient, type ObjectConnection } from "@destack/object/client";
import {
    CalendarLayout,
    ObjectAppearance,
    ObjectView,
    ObjectViewState,
    type ObjectQuery,
    type TimelineLayout,
    type TreeLayout,
} from "@destack/object";
import { principal } from "@destack/access";
import { Scope } from "@destack/sync";
import { type JsonObject, type JsonValue, present, schema } from "@destack/schema";
import { type Verb, type Group, OptionValue, type Options } from "../command/command.ts";
import { Interrupt } from "../command/error.ts";
import type { Context } from "../context/context.ts";
import type { Live, Terminal } from "../output/index.ts";

import { FlagSet, INPUT_OPTION, LIST_OPTIONS } from "./flag.ts";
import { Noun, Catalog } from "./catalog.ts";
import { Localization, SOURCE_LOCALE } from "@destack/locale";
import { Command } from "@destack/object";
import { Render } from "./render.ts";
import { LayoutRender } from "./layout.ts";
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
export function nounCommand(
    context: Context,
    catalog: Catalog,
    word: string,
): Verb | Group | undefined {
    // find the noun, refusing a word several nouns claim
    const noun = catalog.find(word);

    // leave a word no noun claims to the other commands
    if (noun === undefined) {
        return undefined;
    }
    // list a noun's objects by its plural, taking the singular's subcommands too, as kubectl's plurals do
    else if (noun.lists(word)) {
        const list = listCommand(context, noun);

        return { ...objectCommand(context, noun), description: list.description, default: list };
    }
    // get, create and call a noun's objects by its singular
    else {
        return objectCommand(context, noun);
    }
}

/** Build the command that lists a noun's rows, once or live. */
export function listCommand(context: Context, noun: Noun): Verb {
    return {
        description: `List ${noun.plural} (${noun.qualifier})`,
        json: true,
        get options() {
            return { ...FlagSet.rows(noun.type).options(), ...LIST_OPTIONS };
        },
        async run(call) {
            // read the keys the flags give as identifiers
            const { json } = call;
            const flags = FlagSet.rows(noun.type);
            const options = await identifyFlags(context, flags, call.options);

            // build the condition, the order and the limit
            const filter = flags.filter(
                await context.me(),
                context.environment.clock.now(),
                context.environment.timeZone,
            );
            const where = flags.condition(options, filter);
            const orderBy = Render.order(OptionValue.string(options["sort"], "sort"));
            const limit = OptionValue.count(options["limit"], "limit");
            const query = {
                ...(where === undefined ? {} : { where: Condition.schema.parse(where) }),
                ...(orderBy === undefined ? {} : { orderBy }),
                ...(limit === undefined ? {} : { limit }),
            };
            // pick the view the flags ask for, extending the query to read what it shows
            const view = chosenView(noun, OptionValue.string(options["view"], "view"));
            const { layout } = view;
            const columns = Render.columns(
                noun.type,
                layout.kind === "table" ? layout : { kind: "table" },
                OptionValue.string(options["fields"], "fields"),
            );
            const window =
                layout.kind === "calendar"
                    ? CalendarLayout.month(
                          context.environment.clock.now(),
                          context.environment.timeZone,
                      )
                    : undefined;
            const laidOut = ObjectView.query(view, query, window);
            const render = async (rows: readonly JsonObject[]) =>
                json
                    ? JSON.stringify(rows)
                    : layout.kind === "table"
                      ? Render.table(
                            await titled(context, noun, rows),
                            columns,
                            noun.type,
                            context.terminal,
                        )
                      : LayoutRender.render(rows, layout, noun.type, context.terminal, window);

            // print the rows after every change until interrupted
            const remote = await noun.client(context);
            if (options["watch"] === true) {
                const live = context.terminal.live(json);
                using interrupted = new Interrupt(context.environment.process);
                const followed = watch(noun, remote, laidOut, interrupted.signal, context);

                // page a tree or a timeline under the keys on a terminal, else redraw each change
                const { terminal } = context;
                const isPaged =
                    (layout.kind === "tree" || layout.kind === "timeline") &&
                    terminal.isTerminal &&
                    terminal.process.stdin.isTerminal &&
                    !json;
                if (isPaged) {
                    await page(followed, layout, noun, live, terminal);
                } else {
                    for await (const rows of followed) {
                        await live.show(await render(rows));
                    }
                }
                interrupted.require();
            }
            // print the rows once, nothing for a program when there are none
            else {
                const rendered = await render(await remote.list(noun.type, laidOut));
                if (rendered !== "") {
                    await context.terminal.print(rendered);
                }
            }
        },
    };
}

/** Page followed rows as a tree or a timeline under a terminal's keys, until quit or their end. */
async function page(
    followed: AsyncGenerator<readonly JsonObject[]>,
    layout: TreeLayout | TimelineLayout,
    noun: Noun,
    live: Live,
    terminal: Terminal,
): Promise<void> {
    // read the rows and the keys at once, the pager's state kept across changes
    const keys = terminal.keys();
    let rows: readonly JsonObject[] = [];
    let state = ObjectViewState.start();
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

            // take new rows, end on quit or the end of either, else answer the key
            if (event.kind === "rows") {
                if (event.read.done === true) {
                    return;
                }
                rows = event.read.value;
                nextRows = followed.next();
            } else if (event.read.done === true || event.read.value === "quit") {
                return;
            } else {
                const drawn = LayoutRender.page(rows, layout, noun.type, terminal);
                state = ObjectViewState.press(
                    state,
                    event.read.value,
                    drawn.lines,
                    LayoutRender.room(drawn, height),
                );
                nextKey = keys.next();
            }

            // draw the page under the pager
            const drawn = LayoutRender.page(rows, layout, noun.type, terminal);
            await live.show(LayoutRender.paged(drawn, state, height, terminal));
        }
    } finally {
        await keys.return(undefined);
        await followed.return(undefined);
    }
}

/** Pick the view a list asks for by name, else the type's default. */
function chosenView(noun: Noun, name: string | undefined): ObjectView {
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

/** Write each reference in rows as its object's title, for a person on a terminal. */
async function titled(
    context: Context,
    noun: Noun,
    rows: readonly JsonObject[],
): Promise<readonly JsonObject[]> {
    // leave a program's identifiers as they are
    if (!context.terminal.isTerminal) {
        return rows;
    }

    // read the titles of each reference field's objects through their type's noun
    const catalog = await context.catalog();
    const titles = new Map<string, Map<string, string>>();
    for (const [column, field] of Object.entries(noun.type.fields)) {
        const target = field.target?.();
        const referenced = catalog.nouns.find(
            (each) => target !== undefined && each.type.same(target),
        );
        const ids = [
            ...new Set(
                rows.flatMap((row) => (typeof row[column] === "string" ? [row[column]] : [])),
            ),
        ];
        if (referenced !== undefined && ids.length > 0) {
            const remote = await referenced.client(context);
            const objects = await remote.list(referenced.type, { where: { id: { in: ids } } });
            titles.set(
                column,
                new Map(
                    objects.map((row) => [
                        schema.string().parse(row["id"]),
                        ObjectAppearance.of(referenced.type, row).title,
                    ]),
                ),
            );
        }
    }

    // replace each referenced identifier by its title
    return rows.map((row) =>
        Object.fromEntries(
            Object.entries(row).map(([column, value]) => [
                column,
                (typeof value === "string" ? titles.get(column)?.get(value) : undefined) ??
                    value ??
                    null,
            ]),
        ),
    );
}

/** Build the commands of one noun's objects: create, and per object its get and its methods. */
function objectCommand(context: Context, noun: Noun): Group {
    // call the methods of the scope without an object, such as the noun's create
    const methods = Object.entries(noun.type.describeMethods()).filter(
        ([, method]) => !NOUN_KINDS.has(method.kind),
    );
    const creates = methods.filter(([, method]) => method.target === "scope");

    // call the type's create method by the word new too, unless a method or command has the word
    const created = creates.find(([, method]) => method.kind === "create");
    const isTaken =
        creates.some(([name]) => name === NEW) ||
        noun.commands.some((command) => command.target === "scope" && command.name === NEW);
    const added: [string, Verb][] =
        created === undefined || isTaken
            ? []
            : [[NEW, methodCommand(context, noun, created[0], undefined)]];
    const called = methods.filter(([, method]) => method.target === "object");

    return {
        description: `Get, create and change ${noun.plural} (${noun.qualifier})`,
        commands: Object.fromEntries([
            // list a method named create under the word new alone, taking both words
            ...creates.map(([name]): [string, Verb] => [
                name,
                {
                    ...methodCommand(context, noun, name, undefined),
                    ...(name === CREATE && added.length > 0 ? { isHidden: true } : {}),
                },
            ]),
            ...added,
            ...noun.commands
                .filter((command) => command.target === "scope")
                .map((command): [string, Verb] => [
                    command.name,
                    declaredCommand(context, noun, command, undefined),
                ]),
        ]),
        // call the methods of one object, getting it without a method
        argument: {
            name: "id",
            // offer the identifiers of the first page of objects
            complete: async () => {
                const rows = await (await noun.client(context)).list(noun.type, { limit: ID_PAGE });

                return rows.map((row) => schema.string().parse(row["id"]));
            },
            command: (id) => ({
                description: `Get ${id}, or call one of its methods or commands`,
                commands: Object.fromEntries([
                    ...called.map(([name]): [string, Verb] => [
                        name,
                        methodCommand(context, noun, name, id),
                    ]),
                    ...noun.commands
                        .filter((command) => command.target === "object")
                        .map((command): [string, Verb] => [
                            command.name,
                            declaredCommand(context, noun, command, id),
                        ]),
                ]),
                default: getCommand(context, noun, id),
            }),
        },
    };
}

/** Build the command getting one object, once or live. */
function getCommand(context: Context, noun: Noun, id: string): Verb {
    return {
        description: `Show one of the ${noun.plural}`,
        json: true,
        options: { watch: LIST_OPTIONS.watch },
        async run({ options, json }) {
            // open the object's noun, showing it as fields or JSON
            const remote = await noun.client(context);
            const show = async (row: JsonObject) =>
                json
                    ? JSON.stringify(row)
                    : Render.object(
                          schema.jsonObject().parse((await titled(context, noun, [row]))[0]),
                          noun.type,
                          context.terminal,
                      );

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
function methodCommand(context: Context, noun: Noun, method: string, id: string | undefined): Verb {
    return {
        description: `Call ${noun.name}.${method}`,
        json: true,
        get options() {
            return {
                ...FlagSet.input(noun.type, method).options(),
                ...INPUT_OPTION,
                ...(Command.isDestructive(noun.type, method) ? YES_OPTION : {}),
            };
        },
        async run(call) {
            // ask before a method that removes its target for good
            const { json, options } = call;
            if (Command.isDestructive(noun.type, method)) {
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
    noun: Noun,
    id: string | undefined,
    options: Readonly<Record<string, unknown>>,
    context: Context,
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

/** Build the command running a declared command on a matching object, or on its scope. */
function declaredCommand(
    context: Context,
    noun: Noun,
    command: Command,
    id: string | undefined,
): Verb {
    const fields = Object.keys(command.preset);

    return {
        description: SOURCE.render(command.title),
        json: true,
        get options() {
            return {
                ...FlagSet.input(noun.type, command.method).without(fields).options(),
                ...INPUT_OPTION,
                ...(command.isDestructive ? YES_OPTION : {}),
            };
        },
        async run(call) {
            // refuse an object the command does not apply to, or which the person may not call it on
            const remote = await noun.client(context);
            const target = id === undefined ? undefined : await noun.identify(id, context);
            if (target !== undefined) {
                const [applies] = await remote.list(noun.type, {
                    ...command.query([target]),
                    limit: 1,
                });
                if (applies === undefined) {
                    throw CliError.of(
                        "COMMAND_BLOCKED",
                        `cannot run '${command.name}' on ${noun.name} '${id}': it does not apply`,
                    );
                }
            }

            // ask before a command that removes its target for good
            if (command.isDestructive) {
                await requireConfirmation(command.name, noun, id, call.options, context);
            }

            // build the input from the JSON option, the flags and the object, beside the preset fields
            const flags = FlagSet.input(noun.type, command.method).without(fields);
            const input = await methodInput(flags, call.options, context);

            // call the method and show its result
            const result = await remote.run(
                noun.type,
                command.method,
                command.input(target === undefined ? input : { ...input, id: target }),
            );
            await context.terminal.print(
                call.json
                    ? JSON.stringify(result)
                    : await renderResult(context, noun, command.method, result),
            );
        },
    };
}

/** Read the keys given to flags of identifiers as the identifiers of their objects. */
async function identifyFlags(context: Context, flags: FlagSet, options: Options): Promise<Options> {
    return flags.identify(options, async (identity, value) => {
        // find the noun listing objects of the identity
        const noun = (await context.catalog()).identity(identity);
        if (noun === undefined) {
            throw CliError.of(
                "USAGE_INVALID",
                `'${value}' is invalid: nothing lists '${identity}'s here, so give its identifier`,
            );
        }

        return noun.identify(value, context);
    });
}

/** Read a method's input: the JSON of --input, with the flags written over it. */
async function methodInput(
    flags: FlagSet,
    options: Options,
    context: Context,
): Promise<JsonObject> {
    const filter = flags.filter(
        await context.me(),
        context.environment.clock.now(),
        context.environment.timeZone,
    );
    const given = await OptionValue.json(
        options["input"],
        "input",
        context.environment,
        context.terminal,
    );

    return flags.input(options, given, filter);
}

/** Render a method's result: a changed object as one event, any other result as it is. */
async function renderResult(
    context: Context,
    noun: Noun,
    method: string,
    result: JsonValue,
): Promise<string> {
    // show what a method without a changed object returned
    const row = schema.jsonObject().safeParse(result);
    const word = RESULT_WORDS[noun.type.method(method).kind];
    if (!row.success) {
        return JSON.stringify(result);
    } else if (word === undefined) {
        const [shown] = await titled(context, noun, [row.data]);

        return Render.object(schema.jsonObject().parse(shown), noun.type, context.terminal);
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
    noun: Noun,
    remote: ObjectConnection,
    query: Pick<ObjectQuery, "where" | "orderBy" | "limit">,
    signal: AbortSignal,
    context: Context,
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
