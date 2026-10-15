import { AccessContext, Caller, Decision, principal } from "@destack/access";
import { DatabaseError, type Row, TABLE } from "@destack/db";
import type { EventKeyShape } from "@destack/event";
import { schema, type JsonValue } from "@destack/schema";
import { CallError } from "@destack/runtime/error";
import type { ServerContext } from "@destack/runtime/workload";
import { type ObjectReference, Subject } from "@destack/sync";
import type { InstalledDeclaration } from "../inspect/object.ts";
import type { Call, PhasedHandler } from "../method/call.ts";
import { PHASES, type Phase } from "../method/method.ts";
import type { ObjectType } from "../object/object.ts";
import type { EventView } from "../query/event.ts";
import {
    BranchCalls,
    CallRecord,
    CreationRecord,
    type EventRequest,
    HandlerAnswer,
    type HandlerRequest,
    HookResult,
    type SessionRequest,
} from "./record.ts";

/** A mutation's call whose hook runs in the sandbox, with the behaviour a handle phase wraps. */
interface Session {
    /** The call. */
    readonly call: Call;
    /** Run the behaviour the hook wraps, absent outside a handle phase. */
    readonly next?: (call?: Call) => Promise<unknown>;
    /** The race a request of the session lost, returned with the hook's failure. */
    lost?: DatabaseError;
}

/** The phases a sandbox runs: every phase but settle, which runs where the settlement is kept. */
const SANDBOXED: readonly Phase[] = PHASES.filter((phase) => phase !== "settle");

/** An installation's object hooks, run in its sandbox and answered in their sessions. */
export class HookSandbox {
    /** The server context of the installation the sandbox runs as. */
    readonly server: ServerContext;
    /** The declarations of the installation's followed build, its object types among them. */
    readonly declarations: readonly InstalledDeclaration[];
    /** Send a request to run a hook to the sandbox. */
    readonly #send: (request: HandlerRequest) => Promise<HandlerAnswer>;
    /** The types the installation serves, whose rows its hooks read, by their tables' SQL names. */
    readonly #types = new Map<string, ObjectType>();
    /** The calls of the hooks running now, by their sessions' tokens. */
    readonly #sessions = new Map<string, Session>();

    /** Run an installation's hooks in its sandbox. */
    constructor(options: {
        /** The server context of the installation the sandbox runs as. */
        readonly server: ServerContext;
        /** The declarations of the installation's followed build. */
        readonly declarations: readonly InstalledDeclaration[];
        /** Send a request to run a hook to the sandbox. */
        readonly send: (request: HandlerRequest) => Promise<HandlerAnswer>;
    }) {
        this.server = options.server;
        this.declarations = options.declarations;
        this.#send = (request) => options.send(request);
    }

    /** The installation's principal, the subject of calls run as the installation. */
    get subject(): Subject {
        const { space, installation } = this.server;

        return principal.installation.reference(space, installation);
    }

