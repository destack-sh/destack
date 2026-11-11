import { Condition } from "@destack/db";
import { connectBunSqlite } from "@destack/db/bun";
import type { DynamicObjectClient } from "@destack/object/client";
import { type JsonObject, type JsonValue, schema } from "@destack/schema";
import { type Verb, type Group, OptionValue, type Options } from "../command/command.ts";
import { Interrupt } from "../command/error.ts";
import type { CliEnvironment, Context } from "../context/context.ts";
import type { Terminal } from "../output/index.ts";

import { FlagSet, INPUT_OPTION, LIST_OPTIONS } from "./flag.ts";
import { Noun, Catalog } from "./catalog.ts";
import { Localization, SOURCE_LOCALE } from "@destack/locale";
import type { Command } from "@destack/object";
import { Render } from "./render.ts";
import { CliError } from "../error/index.ts";

/** The method kinds the singular noun's commands run: getting one object and listing them. */
const NOUN_KINDS = new Set(["get", "list"]);

/** The method kinds called without an object. */
const COLLECTION_KINDS = new Set(["create"]);

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
    // list a noun's objects by its plural
    else if (noun.lists(word)) {
        return listCommand(context, noun);
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
            const columns = Render.columns(
                noun.type,
                OptionValue.string(options["fields"], "fields"),
            );
            const render = (rows: readonly JsonObject[]) =>
                json
                    ? JSON.stringify(rows)
                    : Render.table(rows, columns, noun.type, context.terminal);

            // print the rows after every change until interrupted
            const remote = await noun.client(context);
            if (options["watch"] === true) {
                const live = context.terminal.live(json);
                using interrupted = new Interrupt(context.environment.process);
                for await (const rows of watch(
                    remote,
                    noun.type.name,
                    query,
                    interrupted.signal,
                    context.environment,
                )) {
                    await live.show(render(rows));
                }
                interrupted.require();
            }
            // print the rows once, nothing for a program when there are none
            else {
                const rendered = render(await remote.list(noun.type.name, query));
                if (rendered !== "") {
                    await context.terminal.print(rendered);
                }
            }
        },
    };
}

/** Build the commands of one noun's objects: create, and per object its get and its methods. */
function objectCommand(context: Context, noun: Noun): Group {
    // create without an object, as the noun's create method takes
    const methods = Object.entries(noun.type.describeMethods());
    const creates = methods.filter(([, method]) => COLLECTION_KINDS.has(method.kind));
    const called = methods.filter(
        ([, method]) => !NOUN_KINDS.has(method.kind) && !COLLECTION_KINDS.has(method.kind),
    );

    return {
        description: `Get, create and change ${noun.plural} (${noun.qualifier})`,
        commands: Object.fromEntries([
            ...creates.map(([name]): [string, Verb] => [
                name,
                methodCommand(context, noun, name, undefined),
            ]),
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
                const rows = await (
                    await noun.client(context)
                ).list(noun.type.name, { limit: ID_PAGE });

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
            const show = (row: JsonObject) =>
                json ? JSON.stringify(row) : Render.object(row, noun.type, context.terminal);

            // follow the object as it changes until interrupted
            if (options["watch"] === true) {
                const live = context.terminal.live(json);
                const query = { where: { id: await noun.identify(id, context) } };
                using interrupted = new Interrupt(context.environment.process);
                for await (const rows of watch(
                    remote,
                    noun.type.name,
                    query,
                    interrupted.signal,
                    context.environment,
                )) {
                    const [row] = rows;
                    await live.show(row === undefined ? `No ${noun.name} ${id}.` : show(row));
                }
                interrupted.require();
            }
            // read it once through the noun's get method
            else {
                const row = await remote.call(noun.type.name, noun.method("get"), { id });
                await context.terminal.print(show(schema.jsonObject().parse(row)));
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
            return { ...FlagSet.input(noun.type, method).options(), ...INPUT_OPTION };
        },
        async run(call) {
            // read the flags, whose references the method resolves by key
            const { json, options } = call;
            const flags = FlagSet.input(noun.type, method);

            // build the input from the JSON option, the flags and the object
            const written = await methodInput(flags, options, context);
            const input = id === undefined ? written : { ...written, id };

            // call the method and show its result
            const remote = await noun.client(context);
            const result = await remote.call(noun.type.name, method, input);
            await context.terminal.print(
                json ? JSON.stringify(result) : renderResult(noun, result, context.terminal),
            );
        },
    };
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
            };
        },
        async run(call) {
            // refuse an object the command does not apply to, or which the person may not call it on
            const remote = await noun.client(context);
            const target = id === undefined ? undefined : await noun.identify(id, context);
            if (target !== undefined) {
                const [applies] = await remote.list(noun.type.name, {
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

            // build the input from the JSON option, the flags and the object, beside the preset fields
            const flags = FlagSet.input(noun.type, command.method).without(fields);
            const input = await methodInput(flags, call.options, context);

            // call the method and show its result
            const result = await remote.call(
                noun.type.name,
                command.method,
                command.input(target === undefined ? input : { ...input, id: target }),
            );
            await context.terminal.print(
                call.json ? JSON.stringify(result) : renderResult(noun, result, context.terminal),
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

/** Render a method's result: an object as its fields, anything else as JSON. */
function renderResult(noun: Noun, result: JsonValue, terminal: Terminal): string {
    const row = schema.jsonObject().safeParse(result);

    return row.success ? Render.object(row.data, noun.type, terminal) : JSON.stringify(result);
}

/** Follow a query's rows through an in-memory replica until the signal aborts. */
async function* watch(
    remote: DynamicObjectClient,
    type: string,
    query: Parameters<DynamicObjectClient["watch"]>[1],
    signal: AbortSignal,
    environment: CliEnvironment,
): AsyncGenerator<readonly JsonObject[]> {
    const database = await connectBunSqlite(":memory:", [], { environment: environment });
    try {
        yield* remote.watch(type, query, database, signal);
    } finally {
        await database.close();
    }
}
