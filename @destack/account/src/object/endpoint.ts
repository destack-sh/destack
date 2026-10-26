import { none, through } from "@destack/access";
import type { Select } from "@destack/db";
import { defineObject, field, type MethodContext } from "@destack/object";
import { type Identifier, schema } from "@destack/schema";
import { account } from "./account.ts";
import { EndpointRelay, machine } from "./machine.ts";

/** Where a machine answers: its URL and the relay serving its tunnel. */
export const endpoint = defineObject(
    {
        name: "endpoint",
        plural: "endpoints",
        scope: [account, "universe"],
        key: schema.identifier("machine"),
        nested: { in: machine, delete: "cascade", receive: "publish" },
        fields: {
            /** The URL the machine's packages answer at. */
            url: field.string(schema.url()),
            /** The relay serving its tunnel, null for a machine answering at its URL directly. */
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
    },
    (method) => ({
        /** Publish a machine's first endpoint, as its machine's publish does. */
        create: method.create("publish", {
            fields: ["url", "relay"],
            visibility: "package",
            isPredicted: false,
        }),

        /** Replace a published endpoint, as its machine's publish does. */
        update: method.update("publish", { fields: ["url", "relay"], visibility: "package" }),

        /** Withdraw an endpoint with its revoked machine. */
        delete: method.delete("withdraw", { visibility: "package" }),
    }),
);
/** An endpoint as its table stores it. */
export type Endpoint = Select<typeof endpoint.table>;

/** The reads of endpoints. */
export const Endpoint = {
    /** Report whether an endpoint answers at a time: directly, or through an unlapsed tunnel. */
    isAnswering(row: Pick<Endpoint, "relay">, now: number): boolean {
        return row.relay === null || row.relay.lapsesAt > now;
    },

    /** Create or replace a machine's endpoint as the universe, its calling method gating it. */
    async upsert(
        context: Pick<MethodContext, "read" | "mutate" | "self" | "scope">,
        machineId: Identifier<"machine">,
        published: Pick<Endpoint, "url" | "relay">,
    ): Promise<void> {
        const target = endpoint.target({ scope: context.scope, id: machineId });
        const existing = await context.read(endpoint).findFirst({ where: { id: machineId } });
        await context.mutate(context.self, [
            existing === undefined
                ? endpoint.call("create", { ...target, parentId: machineId, ...published })
                : endpoint.call("update", { ...target, ...published }),
        ]);
    },
};
