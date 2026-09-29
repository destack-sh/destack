import { none, principal, relation, through, union } from "@destack/access";
import { account, device, DeviceProof, DevicePublicKey, region } from "@destack/account/object";
import {
    and,
    check,
    type Column,
    dialectSQL,
    eq,
    isNull,
    sql,
    unique,
    uniqueIndex,
    type DatabaseConnection,
    type Select,
} from "@destack/db";
import { Condition } from "@destack/db/query";
import { type Call, defineObject, field, method } from "@destack/object";
import { ServerRuntime } from "@destack/package/runtime";
import { schema, type Identifier } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import { Scope } from "@destack/sync";

/** How long a host key authenticates, a year in milliseconds. */
const KEY_LIFETIME_MILLISECONDS = 365 * 24 * 60 * 60 * 1000;

/** The longest proof a host presents, far above the few hundred bytes of an ES256 compact JWS. */
const PROOF_LENGTH = 4096;

/** The longest host name: a DNS label (RFC 1035 2.3.4). */
const HOST_NAME_LENGTH = 63;

/** The characters of a host name, as a character class: lowercase letters, digits and hyphens. */
const HOST_NAME_CHARACTERS = "a-z0-9-";

/** Whether a host accepts new work, drains its existing work, or refuses work. */
const HOST_STATUSES = ["enabled", "draining", "disabled"] as const;

/** A proof by a host key. */
const Proof = schema.string().min(1).max(PROOF_LENGTH);

/** A host's name within its account: a DNS label without leading, trailing or doubled hyphens. */
export const HostName = schema
    .string()
    .regex(new RegExp(`^(?!-)(?!.*--)[${HOST_NAME_CHARACTERS}]{1,${HOST_NAME_LENGTH}}(?<!-)$`));

/** Rename a host within its account. */
const renaming = method({
    permission: "rename",
    input: schema.object({
        /** The new name. */
        name: HostName,
    }),
});

/** Report the host's version and runtimes at a proven contact. */
const seeing = method({
    permission: "rotate",
    input: schema.object({
        /** The Destack version the host runs. */
        version: schema.string().min(1).nullable(),
        /** The runtimes the host can execute. */
        runtimes: schema.array(ServerRuntime),
    }),
});

/** A host key and the proof that the host holds its private half. */
const KeyInput = schema.object({
    /** The public key. */
    publicKey: DevicePublicKey,
    /** A proof of the host by the key's private half. */
    proof: Proof,
});

