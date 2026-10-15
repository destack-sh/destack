import { none, through } from "@destack/access";
import type { Select } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { schema } from "@destack/schema";
import { account } from "./account.ts";
import { EndpointRelay, machine } from "./machine.ts";

/** Where a machine answers: its URL and the relay keeping its tunnel. */
export const endpoint = defineObject({
    name: "endpoint",
    plural: "endpoints",
    scope: [account, "universe"],
    key: schema.identifier("machine"),
    nested: { in: machine, delete: "cascade", receive: "publish" },
    fields: {
        /** The URL the machine's packages answer at. */
        url: field.string(schema.url()),
        /** The relay keeping its tunnel, null for a machine answering at its URL directly. */
        relay: field.json(EndpointRelay).optional(),
    },
    // let the universe's signed-in role grant reading endpoints
    permissions: {
        read: none(),
        list: none(),
        publish: through("parent", "publish"),
        withdraw: through("parent", "revoke"),
    },
    reserved: ["publish"],
    constraints: { machine: { kind: "unique", on: ["parentId"] } },
}).methods((method) => ({
    /** Publish a machine's first endpoint, as its machine's publish does. */
    create: method.create("publish", {
        fields: ["url", "relay"],
        isInternal: true,
        isPredicted: false,
    }),

    /** Replace a published endpoint, as its machine's publish does. */
    update: method.update("publish", { fields: ["url", "relay"], isInternal: true }),

    /** Withdraw an endpoint with its revoked machine. */
    delete: method.delete("withdraw", { isInternal: true }),
}));
/** An endpoint as its table stores it. */
export type Endpoint = Select<typeof endpoint.table>;

/** The reads of endpoints. */
export const Endpoint = {
    /** Report whether an endpoint answers at a time: directly, or through an unlapsed tunnel. */
    isAnswering(row: Pick<Endpoint, "relay">, now: number): boolean {
        return row.relay === null || row.relay.lapsesAt > now;
    },
};
