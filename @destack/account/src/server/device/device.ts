import { and, eq, gt, isNull } from "@destack/db";
import type { Call } from "@destack/object";
import { schema } from "@destack/schema";
import { SessionKey } from "@destack/service/authentication";
import { ServiceError } from "@destack/service/error";
import { session } from "../../object/authentication.ts";
import * as base from "../../object/device.ts";
import { DeviceProof, DevicePublicKey, type Device } from "../../object/device.ts";
import { requireUnrevoked } from "../../object/revocation.ts";

/** How long a device key authenticates, a year in milliseconds. */
const KEY_LIFETIME_MILLISECONDS = 365 * 24 * 60 * 60 * 1000;

/** One call of a device method. */
type DeviceCall = Call<typeof base.device.table>;

/** One call of a device key method. */
type DeviceKeyCall = Call<typeof base.deviceKey.table>;

/** Devices on the server. */
export const device = base.device.handle({ create: register, attach, revoke });

/** Device keys on the server; registering one verifies the device's proof. */
export const deviceKey = base.deviceKey.handle({ create: registerKey });

/** Register a device as seen now, attached to the session registering it. */
async function register(
    call: DeviceCall,
    next: (call?: DeviceCall) => Promise<unknown>,
): Promise<unknown> {
    // require a signed-in session before creating the device
    const sessionId = callingSession(call);
    const created = (await next(
        call.with({ input: { ...call.input, lastSeenAt: call.now } }),
    )) as Device;
    await call.invoke(session, "attach", { id: sessionId, deviceId: created.id });

    return created;
}

/** Attach the calling session to a device by a proof. */
async function attach(call: DeviceCall): Promise<unknown> {
    // find the device's active key the proof was signed with
    const target = call.target as Device;
    requireUnrevoked(target, "device");
    const proof = DeviceProof.read(schema.string().parse(call.input.proof));
    const [key] = await call.database
        .select({ publicKey: base.deviceKey.table.publicKey })
        .from(base.deviceKey.table)
        .where(
            and(
                eq(base.deviceKey.table.parentId, target.id),
                eq(base.deviceKey.table.thumbprint, proof.thumbprint),
                isNull(base.deviceKey.table.revokedAt),
                gt(base.deviceKey.table.expiresAt, call.now),
            ),
        );
    if (key === undefined) {
        throw new ServiceError("FORBIDDEN", {
            message: "device proof refers to no active key of the device",
        });
    }

    // verify the proof, then attach the calling session
    await proof.verify(key.publicKey, target.id, call.now);
    await proof.consume(call.database, call.now);
    await call.invoke(session, "attach", { id: callingSession(call), deviceId: target.id });

    return call.revise({ lastSeenAt: call.now });
}

/** Withdraw a device, ending its keys and sessions with it. */
async function revoke(call: DeviceCall): Promise<unknown> {
    // read the device's active keys and sessions
    const target = call.target as Device;
    requireUnrevoked(target, "device");
    const keys = await call.database
        .select({ id: base.deviceKey.table.id })
        .from(base.deviceKey.table)
        .where(
            and(
                eq(base.deviceKey.table.parentId, target.id),
                isNull(base.deviceKey.table.revokedAt),
            ),
        );
    const sessions = await call.database
        .select({ id: session.table.id })
        .from(session.table)
        .where(and(eq(session.table.deviceId, target.id), isNull(session.table.revokedAt)));

    // end each, then the device
    for (const key of keys) {
        await call.invoke(base.deviceKey, "revoke", { id: key.id });
    }
    for (const ended of sessions) {
        await call.invoke(session, "revoke", { id: ended.id });
    }

    return call.revise({ revokedAt: call.now });
}

/** Register a key a device proves it holds. */
async function registerKey(
    call: DeviceKeyCall,
    next: (call?: DeviceKeyCall) => Promise<unknown>,
): Promise<unknown> {
    // refuse keys of a withdrawn device
    const parent = call.parent()!;
    const [owner] = await call.database
        .select()
        .from(base.device.table)
        .where(
            and(eq(base.device.table.id, parent.id as Device["id"]), device.inScope(call.scope)),
        );
    if (owner === undefined) {
        throw new ServiceError("NOT_FOUND", { message: "device not found" });
    }
    requireUnrevoked(owner, "device");

    // verify the proof with the written key and mark the device seen
    const publicKey = DevicePublicKey.parse(call.input.publicKey);
    const proof = DeviceProof.read(schema.string().parse(call.input.proof));
    await proof.verify(publicKey, owner.id, call.now);
    await proof.consume(call.database, call.now);
    await call.invoke(base.device, "see", { id: owner.id });

    // keep the proven key with its thumbprint
    return next(
        call.with({
            input: {
                ...call.input,
                thumbprint: await DeviceProof.thumbprint(publicKey),
                verifiedAt: call.now,
                expiresAt: call.now + KEY_LIFETIME_MILLISECONDS,
            },
        }),
    );
}

/** Read the session a call authenticated with, and refuse a call without one. */
function callingSession(call: DeviceCall): string {
    const id = SessionKey.id(call.authorization!.access.context.session, "session");
    if (id === undefined) {
        throw new ServiceError("FORBIDDEN", { message: "a device attaches a signed-in session" });
    }

    return id;
}
