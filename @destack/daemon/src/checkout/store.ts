import { eq, gt, asc, type DatabaseConnection } from "@destack/db";
import type { AuditRecorder } from "@destack/audit";
import { Page } from "@destack/service/page";
import { identifier } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import { v7 } from "uuid";
import { checkout } from "../stack/index.ts";
import type { Checkout } from "../service/checkout.ts";
import { registerCheckout, unregisterCheckout } from "../audit/index.ts";

/** Public checkout fields mapped from their persisted columns. */
const COLUMNS = {
    id: checkout.id,
    repositoryId: checkout.repositoryId,
    directory: checkout.path,
    createdAt: checkout.createdAt,
};

/** Persist local repository registrations and their audit events. */
export class CheckoutStore {
    /** Database containing host registrations. */
    readonly database: DatabaseConnection;

    /** Retain the initialized host database. */
    constructor(database: DatabaseConnection) {
        this.database = database;
    }

    /** List registrations in stable identifier order. */
    async list(input: { limit: number; cursor?: string }) {
        const page = new Page(input, ["checkout"], identifier("checkout"));
        const entries = await this.database
            .select(COLUMNS)
            .from(checkout)
            .where(page.after === undefined ? undefined : gt(checkout.id, page.after))
            .orderBy(asc(checkout.id))
            .limit(page.limit + 1);

        return page.result(entries, (entry) => entry.id);
    }

    /** Read a registered working directory. */
    async get(id: Checkout["id"]): Promise<Checkout> {
        const entry = await this.database
            .select(COLUMNS)
            .from(checkout)
            .where(eq(checkout.id, id))
            .get();
        if (!entry) {
            throw new ServiceError("NOT_FOUND", { message: "checkout is not registered" });
        }

        return entry;
    }

    /** Retain one registration per canonical directory, recording its first insertion. */
    async register(
        directory: string,
        repositoryId: Checkout["repositoryId"],
        audit: AuditRecorder<DatabaseConnection>,
    ): Promise<Checkout> {
        return await this.database.transaction(async (transaction) => {
            // reuse an identical registration without changing its identity
            const existing = await transaction
                .select(COLUMNS)
                .from(checkout)
                .where(eq(checkout.path, directory))
                .get();
            if (existing) {
                if (existing.repositoryId !== repositoryId) {
                    throw new ServiceError("CONFLICT", {
                        message: "checkout has a different repository registration",
                    });
                }

                return existing;
            }

            // commit the registration and its audit event together
            const id = `checkout-${v7()}` as Checkout["id"];
            const createdAt = Date.now();
            await transaction
                .insert(checkout)
                .values({ id, repositoryId, path: directory, createdAt });
            await audit.record(transaction, registerCheckout, {
                targets: { checkout: { type: "checkout", id } },
                details: { directory, repositoryId },
                outcome: "success",
            });

            return { id, repositoryId, directory, createdAt };
        });
    }

    /** Remove only the registration, leaving repository files untouched. */
    async unregister(id: Checkout["id"], audit: AuditRecorder<DatabaseConnection>): Promise<void> {
        await this.database.transaction(async (transaction) => {
            // acknowledge repeated removal without producing duplicate audit records
            const entry = await transaction
                .delete(checkout)
                .where(eq(checkout.id, id))
                .returning(COLUMNS)
                .get();
            if (!entry) {
                return;
            }
            await audit.record(transaction, unregisterCheckout, {
                targets: { checkout: { type: "checkout", id } },
                details: { directory: entry.directory, repositoryId: entry.repositoryId },
                outcome: "success",
            });
        });
    }
}
