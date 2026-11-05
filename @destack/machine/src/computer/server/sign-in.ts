import { type Controller, eq, type Following, type Select } from "@destack/db";
import type { ObjectStore } from "@destack/object/server";
import { waitAborted } from "@destack/environment";
import { present } from "@destack/schema";
import { RequestId } from "@destack/sync";
import type { Device } from "../device/device.ts";
import { signIn, type SignInState } from "../device/object/index.ts";
import type { Login } from "@destack/account/native";
import { SignUp } from "@destack/account/better-auth";

/** The most sign-ins the machine runs at once: a few programs asking together. */
const RUNNING_SIGN_INS = 8;

/** Run the sign-ins this computer's programs start, writing where each stands. */
export class SignInRunner {
    /** The native device signing people in. */
    readonly #device: Device;
    /** The device's store holding the sign-ins. */
    readonly #store: Pick<ObjectStore, "database" | "mutate" | "as" | "clock" | "environment">;

    /** Run the sign-ins of a device's store. */
    constructor(
        device: Device,
        store: Pick<ObjectStore, "database" | "mutate" | "as" | "clock" | "environment">,
    ) {
        this.#device = device;
        this.#store = store;
    }

    /** Build the controller running each sign-in until it is removed. */
    controller(): Controller<typeof signIn.table> {
        return {
            name: "sign-in",
            table: signIn.table,
            pending: { deletedAt: { isNull: true } },
            concurrency: RUNNING_SIGN_INS,
            follow: (rows, following) => this.follow(rows, following),
        };
    }

    /** Run a sign-in from its start, then hold it until removed. */
    async follow(
        rows: readonly Select<typeof signIn.table>[],
        following: Following,
    ): Promise<void> {
        // read the sign-in, reporting the follow caught up at once
        const row = present(rows[0], "a followed sign-in's row");
        const { signal, synced } = following;
        synced();

        // run a starting sign-in, recording the login or the failure
        if (row.state.kind === "starting") {
            try {
                const login = await this.#run(row, signal);
                await this.#advance(row, { kind: "signedIn", login: login.id });
            } catch (error) {
                if (!signal.aborted) {
                    const reason = error instanceof Error ? error.message : String(error);
                    await this.#advance(row, { kind: "failed", reason });
                }
            }
        }
        // remove a finished sign-in a restart left behind, its client gone
        else if (row.state.kind === "signedIn" || row.state.kind === "failed") {
            await this.#remove(row);
        }
        // fail a sign-in a restart of the machine interrupted
        else {
            await this.#advance(row, {
                kind: "failed",
                reason: "the machine restarted during the sign-in",
            });
        }

        // hold the sign-in until its client removes it
        await waitAborted(signal);
    }

    /** Sign the person in by the sign-in's method, writing what they see at each step. */
    async #run(row: Select<typeof signIn.table>, signal: AbortSignal): Promise<Login["row"]> {
        const { method } = row;
        const advance = (state: SignInState) => this.#advance(row, state);

        // open the browser's page
        if (method.kind === "browser") {
            return this.#device.signIn((url) => advance({ kind: "open", url }), signal);
        }
        // show a code to enter at a page
        else if (method.kind === "code") {
            return this.#device.signInWithCode(
                ({ url, userCode }) => advance({ kind: "enter", url, userCode }),
                signal,
            );
        }
        // wait for the code mailed to the person, signing them up when asked
        else {
            const ask = async () => {
                await advance({ kind: "waiting", email: method.email });

                return this.#answered(row, signal);
            };

            return method.kind === "email"
                ? this.#device.signInWithEmail(method.email, ask)
                : this.#device.signInWithEmail(method.email, ask, SignUp.parse(method.signUp));
        }
    }

    /** Wait until the person answers the mailed code. */
    async #answered(row: Select<typeof signIn.table>, signal: AbortSignal): Promise<string> {
        // read the sign-in at each change of its table
        let code: string | undefined;
        const isAnswered = async () => {
            const [current] = await this.#store.database
                .select()
                .from(signIn.table)
                .where(eq(signIn.table.id, row.id));
            code = current?.state.kind === "answered" ? current.state.code : undefined;

            return code !== undefined;
        };
        await this.#store.database.log.waitUntil(isAnswered, signal);

        return present(code, `the answer of sign-in ${row.id}`);
    }

    /** Remove a sign-in, as the machine running it. */
    async #remove(row: Select<typeof signIn.table>): Promise<void> {
        await this.#store.mutate(this.#store.as(signIn, this.#store.clock.now()), {
            id: RequestId.create(this.#store.environment),
            calls: [signIn.call("remove", signIn.target(row))],
        });
    }

    /** Record where a sign-in stands, as the machine running it. */
    async #advance(row: Select<typeof signIn.table>, state: SignInState): Promise<void> {
        const call = signIn.call("advance", { ...signIn.target(row), state });
        await this.#store.mutate(this.#store.as(signIn, this.#store.clock.now()), {
            id: RequestId.create(this.#store.environment),
            calls: [call],
        });
    }
}
