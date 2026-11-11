import { none, through } from "@destack/access";
import type { Select } from "@destack/db";
import { defineObject, field, type MethodContext } from "@destack/object";
import { type Identifier, schema } from "@destack/schema";
import { account } from "./account.ts";
import { EndpointRelay, machine } from "./machine.ts";
import { nested } from "@destack/object";
import { argument, plural, t } from "@destack/locale";

/** Where a machine answers: its URL and the relay serving its tunnel. */
export const endpoint = defineObject(
    {
        name: "endpoint",
        plural: "endpoints",
        title: t`${plural(argument("count"), { one: "Endpoint", other: "Endpoints" })}`,
        scope: [account, "universe"],
        fields: {
            /** The URL the machine's packages answer at. */
            url: field.string(schema.url()),
            /** The relay serving its tunnel, null for a machine answering at its URL directly. */
            relay: field.json(EndpointRelay).optional(),
        },
        constraints: { machine: { kind: "unique", on: ["parentId"] } },
        access: {
            // let the universe's signed-in role grant reading endpoints, and the machine's declaration and relay record them
            permissions: {
                read: none(),
                list: none(),
                record: through("parent", "record"),
                withdraw: through("parent", "revoke"),
            },
            reserved: ["record"],
        },
        traits: [nested({ in: machine, delete: "cascade", add: "record" })],
    },
    (method) => ({
        /** Record a machine's first endpoint, as its declaration or its relay does. */
        create: method.create("record", {
            fields: ["url", "relay"],
            visibility: "package",
            isPredicted: false,
        }),

        /** Replace a machine's endpoint, as its relay does on each renewal. */
        update: method.update("record", { fields: ["url", "relay"], visibility: "package" }),

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
        const existing = await context.read(endpoint).findFirst({ where: { parentId: machineId } });
        await context.mutate(context.self, [
            existing === undefined
                ? endpoint.call("create", {
                      ...endpoint.in(context.scope),
                      parentId: machineId,
                      ...published,
                  })
                : endpoint.call("update", { ...endpoint.target(existing), ...published }),
        ]);
    },
};