/** A machine running Destack for an account. */
export const host = defineObject({
    name: "host",
    tier: "global",
    plural: "hosts",
    scope: account,
    represents: principal.host,
    fields: {
        /** The name within the account. */
        name: field.string(HostName),
        /** Whether the host is an account's device or a provider's cloud machine. */
        kind: field.enum(["device", "cloud"]),
        /** The device running a device host. */
        device: field.reference(device, { delete: "restrict" }).optional(),
        /** The region a cloud host serves. */
        region: field.reference(region, { delete: "restrict" }).optional(),
        /** The provider running a cloud host. */
        providerCode: field.string().optional(),
        /** The provider location of a cloud host. */
        location: field.string().optional(),
        /** Whether the host accepts new work, drains existing work, or neither. */
        status: field.enum(HOST_STATUSES).default("enabled"),
        /** The Destack version the host last reported. */
        version: field.string().optional(),
        /** The runtimes the host can execute. */
        runtimes: field.json(schema.array(ServerRuntime)).default([]),
        /** The host's last proven contact, in UTC epoch milliseconds. */
        lastSeenAt: field.time().optional(),
        /** The time the account withdrew the host and its keys. */
        revokedAt: field.time().optional(),
    },
    relations: {
        tenant: { subjects: [account.members("member")], grantedBy: "share" },
        self: { subjects: [principal.host], grantedBy: null },
        peer: { subjects: [principal.host.all()], grantedBy: null },
    },
    permissions: {
        read: union(relation("tenant"), relation("self")),
        verify: union(relation("tenant"), relation("self"), relation("peer")),
        enroll: none(),
        rotate: relation("self"),
        rename: relation("self"),
        revoke: relation("self"),
        update: none(),
        share: none(),
    },
    reserved: ["rotate"],
    methods: {
        get: method.get("read"),
        list: method.list("read"),
        enroll: method
            .create("enroll", {
                fields: [
                    "name",
                    "kind",
                    "device",
                    "region",
                    "providerCode",
                    "location",
                    "version",
                    "runtimes",
                ],
                input: KeyInput,
                creation: () => ({
                    relationships: [
                        { relation: "peer", subject: principal.host.reference("*", "*") },
                    ],
                }),
            })
            .handle(enroll),
        revoke: method({ permission: "revoke" }).handle(revoke),
        enable: method({ permission: "revoke" }).handle((call) => Host.transition(call, "enabled")),
        drain: method({ permission: "revoke" }).handle((call) => Host.transition(call, "draining")),
        disable: method({ permission: "revoke" }).handle((call) =>
            Host.transition(call, "disabled"),
        ),
        see: seeing.handle((call) =>
            // record the host's proven contact and what it runs now
            call.revise({
                lastSeenAt: call.now,
                version: call.input.version,
                runtimes: call.input.runtimes,
            }),
        ),
        rename: renaming.handle(async (call) => {
            // refuse renaming a withdrawn host
            Host.requireActive(call.target as Host);

            return call.revise({ name: HostName.parse(call.input.name) });
        }),
    },
    constraints: (host) => [
        unique("host_device_id").on(host.device, host.id),
        unique("host_scope_id").on(host.scope, host.id),
        uniqueIndex("host_scope_name")
            .on(host.scope, host.name)
            .where(sql`${host.revokedAt} IS NULL`),
        check("host_name", hostNameCheck(host.name)),
        check(
            "host_location",
            sql`${host.location} IS NULL OR (${host.providerCode} IS NOT NULL AND length(${host.location}) > 0)`,
        ),
        check(
            "host_kind",
            sql`(${host.kind} = 'device' AND ${host.device} IS NOT NULL AND ${host.region} IS NULL) OR (${host.kind} = 'cloud' AND ${host.device} IS NULL)`,
        ),
    ],
});

/** A host's revocable authentication key, registered with a proof by its private half. */
export const hostKey = defineObject({
    name: "host-key",
    tier: "global",
    plural: "hostKeys",
    scope: account,
    nested: { in: host, receive: "rotate", delete: "cascade" },
    fields: {
        /** The public key. */
        publicKey: field.json(DevicePublicKey),
        /** The RFC 7638 JWK thumbprint that identifies the key in the host's proofs. */
        thumbprint: field.string(schema.string().min(1)),
        /** The time the host proved possession of the private key. */
        verifiedAt: field.time(),
        /** The time the key stops authenticating. */
        expiresAt: field.time(),
        /** The time the key was withdrawn. */
        revokedAt: field.time().optional(),
        /** The time disabling the host suspended the key. */
        suspendedAt: field.time().optional(),
    },
    permissions: {
        read: through("parent", "verify"),
        rotate: through("parent", "rotate"),
        revoke: through("parent", "revoke"),
    },
    reserved: ["rotate"],
    methods: {
        get: method.get("read"),
        list: method.list("read"),
        create: method
            .create("rotate", { fields: ["publicKey"], input: KeyInput.pick({ proof: true }) })
            .handle(register),
        enroll: method.create(null, { isSystem: true }),
        suspend: method({ permission: null, isSystem: true }).handle((call) =>
            call.revise({ suspendedAt: call.now }),
        ),
        resume: method({ permission: null, isSystem: true }).handle((call) =>
            call.revise({ suspendedAt: null }),
        ),
        revoke: method({ permission: "revoke" }).handle(async (call) => {
            // refuse withdrawing a key twice
            const target = call.target as HostKey;
            if (target.revokedAt !== null) {
                throw new ServiceError("CONFLICT", { message: "host key is revoked" });
            }

            return call.revise({ revokedAt: call.now });
        }),
    },
    constraints: (key) => [
        unique("host_key_thumbprint").on(key.thumbprint),
        unique("host_key_host_id").on(key.parentId, key.id),
        check("host_key_expiry", sql`${key.expiresAt} > ${key.createdAt}`),
        check(
            "host_key_verification",
            sql`${key.verifiedAt} >= ${key.createdAt} AND ${key.verifiedAt} < ${key.expiresAt}`,
        ),
    ],
});

