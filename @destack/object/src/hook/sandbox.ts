import { Caller, principal } from "@destack/access";
import { DatabaseError, type Row, TABLE } from "@destack/db";
import { schema, type JsonValue } from "@destack/schema";
import { CallError } from "@destack/runtime/error";
import type { InstallationContext } from "@destack/runtime/workload";
import type { ObjectReference, Subject } from "@destack/sync";
import type { InstalledDeclaration } from "../inspect/object.ts";
import type { Call, PhasedHandler } from "../method/call.ts";
import { PHASES, type Phase } from "../method/method.ts";
import type { ObjectType } from "../object/object.ts";
import {
    BranchCalls,
    CallRecord,
    CreationRecord,
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
    /** The race a request of the session lost, which the hook's failure carries back. */
    lost?: DatabaseError;
}

/** The phases a sandbox runs: every phase but settle, which runs where the settlement is kept. */
const SANDBOXED: readonly Phase[] = PHASES.filter((phase) => phase !== "settle");

/** An installation's object hooks, run in its sandbox and answered in their sessions. */
export class SandboxHooks {
    /** The installation the sandbox runs as. */
    readonly installation: InstallationContext;
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
        /** The installation the sandbox runs as. */
        readonly installation: InstallationContext;
        /** The declarations of the installation's followed build. */
        readonly declarations: readonly InstalledDeclaration[];
        /** Send a request to run a hook to the sandbox. */
        send(request: HandlerRequest): Promise<HandlerAnswer>;
    }) {
        this.installation = options.installation;
        this.declarations = options.declarations;
        this.#send = (request) => options.send(request);
    }

    /** The installation's principal, the subject of calls run as the installation. */
    get subject(): Subject {
        const { scope, id } = this.installation;

        return principal.installation.reference(scope, id);
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
        // run a call as the hook's caller or as the installation, reaching public methods and the package's own internal ones
        const { call } = session;
        if (asked.kind === "call") {
            const caller = asked.as === "caller" ? call.caller : Caller.of(this.subject, call.now);
            const result = await call.runSandboxed(
                caller,
                asked.call,
                asked.mutates,
                this.installation.build.manifest.package.id,
            );

            return schema.json().parse(result ?? null);
        }
        // read one row of a type the installation serves, absent when the caller may not read it
        else if (asked.kind === "row") {
            const type = this.#type(asked.table);
            const table = type.table[TABLE];
            const row = await call.read(type).row(table.decode(asked.key));
            const [readable] = row === null ? [] : await SandboxHooks.#readable(call, type, [row]);

            return readable === undefined ? null : table.encode(readable);
        }
        // read the rows of a type the installation serves that the caller may read
        else if (asked.kind === "rows") {
            const type = this.#type(asked.table);
            const rows = await call.read(type).rows(asked.where, {
                ...(asked.orderBy === undefined ? {} : { orderBy: asked.orderBy }),
                ...(asked.limit === undefined ? {} : { limit: asked.limit }),
            });
            const readable = await SandboxHooks.#readable(call, type, rows);

            return readable.map((row) => type.table[TABLE].encode(row));
        }
        // run the behaviour a handle phase wraps with the hook's input
        else if (session.next !== undefined) {
            return HookResult.encode(call, await session.next(call.with({ input: asked.input })));
        }
        // refuse it outside a handle phase
        else {
            throw new TypeError(`${call.object.name}.${call.name} wraps nothing outside handle`);
        }
    }

    /** Find a type the installation serves by its table, refusing any other. */
    #type(name: string): ObjectType {
        const type = this.#types.get(name);
        if (type === undefined) {
            throw new CallError("FORBIDDEN", {
                message: `hooks read the tables of the types their installation serves, not ${name}`,
            });
        }

        return type;
    }

    /** Keep the rows the hook's caller may read, as the call's authorization decides them. */
    static async #readable<Read extends Row>(
        call: Call,
        type: ObjectType,
        rows: readonly Read[],
    ): Promise<Read[]> {
        // refuse a type nobody reads
        const reading = type.reading;
        if (reading === undefined) {
            throw new CallError("FORBIDDEN", { message: `no caller reads ${type.plural}` });
        }

        // keep the rows the caller may read in the call's scope
        const authorization = call.requireAuthorization();
        const { permitted } = await authorization.admitRows(type, reading, call.scope, rows);

        return rows.filter((_row, index) => permitted.has(index));
    }

    /** Build a method's handler running its listed phases in the sandbox. */
    #handler(type: ObjectType, name: string, phases: readonly string[]): PhasedHandler {
        // refuse a phase the sandbox does not run
        const unknown = phases.find((phase) => !SANDBOXED.some((each) => each === phase));
        if (unknown !== undefined) {
            throw new TypeError(`${type.name}.${name} has no phase ${unknown} a sandbox runs`);
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
