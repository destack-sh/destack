import {
    implement,
    Server,
    type ServerOptions,
    type ServiceContext,
} from "@destack/service/server";
import { createProcedureAudit } from "@destack/audit/server";
import { vaultService, VaultScope } from "../service/index.ts";
import { VaultStore } from "./store.ts";
import { ServiceError } from "@destack/service/error";
import { vaultPackage } from "../audit/index.ts";
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

/** Regional secret procedures hosted through the shared Server lifecycle. */
export class VaultServer {
    /** Typed procedures with transaction-bound secret authorization. */
    readonly router: ReturnType<typeof createRouter>;
    /** Storage and separately provisioned encryption keys. */
    readonly store: VaultStore;

    /** Attach prepared regional storage to the service procedures. */
    constructor(store: VaultStore) {
        this.store = store;
        this.router = createRouter(store);
    }

    /** Start authenticated hosting with durable auditing and uncached responses. */
    start(options: VaultServerOptions): Promise<Server> {
        return Server.start({
            ...options,
            router: this.router,
            audience: vaultPackage.id,
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
                // enforce persisted grants inside each store transaction
                context.requireCaller();
            },
            audit: createProcedureAudit(async ({ context }) => {
                // attribute attempts only to the credential's verified tenant
                const scope =
                    context.authenticationError === undefined
                        ? context.caller?.authentication.scope
                        : undefined;
                const selected = scope
                    ? await this.store.database
                          .select({ id: space.id, accountId: space.accountId })
                          .from(space)
                          .where(sql`${space.id} = ${scope}`)
                          .get()
                    : undefined;

                return vaultAudit(context, this.store.database, selected);
            }),
            responseHeaders: { "Cache-Control": "no-store", Pragma: "no-cache" },
        });
    }
}

/** Host identity verification, installation policy, resources and lifecycle. */
export type VaultServerOptions = Omit<
    ServerOptions,
    "router" | "audience" | "target" | "authorize" | "audit" | "responseHeaders"
>;

/** Connect every procedure to transaction-bound storage operations. */
function createRouter(store: VaultStore) {
    const implementation = implement(vaultService)
        .$context<ServiceContext>()
        .use(async ({ context, next }, input) => {
            // derive audit tenancy from persisted regional ownership
            const { spaceId } = targetSchema.parse(input);
            const selected = await store.database
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
                    audit: vaultAudit(context, store.database, selected),
                },
            });
        });
    const router = implementation.router({
        vault: {
            get: implementation.vault.get.handler(({ input, context }) =>
                store.getVault(input, context),
            ),
            list: implementation.vault.list.handler(({ input, context }) =>
                store.listVaults(input, context),
            ),
        },
        secret: {
            purge: implementation.secret.purge.handler(({ input, context }) =>
                store.purge(input, context),
            ),
            create: implementation.secret.create.handler(({ input, context }) =>
                store.create(input, context),
            ),
            get: implementation.secret.get.handler(({ input, context }) =>
                store.get(input, context),
            ),
            list: implementation.secret.list.handler(({ input, context }) =>
                store.list(input, context),
            ),
            update: implementation.secret.update.handler(({ input, context }) =>
                store.change("update", input, context),
            ),
            disable: implementation.secret.disable.handler(({ input, context }) =>
                store.change("disable", input, context),
            ),
            enable: implementation.secret.enable.handler(({ input, context }) =>
                store.change("enable", input, context),
            ),
            delete: implementation.secret.delete.handler(({ input, context }) =>
                store.change("delete", input, context),
            ),
            restore: implementation.secret.restore.handler(({ input, context }) =>
                store.change("restore", input, context),
            ),
        },
        version: {
            rewrap: implementation.version.rewrap.handler(({ input, context }) =>
                store.rewrap(input, context),
            ),
            get: implementation.version.get.handler(({ input, context }) =>
                store.getVersion(input, context),
            ),
            list: implementation.version.list.handler(({ input, context }) =>
                store.listVersions(input, context),
            ),
            read: implementation.version.read.handler(({ input, context }) =>
                store.read(input, context),
            ),
            write: implementation.version.write.handler(({ input, context }) =>
                store.write(input, context),
            ),
            promote: implementation.version.promote.handler(({ input, context }) =>
                store.promote(input, context),
            ),
            disable: implementation.version.disable.handler(({ input, context }) =>
                store.changeVersion("disable", input, context),
            ),
            enable: implementation.version.enable.handler(({ input, context }) =>
                store.changeVersion("enable", input, context),
            ),
            destroy: implementation.version.destroy.handler(({ input, context }) =>
                store.changeVersion("destroy", input, context),
            ),
        },
        request: {
            rewrap: implementation.request.rewrap.handler(({ input, context }) =>
                store.rewrapRequests(input, context),
            ),
        },
    });

    return router;
}