/** Enroll a host under the identifier its proof carries, with the key it proves. */
async function enroll(call: Call, next: (call?: Call) => Promise<unknown>): Promise<unknown> {
    // require the right to serve a region the host would act for
    if (call.input.region !== undefined && call.input.region !== null) {
        const served = region.reference(
            Scope.universe.id,
            schema.string().parse(call.input.region),
        );
        const authorization = call.served();
        await authorization.require(
            authorization.authorizer.policy(served).permission("serve"),
            served,
        );
    }

    // require the host's proof by the key it registers
    const input = KeyInput.parse({ publicKey: call.input.publicKey, proof: call.input.proof });
    if (call.id === undefined) {
        throw new ServiceError("BAD_REQUEST", {
            message: "a host enrolls under the identifier its proof carries",
        });
    }
    const proof = DeviceProof.read(input.proof);
    await proof.verify(input.publicKey, call.id, call.now);
    await proof.consume(call.database, call.now);

    // create the host under the first free variant of its preferred name, then its first key
    const name = await freeName(call.database, call.scope, String(call.input.name));
    const created = (await next(
        call.with({ input: { ...call.input, name, lastSeenAt: call.now } }),
    )) as Host;
    await call.invoke(hostKey, "enroll", {
        parentId: created.id,
        ...(await keyRow(input.publicKey, call.now)),
    });

    return created;
}

/** Take a preferred host name, or its first free numbered variant in the account, as `laptop-2`. */
async function freeName(
    database: DatabaseConnection,
    scope: string,
    preferred: string,
): Promise<string> {
    // read the names of the account's standing hosts
    const rows = await database
        .select({ name: host.table.name })
        .from(host.table)
        .where(and(eq(host.table.scope, scope as never), isNull(host.table.revokedAt)));
    const taken = new Set(rows.map((row) => row.name));

    // number the name past the taken ones within a DNS label
    let name = preferred;
    for (let number = 2; taken.has(name); number++) {
        const suffix = `-${number}`;
        name = `${preferred.slice(0, HOST_NAME_LENGTH - suffix.length)}${suffix}`;
    }

    return name;
}

/** Register another key a host proves it holds. */
async function register(call: Call, next: (call?: Call) => Promise<unknown>): Promise<unknown> {
    // refuse keys of a withdrawn host
    const [owner] = await call.database
        .select()
        .from(host.table)
        .where(and(eq(host.table.id, call.parent()!.id as Host["id"]), host.inScope(call.scope)));
    if (owner === undefined) {
        throw new ServiceError("NOT_FOUND", { message: "host not found" });
    }
    Host.requireActive(owner);

    // verify the proof with the written key, used once, and mark the host seen
    const publicKey = DevicePublicKey.parse(call.input.publicKey);
    const proof = DeviceProof.read(Proof.parse(call.input.proof));
    await proof.verify(publicKey, owner.id, call.now);
    await proof.consume(call.database, call.now);
    await call.invoke(host, "see", {
        id: owner.id,
        version: owner.version,
        runtimes: owner.runtimes,
    });

    // revoke the host's other active keys, then keep the proven key with its thumbprint
    await revokeKeys(call, owner.id);

    return next(call.with({ input: { ...call.input, ...(await keyRow(publicKey, call.now)) } }));
}

/** Withdraw a host, ending its keys with it. */
async function revoke(call: Call): Promise<unknown> {
    // end the host's active keys
    const target = call.target as Host;
    Host.requireActive(target);
    await revokeKeys(call, target.id);

    // withdraw the host
    return call.revise({ revokedAt: call.now });
}

/** Revoke a host's active keys through the key's own method, recording each revocation. */
function revokeKeys(call: Call, hostId: Host["id"]): Promise<void> {
    return invokeKeys(call, hostId, "revoke");
}

