import { type Caller, Grant, principal } from "@destack/access";
import { Snapshot } from "@destack/db";
import type { ExchangeAsk } from "@destack/runtime/authentication";
import { RequestId } from "@destack/runtime/request";
import { subjectContext } from "@destack/runtime/test";
import {
    RunEvent,
    type RunClient,
    type RunDelivery,
    type RunRequest,
} from "@destack/runtime/trigger";
import type { Subject } from "@destack/sync";
import type { ObjectStore } from "../server/index.ts";

/** A machine running the calls an installation's store sends and fires, in that store. */
export class TestRunClient implements RunClient {
    /** The store of the installation, read once the store exists. */
    readonly #store: () => ObjectStore;

    /** Run the calls in the store of the installation recording them. */
    constructor(store: () => ObjectStore) {
        this.#store = store;
    }

    /** Run a call once per request as the installation, or for the exchange's subject. */
    async send(request: RunRequest, delivery: RunDelivery = {}): Promise<void> {
        // refuse a call due later, which only the machine's run waits for
        const store = this.#store();
        const at = "at" in request ? request.at : undefined;
        if (at !== undefined && at > store.clock.now()) {
            throw new TypeError("the test machine runs no call due later");
        }

        // key the run by its request: the sent one, or the firing event's
        const requestId =
            "event" in request
                ? RunEvent.requestId(request.event)
                : (delivery.requestId ?? RequestId.create(store.clock.now()));

        // run the call under that request in the installation's space, as the machine pushes it
        const installation = store.principal;
        if (!principal.installation.is(installation)) {
            throw new TypeError("the test machine runs calls of an installation's store alone");
        }
        const { scope } = installation;
        const exchange = "exchange" in request ? request.exchange : undefined;
        const caller = await TestRunClient.#caller(store, installation, exchange);
        await store.mutate(caller, { id: requestId, calls: [request.call] }, { scope });
    }

    /** Verify the installation's caller, or the subject's under its grant to the installation. */
    static async #caller(
        store: ObjectStore,
        installation: Subject,
        exchange: ExchangeAsk | undefined,
    ): Promise<Caller> {
        // act as the installation in its space
        const { clock } = store;
        const scope = installation.scope;
        if (exchange === undefined) {
            return subjectContext(installation, scope, { clock }).caller();
        }

        // act for the subject through the installation, under its grant when asked
        const delegates = [installation];
        const asked = exchange.grant;
        if (asked === undefined) {
            return subjectContext(exchange.subject, scope, { clock, delegates }).caller();
        }

        // require the asked grant live between the subject and the installation
        const read = await Grant.read(Snapshot.live(store.database), asked);
        const parties = { delegator: exchange.subject, delegate: installation };
        const grant = Grant.claim(Grant.require(read, asked, parties, clock.now()));

        return subjectContext(exchange.subject, scope, { clock, delegates, grant }).caller();
    }
}
