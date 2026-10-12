import { and, Change, eq, isNotNull, isNull, Snapshot } from "@destack/db";
import type { ObjectStore } from "@destack/object/server";
import type { Controller } from "@destack/service/control";
import { installation, installationRevision } from "@destack/stack/object";
import { BuildBucket } from "@destack/stack/server";
import type { BucketHost } from "@destack/bucket/s3";
import { type Identifier, schema } from "@destack/schema";
import { AlertRuleDefinition, alertRule } from "../object/index.ts";
import { Caller } from "@destack/access";
import { RequestId } from "@destack/service/request";

/** Keep the alert rules each installation's applied revision declares in its space. */
export class DeclarationController implements Controller {
    /** The controller's name. */
    readonly name = "alert-rule-declarations";
    /** The copies whose changes name the installations to reconcile. */
    readonly watches = [installation.table, installationRevision.table];
    /** The object store of the rules, beside copies of the installations and their revisions. */
    readonly #store: ObjectStore;
    /** Open an installation revision's build. */
    readonly #buckets: BucketHost;

    /** Apply the rules the builds of an object store's copied installations declare. */
    constructor(store: ObjectStore, buckets: BucketHost) {
        this.#store = store;
        this.#buckets = buckets;
    }

    /** Name the installation a changed installation or revision belongs to. */
    keys(change: Change): readonly string[] {
        return Change.of(change, installation.table)
            ? [Change.image(change).id]
            : Change.of(change, installationRevision.table)
              ? [Change.image(change).installationId]
              : [];
    }

    /** List the copied installations and those managing rules still kept. */
    async list(): Promise<readonly string[]> {
        // read the installations and the managers of kept rules
        const database = this.#store.database;
        const installed = await database
            .select({ id: installation.table.id })
            .from(installation.table);
        const managing = await database
            .selectDistinct({ id: alertRule.table.managerInstallationId })
            .from(alertRule.table)
            .where(isNotNull(alertRule.table.managerInstallationId));

        return [
            ...new Set([...installed, ...managing].flatMap(({ id }) => (id === null ? [] : [id]))),
        ];
    }

    /** Apply an installation's declared rules and retire those it no longer declares. */
    async reconcile(key: string): Promise<number | undefined> {
        // read the rules the installation's applied revision declares, and the rules it keeps
        const id = schema.identifier("installation").parse(key);
        const declaring = await this.#declaring(id);
        const kept = await this.#store.database
            .select({
                scope: alertRule.table.scope,
                packageId: alertRule.table.managerPackageId,
                name: alertRule.table.managerName,
            })
            .from(alertRule.table)
            .where(
                and(
                    eq(alertRule.table.managerInstallationId, id),
                    isNull(alertRule.table.detachedAt),
                ),
            );

        // apply each declared rule, none while the installation applies no revision
        const declared = declaring === undefined ? [] : declaring.rules;
        const applied =
            declaring === undefined
                ? []
                : declaring.rules.map((rule) =>
                      alertRule.call("apply", {
                          ...alertRule.in(declaring.scope),
                          manager: {
                              installationId: key,
                              packageId: rule.package,
                              name: rule.name,
                          },
                          values: rule.description,
                      }),
                  );

        // retire each kept rule no longer declared
        const calls = [
            ...applied,
            ...kept
                .filter((row) => !declared.some((rule) => rule.name === row.name))
                .map((row) =>
                    alertRule.call("apply", {
                        ...alertRule.in(row.scope),
                        manager: { installationId: key, packageId: row.packageId, name: row.name },
                        values: null,
                    }),
                ),
        ];
        if (calls.length > 0) {
            const store = this.#store;
            const now = store.clock.now();
            await store.mutate(
                Caller.of(store.principal, now),
                { id: RequestId.create(now), calls },
                { component: this.name },
            );
        }

        return undefined;
    }

    /** Read the rules an installation's applied revision declares, with its space. */
    async #declaring(id: Identifier<"installation">) {
        // read the installation's applied revision from the copies
        const [applied] = await this.#store.database
            .select({
                scope: installation.table.scope,
                packageId: installation.table.packageId,
                build: installationRevision.table.build,
            })
            .from(installation.table)
            .innerJoin(
                installationRevision.table,
                eq(installationRevision.table.id, installation.table.appliedRevisionId),
            )
            .where(eq(installation.table.id, id));
        if (applied === undefined) {
            return undefined;
        }

        // read the alert rules its build declares
        const snapshot = Snapshot.live(this.#store.database);
        const { scope, build } = applied;
        const reader = await BuildBucket.read(this.#buckets, snapshot, scope, build.manifest);
        const rules = await reader.declared(
            alertRule.package.id,
            "alert-rule",
            AlertRuleDefinition,
        );

        return { scope: applied.scope, rules };
    }
}