/** Invoke a method on each of a host's unrevoked keys, recording each call. */
async function invokeKeys(
    call: Call,
    hostId: Host["id"],
    name: "revoke" | "suspend" | "resume",
): Promise<void> {
    const keys = await call.database
        .select({ id: hostKey.table.id })
        .from(hostKey.table)
        .where(and(eq(hostKey.table.parentId, hostId), isNull(hostKey.table.revokedAt)));
    for (const key of keys) {
        await call.invoke(hostKey, name, { id: key.id });
    }
}

/** Require a host name to be a DNS label in both dialects' check syntax. */
function hostNameCheck(name: Column) {
    // spell the label's length, characters and hyphen rules as SQL
    const length = sql.raw(String(HOST_NAME_LENGTH));
    const characters = sql.raw(`'[^${HOST_NAME_CHARACTERS}]'`);
    const glob = sql.raw(`'*[^${HOST_NAME_CHARACTERS}]*'`);
    const shape = sql`length(${name}) BETWEEN 1 AND ${length} AND ${name} NOT LIKE '-%' AND ${name} NOT LIKE '%-' AND ${name} NOT LIKE '%--%'`;

    return dialectSQL({
        sqlite: sql`${shape} AND ${name} NOT GLOB ${glob}`,
        postgresql: sql`${shape} AND (${name} COLLATE "C") !~ ${characters}`,
    });
}

/** Describe a proven key's row. */
async function keyRow(publicKey: DevicePublicKey, now: number) {
    return {
        publicKey,
        thumbprint: await DeviceProof.thumbprint(publicKey),
        verifiedAt: now,
        expiresAt: now + KEY_LIFETIME_MILLISECONDS,
    };
}

/** A persisted host record. */
export type Host = Select<typeof host.table>;

/** The checks and reads of hosts that methods and other packages route by. */
export const Host = {
    /** Refuse changing a withdrawn host. */
    requireActive(target: { readonly revokedAt: number | null }): void {
        if (target.revokedAt !== null) {
            throw new ServiceError("CONFLICT", { message: "host is revoked" });
        }
    },

    /** Move an active host to a status, suspending its keys while it is disabled. */
    async transition(call: Call, status: (typeof HOST_STATUSES)[number]): Promise<unknown> {
        // refuse a withdrawn host
        const target = call.target as Host;
        Host.requireActive(target);

        // suspend the keys on disabling and resume them on leaving disabled
        if (status === "disabled" && target.status !== "disabled") {
            await invokeKeys(call, target.id, "suspend");
        } else if (status !== "disabled" && target.status === "disabled") {
            await invokeKeys(call, target.id, "resume");
        }

        return call.revise({ status });
    },

    /** Find an account's standing host by its name. */
    async find(
        database: DatabaseConnection,
        accountId: string,
        name: string,
    ): Promise<Identifier<"host"> | undefined> {
        const [found] = await database
            .select({ id: host.table.id })
            .from(host.table)
            .where(
                and(
                    host.inScope(accountId),
                    eq(host.table.name, name),
                    isNull(host.table.revokedAt),
                ),
            );

        return found?.id;
    },
};

/** A host authentication key. */
export type HostKey = Select<typeof hostKey.table>;

/** Reads of host keys that proofs and tunnels authenticate by. */
export const HostKey = {
    /** Match the keys standing at a time: unrevoked and unexpired. */
    standing(now: number): Condition {
        return Condition.all(Condition.missing("revokedAt"), Condition.gt("expiresAt", now));
    },
    /** Match the keys authenticating at a time: standing and not suspended by a disabled host. */
    authenticates(now: number): Condition {
        return Condition.all(HostKey.standing(now), Condition.missing("suspendedAt"));
    },
    /** Read the keys a condition matches, with their hosts. */
    select(database: DatabaseConnection, where: Condition) {
        return database
            .select({
                id: hostKey.table.id,
                publicKey: hostKey.table.publicKey,
                hostId: host.table.id,
                accountId: host.table.scope,
                regionId: host.table.region,
                suspendedAt: hostKey.table.suspendedAt,
            })
            .from(hostKey.table)
            .innerJoin(host.table, eq(host.table.id, hostKey.table.parentId))
            .where(Condition.render(where, Condition.bind(hostKey.table)))
            .orderBy(hostKey.table.createdAt);
    },
};
