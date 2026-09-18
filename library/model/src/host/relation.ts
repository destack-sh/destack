import { defineRelationsPart } from "@destack/db";
import { account } from "../account/account.ts";
import { hostAccess } from "../host/access.ts";
import { deviceKey } from "../host/key.ts";
import { device } from "../host/device.ts";
import { host } from "../host/host.ts";
import { region } from "../host/region.ts";
import { spaceRegistration } from "../host/registration.ts";
import { serverConnection } from "../host/server.ts";
import { tunnel } from "../host/tunnel.ts";

/** Query relationships for host records. */
export const hostRelations = defineRelationsPart({
    host,
    hostAccess,
    account,
    device,
    deviceKey,
    region,
    serverConnection,
    spaceRegistration,
    tunnel,
}, (relation) => ({
    hostAccess: {
        host: relation.one.host({
            from: [relation.hostAccess.hostId],
            to: [relation.host.id],
            optional: false,
        }),
        account: relation.one.account({
            from: [relation.hostAccess.accountId],
            to: [relation.account.id],
            optional: false,
        }),
    },
    deviceKey: {
        device: relation.one.device({
            from: [relation.deviceKey.deviceId],
            to: [relation.device.id],
            optional: false,
        }),
    },
    device: {
        account: relation.one.account({
            from: [relation.device.accountId],
            to: [relation.account.id],
            optional: false,
        }),
    },
    host: {
        device: relation.one.device({
            from: [relation.host.accountId, relation.host.deviceId],
            to: [relation.device.accountId, relation.device.id],
            optional: true,
        }),
        account: relation.one.account({
            from: [relation.host.accountId],
            to: [relation.account.id],
            optional: false,
        }),
        region: relation.one.region({
            from: [relation.host.regionId],
            to: [relation.region.id],
            optional: true,
        }),
    },
    spaceRegistration: {
        serverConnection: relation.one.serverConnection({
            from: [relation.spaceRegistration.serverConnectionId],
            to: [relation.serverConnection.id],
            optional: true,
        }),
    },
    tunnel: {
        host: relation.one.host({
            from: [relation.tunnel.deviceId, relation.tunnel.hostId],
            to: [relation.host.deviceId, relation.host.id],
            optional: false,
        }),
        key: relation.one.deviceKey({
            from: [relation.tunnel.deviceId, relation.tunnel.deviceKeyId],
            to: [relation.deviceKey.deviceId, relation.deviceKey.id],
            optional: false,
        }),
        device: relation.one.device({
            from: [relation.tunnel.deviceId],
            to: [relation.device.id],
            optional: false,
        }),
    },
}));
