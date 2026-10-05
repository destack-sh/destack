import { and, Change, eq, isNotNull, isNull } from "@destack/db";
import type { ObjectServer } from "@destack/object/server";
import type { Controller } from "@destack/service/control";
import { installation, installationRevision } from "@destack/space/object";
import type { OpenBuild } from "@destack/space/server";
import { type Identifier, schema } from "@destack/schema";
import { AlertRuleDefinition, alertRule } from "../object/index.ts";

/** Keep the alert rules each installation's applied revision declares in its space, retiring those it no longer declares. */
export class DeclarationController implements Controller {
    /** The controller's name. */
    readonly name = "alert-rule-declarations";
    /** The copies whose changes name the installations to reconcile. */
    readonly watches = [installation.table, installationRevision.table];
    /** The object server keeping the rules beside copies of the installations and their revisions. */
    readonly #server: ObjectServer;
    /** Open an installation revision's build. */
    readonly #open: OpenBuild;

    /** Apply the rules the builds of an object server's copied installations declare. */
    constructor(server: ObjectServer, open: OpenBuild) {
        this.#server = server;
        this.#open = open;
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
        const database = this.#server.database;
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
        const kept = await this.#server.database
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

        // apply each declared rule and retire each kept one no longer declared
        const declared = declaring?.rules ?? [];
        const calls = [
            ...declared.map((rule) => ({
                scope: declaring?.scope ?? "",
                input: {
                    manager: { installationId: key, packageId: rule.package, name: rule.name },
                    values: rule.description,
                },
            })),
            ...kept
                .filter((row) => !declared.some((rule) => rule.name === row.name))
                .map((row) => ({
                    scope: row.scope,
                    input: {
                        manager: { installationId: key, packageId: row.packageId, name: row.name },
                        values: null,
                    },
                })),
        ];
        if (calls.length > 0) {
            await this.#server.executeAsSystem(alertRule, "apply", calls, Date.now());
        }

        return undefined;
    }

    /** Read the rules an installation's applied revision declares with its space, absent while it applies none. */
    async #declaring(id: Identifier<"installation">) {
        // read the installation's applied revision from the copies
        const [applied] = await this.#server.database
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
        const reader = await this.#open(applied.packageId, applied.build, applied.scope);
        const rules = await reader.declared(alertRule.package.id, "alert-rule", AlertRuleDefinition);

        return { scope: applied.scope, rules };
    }
}
