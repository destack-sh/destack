import { and, type DatabaseConnection, eq, type Table, type Tracker } from "@destack/db";
import { principal } from "@destack/access";
import type { ObjectType } from "@destack/object";
import { EphemeralStorage } from "@destack/object/server";
import { type Identifier, present } from "@destack/schema";
import { ObjectReference, type Subject } from "@destack/sync";
import { socket, type SocketKind } from "../../object/socket.ts";
import stackPackage from "../../package.ts";
import type { Environment } from "@destack/environment";

/** What a socket's opener knows of the socket. */
export interface SocketOptions {
    /** What the link connects. */
    readonly kind: SocketKind;
    /** The principal connected. */
    readonly principal: Subject;
    /** The account client the socket's credential is bound to, absent for an unbound credential. */
    readonly client?: Identifier<"client">;
    /** The session the socket authenticated with, as access decisions key it. */
    readonly session?: string;
    /** The installation connected. */
    readonly installation?: Identifier<"installation">;
}

/** An open socket its opener renews with beats and releases on close. */
export interface SocketLease {
    /** The socket. */
    readonly id: Identifier<"socket">;
    /** Renew the lease with a heartbeat now. */
    readonly beat: () => Promise<void>;
    /** Release the lease, removing the socket. */
    readonly close: () => Promise<void>;
}

/** The live sockets of a machine's spaces in a replicated memory database. */
export class SocketRegistry implements AsyncDisposable {
    /** The memory store the stack package serves the sockets from. */
    readonly storage: EphemeralStorage;
    /** The environment the sockets run in. */
    readonly #environment: Environment;

    /** Write sockets into a memory store. */
    private constructor(storage: EphemeralStorage, environment: Environment) {
        this.storage = storage;
        this.#environment = environment;
    }

    /** Store the sockets and the spaces' other ephemeral objects in a migrated memory database. */
    static async open(options: {
        /** The ephemeral object types the spaces serve with the sockets, such as presence. */
        readonly objects: readonly ObjectType[];
        /** The tracker of the machine's live instances. */
        readonly tracker: Tracker;
        /** Open a memory database with tables. */
        readonly memory: (
            tables: readonly Table[],
        ) => Promise<DatabaseConnection & { readonly close: () => Promise<void> }>;
        /** Report a failure to end a closed socket's rows. */
        readonly report: (error: unknown) => void;
        /** The environment the sockets run in. */
        readonly environment: Environment;
    }): Promise<SocketRegistry> {
        const storage = await EphemeralStorage.open(
            [...Object.values(stackPackage.objects), ...options.objects],
            options.tracker,
            (tables) => options.memory(tables),
            { report: (error) => options.report(error) },
        );

        return new SocketRegistry(
            present(storage, "the stack package's ephemeral storage"),
            options.environment,
        );
    }

    /** Open a socket to a space, live while its opener tracks it and gone once closed. */
    async connect(spaceId: Identifier<"space">, options: SocketOptions): Promise<SocketLease> {
        // write the socket's row as its writer while the opener tracks it
        const id = socket.generateId(this.#environment);
        const release = this.storage.track(id);
        const now = this.#environment.clock.now();
        try {
            await this.storage.write(id, async (transaction) => {
                await transaction.insert(socket.table).values({
                    id,
                    scope: spaceId,
                    writerId: id,
                    kind: options.kind,
                    ...ObjectReference.columns("principal", options.principal),
                    clientId: options.client ?? null,
                    session: options.session ?? null,
                    installationId: options.installation ?? null,
                    lastActiveAt: now,
                    createdAt: now,
                    ...ObjectReference.columns("createdBy", undefined),
                    updatedAt: now,
                    ...ObjectReference.columns("updatedBy", undefined),
                });
            });
        } catch (error) {
            release();
            throw error;
        }

        // refresh the row on each heartbeat and remove it on close
        let isClosed = false;

        return {
            id,
            beat: async () => {
                const beaten = this.#environment.clock.now();
                await this.storage.write(id, async (transaction) => {
                    await transaction
                        .update(socket.table)
                        .set({ lastActiveAt: beaten, updatedAt: beaten })
                        .where(eq(socket.table.id, id));
                });
            },
            close: async () => {
                // remove the row once and stop tracking its owner
                if (isClosed) {
                    return;
                }
                isClosed = true;
                try {
                    await this.storage.write(id, async (transaction) => {
                        await transaction.delete(socket.table).where(eq(socket.table.id, id));
                    });
                } finally {
                    release();
                }
            },
        };
    }

    /** Open a principal's socket to a space for one of its streams. */
    async link(spaceId: Identifier<"space">, subject: Subject): Promise<SocketLease> {
        return await this.connect(spaceId, {
            kind: SocketRegistry.#kindOf(subject),
            principal: subject,
        });
    }

    /** Read the kind of link a principal opens. */
    static #kindOf(subject: Subject): SocketKind {
        // link a machine as itself
        if (principal.machine.is(subject)) {
            return "machine";
        }
        // link an installation as a workload
        else if (principal.installation.is(subject)) {
            return "workload";
        }
        // link anyone else as a browser tab
        else {
            return "tab";
        }
    }

    /** Report whether a principal has a socket open on any instance. */
    async isConnected(subject: Subject): Promise<boolean> {
        const [open] = await this.storage.database
            .select({ id: socket.table.id })
            .from(socket.table)
            .where(
                and(
                    eq(socket.table.principalType, subject.type),
                    eq(socket.table.principalId, subject.id),
                ),
            )
            .limit(1);

        return open !== undefined;
    }

    /** Stop replicating and close the memory database. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.storage[Symbol.asyncDispose]();
    }
}
