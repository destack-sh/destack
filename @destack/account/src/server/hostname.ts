import { schema } from "@destack/schema";
import { CallError } from "@destack/object";
import { account } from "../object/account.ts";
import { Domain, domain } from "../object/domain.ts";
import { hostname } from "../object/hostname.ts";

/** Implement hostnames. */
export function implementHostname(): typeof hostname {
    return hostname.implement({
        methods: {
            create: async (call, next) => {
                // read the account's owner
                const accountId = schema.identifier("account").parse(call.scope);
                const row = await call.read(account).findFirst({ where: { id: accountId } });
                if (row === undefined) {
                    throw new CallError("NOT_FOUND", { message: `no account ${accountId}` });
                }

                // require a verified domain of the account or its owner covering the name
                const domains = await call.read(domain).findMany({
                    where: {
                        scope: { in: [call.scope, row.scope] },
                        verifiedAt: { isNotNull: true },
                    },
                });
                if (!domains.some((entry) => Domain.covers(entry.name, call.input.name))) {
                    throw new CallError("PRECONDITION_FAILED", {
                        message: `verify a domain of ${call.input.name} first`,
                    });
                }

                return next();
            },
        },
    });
}
