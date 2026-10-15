import { Caller, principal } from "@destack/access";
import { type DatabaseConnection, type Select, Snapshot } from "@destack/db";
import type { ObjectStore } from "@destack/object/server";
import { CallError } from "@destack/runtime/error";
import { RequestId } from "@destack/runtime/request";
import { Scope } from "@destack/sync";
import { defineObject, field } from "@destack/object";
import { defineSchema, schema } from "@destack/schema";
import { CompactProof } from "@destack/identity";
import { key } from "./key.ts";
import { machine } from "./machine.ts";
import { Platform, PLATFORMS } from "./platform.ts";
import { user } from "./user.ts";
import { DisplayName } from "./display-name.ts";
import { PushSubscription } from "./contact.ts";

/** The name a device's client starts with, which its program renames once it signs in. */
export const DEVICE_CLIENT_NAME = "Destack app";

/** The longest operating system version, in characters, such as 14.5.0. */
const VERSION_LENGTH = 64;

/** The kinds of clients. */
export const CLIENT_KINDS = ["native", "browser", "mobile"] as const;

/** A browser profile a machine registers for the person signed in on it. */
export const ClientRegistration = defineSchema(
    schema.object({
        /** The client's identifier, which the browser generated with its key on its first visit. */
        id: schema.identifier("client"),
        /** The person's name for the client. */
        name: DisplayName,
        /** The operating system. */
        platform: Platform.exactOptional(),
    }),
);
/** A browser profile a machine registers for the person signed in on it. */
export type ClientRegistration = schema.Infer<typeof ClientRegistration>;

/** A program a person signs in through, identified by its keys, after OAuth's client instances. */
export const client = defineObject({
    name: "client",
    plural: "clients",
    scope: user,
    // replicate a user's clients into the scopes inside the user's scope
    inherited: {},
    attachments: [key.attach({ by: "rotate" })],
    fields: {
        /** The person's name for the client. */
        name: field.string(DisplayName),
        /** What the client is. */
        kind: field.enum(CLIENT_KINDS),
        /** The machine the client runs on, absent where none runs Destack. */
        machine: field.reference(machine, { delete: "null" }).optional(),
        /** The operating system. */
        platform: field.enum(PLATFORMS).optional(),
        /** The operating system's version. */
        platformVersion: field.string(schema.string().min(1).max(VERSION_LENGTH)).optional(),
        /** The processor architecture, such as arm64. */
        architecture: field.string().optional(),
        /** The hardware model. */
        model: field.string().optional(),
        /** The version of the Destack program that registered the client. */
        version: field.string().optional(),
        /** The Web Push subscription of a browser client, absent until it subscribes. */
        push: field.json(PushSubscription).sensitive().optional(),
        /** The last time the client proved itself. */
        lastActiveAt: field.time().optional(),
        /** The time the person withdrew the client. */
        revokedAt: field.time().optional(),
    },
    permissions: [
        "read",
        "list",
        "create",
        "update",
        "verify",
        "rotate",
        "unsubscribe",
        "revoke",
        "report",
    ],
    constraints: {
        scope: { kind: "index", on: ["scope"] },
        push: {
            kind: "check",
            condition: { OR: [{ push: { isNull: true } }, { kind: "browser" }] },
        },
    },
})
    .methods((method) => ({
        list: method.list("list"),
        create: method.create("create", {
            fields: [
                "name",
                "kind",
                "machineId",
                "platform",
                "platformVersion",
                "architecture",
                "model",
                "version",
            ],
            isPredicted: false,
        }),
        update: method.update("update", {
            fields: ["name", "platform", "platformVersion", "architecture", "model", "version"],
        }),
        /** Attach the calling session to a client by a proof of one of its keys. */
        attach: method.mutation({
            permission: "update",
            input: schema.object({ proof: CompactProof }),
        }),
        /** Keep a browser client's Web Push subscription. */
        subscribe: method.mutation({
            permission: "update",
            input: schema.object({ push: PushSubscription }),
        }),
        /** Forget a client's Web Push subscription, as its push service no longer knows it. */
        unsubscribe: method.mutation({ permission: "unsubscribe" }),
        /** Withdraw a client with its keys and sessions. */
        revoke: method.mutation({ permission: "revoke" }),
        /** Record the client's proven contact now. */
        report: method.mutation({ permission: "report", isInternal: true }),
    }))
    .implement({
        methods: {
            unsubscribe: (call) => call.update({ push: null }),
            report: (call) => call.update({ lastActiveAt: call.now }),
        },
    });
/** A persisted client record. */
export type Client = Select<typeof client.table>;

/** The reads and registrations of clients. */
export const Client = {
    /** Report whether a client may sign in: a client identifier no person withdrew. */
    async isAdmissible(database: DatabaseConnection, clientId: string): Promise<boolean> {
        // refuse an identifier of anything but a client
        const id = schema.identifier("client").safeParse(clientId);
        if (!id.success) {
            return false;
        }

        // refuse a withdrawn client
        const row = await Snapshot.live(database)
            .read(client.table)
            .findFirst({ where: { id: id.data } });

        return row === undefined || row.revokedAt === null;
    },

    /** Register a device's client for a person once, refusing another person's client. */
    async register(
        objects: Pick<ObjectStore, "mutate" | "database">,
        clientId: string,
        userId: string,
        now: number,
    ): Promise<void> {
        // keep a client the person registered before, refusing another person's
        const id = client.identifier(clientId);
        const row = await Snapshot.live(objects.database)
            .read(client.table)
            .findFirst({ where: { id } });
        if (row !== undefined && row.scope !== userId) {
            throw new CallError("FORBIDDEN", { message: `${id} is another person's client` });
        } else if (row !== undefined) {
            return;
        }

        // register it as the person
        const person = principal.user.reference(Scope.universe.id, userId);
        await objects.mutate(Caller.of(person, now), {
            id: RequestId.create(now),
            calls: [
                client.call("create", {
                    ...client.target({ scope: user.identifier(userId), id }),
                    name: DEVICE_CLIENT_NAME,
                    kind: "native",
                }),
            ],
        });
    },
};
