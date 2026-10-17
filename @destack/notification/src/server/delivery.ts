import { GONE, message } from "@destack/message";
import type { schema } from "@destack/schema";
import type { RunContext, Step } from "@destack/workflow";
import { type deliver, notification } from "../object/notification.ts";
import type { NotificationServer } from "./server.ts";

/** A push one round sent, which the round waits on to settle. */
interface Push {
    /** The destination's key. */
    readonly key: string;
    /** The browser's push endpoint. */
    readonly url: string;
    /** The message sent. */
    readonly message: string;
}

/** A destination deferred to a later round, until a time. */
interface Deferral {
    /** The destination's key. */
    readonly key: string;
    /** When the destination is decided again. */
    readonly until: number;
}

/** An occurrence's delivery to its person's destinations, decided in rounds until none waits. */
export class Delivery {
    /** The server, whose push lifetime the delivery waits for. */
    readonly #server: NotificationServer;
    /** The run's steps. */
    readonly #step: Step;
    /** The run's input: the notification and its occurrence. */
    readonly #input: schema.Infer<typeof deliver.input>;
    /** The home the run delivers from. */
    readonly #home: string;

    /** Deliver an occurrence in a run's steps. */
    constructor(
        server: NotificationServer,
        step: Step,
        input: schema.Infer<typeof deliver.input>,
        run: RunContext,
    ) {
        // take the server, the steps, the input and the run's home
        this.#server = server;
        this.#step = step;
        this.#input = input;
        this.#home = run.scope;
    }

    /** Decide each destination in rounds until none waits, sleeping until the earliest deferral. */
    async run(): Promise<void> {
        // decide every destination in the first round, and the deferred ones after
        let waiting: readonly string[] | undefined;
        for (let round = 0; ; round++) {
            const deferred = await this.#round(round, waiting);
            if (deferred.length === 0) {
                return;
            }

            // sleep until the earliest deferred destination
            waiting = deferred.map((each) => each.key);
            const until = Math.min(...deferred.map((each) => each.until));
            await this.#step.sleepUntil(Delivery.#name("wait", round), until);
        }
    }

    /** Decide the waiting destinations, send or schedule each, and answer the deferred ones. */
    async #round(round: number, waiting: readonly string[] | undefined): Promise<Deferral[]> {
        // decide the waiting destinations
        const target = notification.target({ scope: this.#home, id: this.#input.notification });
        const decide = notification.call("decide", {
            ...target,
            occurredAt: this.#input.occurredAt,
            ...(waiting === undefined ? {} : { destinations: [...waiting] }),
        });
        const { items } = await this.#step.query(Delivery.#name("decide", round), decide);

        // send what goes out now, and schedule what waits for a summary
        const pushes: Push[] = [];
        for (const { key, to, decision } of items) {
            // send it
            if (decision.action === "send") {
                const send = notification.call("send", { ...target, to });
                const sent = await this.#step.call(Delivery.#name("send", round, key), send);
                if (to.channel === "push") {
                    pushes.push({ key, url: to.url, message: sent.message });
                }
            }
            // schedule it for the summary
            else if (decision.action === "summarize") {
                const schedule = notification.call("schedule", { ...target, at: decision.at });
                await this.#step.call(Delivery.#name("schedule", round, key), schedule);
            }
        }

        // forget each browser whose push service forgot it
        for (const push of pushes) {
            await this.#settle(round, push);
        }

        return items.flatMap(({ key, decision }) =>
            decision.action === "defer" ? [{ key, until: decision.until }] : [],
        );
    }

    /** Wait for a push to settle, forgetting the browser once its push service forgot it. */
    async #settle(round: number, push: Push): Promise<void> {
        // wait for the push service's answer, at most as long as it stores the message
        const settled = await this.#step.waitFor(
            Delivery.#name("settle", round, push.key),
            message.call("list", {
                ...message.in(this.#home),
                where: { id: push.message, settledAt: { isNotNull: true } },
                limit: 1,
            }),
            { timeout: this.#server.pushTtl },
        );

        // forget a browser the push service no longer knows
        if (settled?.reason === GONE) {
            const target = notification.target({ scope: this.#home, id: this.#input.notification });
            const forget = notification.call("forget", { ...target, url: push.url });
            await this.#step.call(Delivery.#name("forget", round, push.key), forget);
        }
    }

    /** Name a step of a round, for one destination when given. */
    static #name(verb: string, round: number, key?: string): string {
        return key === undefined ? `${verb}.${round}` : `${verb}.${round}.${key}`;
    }
}