    /** Serve a type as the installation, its methods' listed phases run in the sandbox. */
    implement(type: ObjectType, hooks: Readonly<Record<string, readonly string[]>>): ObjectType {
        // let the hooks read the rows of every type the installation serves, and keep a type without hooks
        this.#types.set(type.table[TABLE].sqlName, type);
        const entries = Object.entries(hooks);
        if (entries.length === 0) {
            return type;
        }

        // run each listed phase in the sandbox
        const methods = Object.fromEntries(
            entries.map(([name, phases]) => [name, this.#handler(type, name, phases)]),
        );

        return type.implement({ methods });
    }

    /** Answer a request a hook sends in its mutation's session. */
    answer(request: SessionRequest): Promise<HandlerAnswer> {
        return HandlerAnswer.of(async () => {
            // require the hook's open session
            const session = this.#sessions.get(request.mutation);
            if (session === undefined) {
                throw new CallError("NOT_FOUND", { message: "no hook runs in this session" });
            }

            // keep a lost race, which the mutation runs again once the hook fails with it
            try {
                return await this.#answer(session, request.request);
            } catch (error) {
                if (error instanceof DatabaseError && error.code === "CONCURRENT_UPDATE") {
                    session.lost = error;
                }
                throw error;
            }
        });
    }

    /** Answer a hook's request in its session. */
    async #answer(session: Session, asked: SessionRequest["request"]): Promise<JsonValue> {
        // run a call as the hook's caller or as the installation, allowing the package's internal methods
        const { call } = session;
        if (asked.kind === "call") {
            const caller =
                asked.as === "caller"
                    ? call.caller
                    : AccessContext.of(Caller.of(this.subject), call.now);
            const result = await call.runSandboxed(
                caller,
                asked.call,
                asked.mutates,
                this.server.build.manifest.package.id,
            );

            return schema.json().parse(result ?? null);
        }
        // read the first row or the rows of a type the installation serves, as the package
        else if (asked.kind === "findFirst" || asked.kind === "findMany") {
            const type = this.#type(asked.table);
            const reader = asked.columns === "all" ? call.reveal(type) : call.read(type);
            const query = {
                ...(asked.where === undefined ? {} : { where: asked.where }),
                ...(asked.orderBy === undefined ? {} : { orderBy: asked.orderBy }),
                ...(asked.limit === undefined ? {} : { limit: asked.limit }),
            };
            const encode = (row: Row) => type.table[TABLE].encode(row);
            if (asked.kind === "findFirst") {
                const row = await reader.findFirst(query);

                return row === undefined ? null : encode(row);
            }

            return (await reader.findMany(query)).map(encode);
        }
        // read events of a type the installation serves
        else if (asked.kind === "events") {
            return HookSandbox.#events(
                call.events(this.#type(asked.table).eventKind()),
                asked.read,
            );
        }
        // list the subjects with a permission on an object
        else if (asked.kind === "lookup") {
            const { permission, object, type, page } = asked.lookup;
            const subjects = await call.lookup(permission, object, { type, page });

            return subjects.map((subject) => Subject.parse(subject));
        }
        // rehearse calls over rows
        else if (asked.kind === "rehearse") {
            return schema.json().parse(await call.rehearse(asked.over, asked.calls));
        }
        // decide a permission of the hook's caller
        else if (asked.kind === "check") {
            return Decision.parse(await call.check(asked.permission, asked.object));
        }
        // require a permission of the hook's caller
        else if (asked.kind === "require") {
            await call.require(asked.permission, asked.object);

            return null;
        }
        // send a call once the mutation commits
        else if (asked.kind === "send") {
            await call.send({
                call: asked.call,
                ...(asked.at === undefined ? {} : { at: asked.at }),
            });

            return null;
        }
        // take the transaction's next position
        else if (asked.kind === "position") {
            return call.takePosition();
        }
        // run the behaviour a handle phase wraps with the hook's input
        else if (session.next !== undefined) {
            return HookResult.encode(call, await session.next(call.with({ input: asked.input })));
        }
        // refuse it outside a handle phase
        else {
            throw new TypeError(
                `${call.object.name}.${call.name} has nothing to wrap outside a handle phase`,
            );
        }
    }

    /** Answer a hook's read of events as JSON. */
    static async #events(
        view: EventView<EventKeyShape, JsonValue>,
        read: EventRequest,
    ): Promise<JsonValue> {
        // read a page, keeping its cursor where one follows
        if (read.verb === "query") {
            const { events, cursor } = await view.query(read.filter, read.page);

            return schema.json().parse({ events, ...(cursor === undefined ? {} : { cursor }) });
        }
        // read one event, null when absent
        else if (read.verb === "get") {
            const event = await view.get(read);

            return event === undefined ? null : schema.json().parse(event);
        }
        // read the events appended after a sequence
        else if (read.verb === "changes") {
            const { events, sequence } = await view.changes(read.filter, read.after, read.limit);

            return schema.json().parse({ events, sequence });
        }

        return schema.json().parse(await view.series(read.filter, read.series));
    }

    /** Find a type the installation serves by its table, refusing any other. */
    #type(name: string): ObjectType {
        const type = this.#types.get(name);
        if (type === undefined) {
            throw new CallError("FORBIDDEN", {
                message: `hooks cannot read ${name}: only the tables of the types their installation serves`,
            });
        }

        return type;
    }

    /** Build a method's handler running its listed phases in the sandbox. */
    #handler(type: ObjectType, name: string, phases: readonly string[]): PhasedHandler {
        // refuse a phase the sandbox does not run
        const unknown = phases.find((phase) => !SANDBOXED.some((each) => each === phase));
        if (unknown !== undefined) {
            throw new TypeError(
                `${type.name}.${name} has phase ${unknown}, which a sandbox does not run`,
            );
        }

        // send each listed phase to the sandbox
        const listed = new Set(phases);
        const run = (call: Call, hook: HandlerRequest["hook"], next?: Session["next"]) =>
            this.#run({ call, ...(next === undefined ? {} : { next }) }, hook);

        return {
            ...(listed.has("authorize")
                ? {
                      authorize: async (call: Call) => {
                          await run(call, { phase: "authorize" });
                      },
                  }
                : {}),
            ...(listed.has("creation")
                ? {
                      creation: async (call: Call, object: ObjectReference) =>
                          CreationRecord.parse(await run(call, { phase: "creation", object })),
                  }
                : {}),
            ...(listed.has("expand")
                ? {
                      expand: async (call: Call) =>
                          BranchCalls.parse(await run(call, { phase: "expand" })),
                  }
                : {}),
            ...(listed.has("prepare")
                ? { prepare: (call: Call) => run(call, { phase: "prepare" }) }
                : {}),
            ...(listed.has("handle")
                ? {
                      handle: async (call: Call, next: (call?: Call) => Promise<unknown>) =>
                          HookResult.decode(call, await run(call, { phase: "handle" }, next)),
                  }
                : {}),
        };
    }

    /** Run one hook of a call in the sandbox, its requests answered in a session while it runs. */
    async #run(session: Session, hook: HandlerRequest["hook"]): Promise<JsonValue> {
        // open the session while the hook runs
        const mutation = crypto.randomUUID();
        this.#sessions.set(mutation, session);
        try {
            const call = CallRecord.of(session.call);
            const answer = await this.#send({ mutation, call, hook });

            // fail with the race a request lost, else with the hook's own failure
            if ("error" in answer && session.lost !== undefined) {
                throw session.lost;
            }

            return HandlerAnswer.result(answer);
        } finally {
            this.#sessions.delete(mutation);
        }
    }
}
