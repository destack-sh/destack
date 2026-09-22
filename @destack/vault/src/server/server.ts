import {
    writeVersion,
    getVersion,
    listVersions,
    readVersion,
    promoteVersion,
    changeVersion,
    rewrapVersion,
} from "../secret/version.ts";
import { rewrapRequests } from "../vault/request.ts";
import {
    implement,
    type ServiceImplementation,
    type ServiceContext,
} from "@destack/service/server";
import { createProcedureAudit } from "@destack/audit/server";
import { vaultService, VaultScope } from "../service/index.ts";
import { Vault } from "../vault/index.ts";
import { ServiceError } from "@destack/service/error";
import { vaultAudit } from "./context.ts";
import { identifier } from "@destack/schema";
import { space } from "@destack/model/regional";
import { eq, sql } from "@destack/db";

/** Space and object selectors shared by the vault procedures. */
const targetSchema = VaultScope.extend({
    /** Exact secret selected by secret and version operations. */
    secretId: identifier("secret").optional(),
    /** Vault selected by collection and creation operations. */
    vaultId: identifier("resource").optional(),
}).strip();

/** Implement regional secret procedures and transaction-bound authorization. */
export function implementService(vault: Vault): ServiceImplementation {
    return {
        router: createRouter(vault),
        target: async (call) => {
            const selected = targetSchema.safeParse(call.input);
            if (!selected.success) {
                throw new ServiceError("BAD_REQUEST");
            }

            return {
                scope: selected.data.spaceId,
                id: selected.data.secretId ?? selected.data.vaultId ?? selected.data.spaceId,
            };
        },
        authorize: async ({ context }) => {
            // enforce persisted grants inside each vault transaction
            context.requireCaller();
        },
        audit: createProcedureAudit(async ({ context }) => {
            // attribute attempts only to the credential's verified tenant
            const scope =
                context.authenticationError === undefined
                    ? context.caller?.authentication.scope
                    : undefined;
            const selected = scope
                ? await vault.database
                      .select({ id: space.id, accountId: space.accountId })
                      .from(space)
                      .where(sql`${space.id} = ${scope}`)
                      .get()
                : undefined;

            return vaultAudit(context, vault.database, selected);
        }),
        responseHeaders: { "Cache-Control": "no-store", Pragma: "no-cache" },
    };
}

/** Connect every procedure to transaction-bound storage operations. */
function createRouter(vault: Vault) {
    const implementation = implement(vaultService)
        .$context<ServiceContext>()
        .use(async ({ context, next }, input) => {
            // derive audit tenancy from persisted regional ownership
            const { spaceId } = targetSchema.parse(input);
            const selected = await vault.database
                .select({ id: space.id, accountId: space.accountId })
                .from(space)
                .where(eq(space.id, spaceId))
                .get();
            if (!selected) {
                throw new ServiceError("NOT_FOUND");
            }

            return next({
                context: {
                    caller: context.requireCaller(),
                    audit: vaultAudit(context, vault.database, selected),
                },
            });
        });
    const router = implementation.router({
        vault: {
            get: implementation.vault.get.handler(({ input, context }) =>
                vault.getVault(input, context),
            ),
            list: implementation.vault.list.handler(({ input, context }) =>
                vault.listVaults(input, context),
            ),
        },
        secret: {
            purge: implementation.secret.purge.handler(({ input, context }) =>
                vault.purge(input, context),
            ),
            create: implementation.secret.create.handler(({ input, context }) =>
                vault.create(input, context),
            ),
            get: implementation.secret.get.handler(({ input, context }) =>
                vault.get(input, context),
            ),
            list: implementation.secret.list.handler(({ input, context }) =>
                vault.list(input, context),
            ),
            update: implementation.secret.update.handler(({ input, context }) =>
                vault.change("update", input, context),
            ),
            disable: implementation.secret.disable.handler(({ input, context }) =>
                vault.change("disable", input, context),
            ),
            enable: implementation.secret.enable.handler(({ input, context }) =>
                vault.change("enable", input, context),
            ),
            delete: implementation.secret.delete.handler(({ input, context }) =>
                vault.change("delete", input, context),
            ),
            restore: implementation.secret.restore.handler(({ input, context }) =>
                vault.change("restore", input, context),
            ),
        },
        version: {
            rewrap: implementation.version.rewrap.handler(({ input, context }) =>
                rewrapVersion(vault, input, context),
            ),
            get: implementation.version.get.handler(({ input, context }) =>
                getVersion(vault, input, context),
            ),
            list: implementation.version.list.handler(({ input, context }) =>
                listVersions(vault, input, context),
            ),
            read: implementation.version.read.handler(({ input, context }) =>
                readVersion(vault, input, context),
            ),
            write: implementation.version.write.handler(({ input, context }) =>
                writeVersion(vault, input, context),
            ),
            promote: implementation.version.promote.handler(({ input, context }) =>
                promoteVersion(vault, input, context),
            ),
            disable: implementation.version.disable.handler(({ input, context }) =>
                changeVersion(vault, "disable", input, context),
            ),
            enable: implementation.version.enable.handler(({ input, context }) =>
                changeVersion(vault, "enable", input, context),
            ),
            destroy: implementation.version.destroy.handler(({ input, context }) =>
                changeVersion(vault, "destroy", input, context),
            ),
        },
        request: {
            rewrap: implementation.request.rewrap.handler(({ input, context }) =>
                rewrapRequests(vault, input, context),
            ),
        },
    });

    return router;
}
