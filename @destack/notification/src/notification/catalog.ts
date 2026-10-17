import type { MethodContext } from "@destack/object";
import { declaration, type InstallationRevision } from "@destack/stack/object";
import { principal } from "@destack/access";
import { user } from "@destack/account/object";
import { Catalog, Localization, type LocaleTag, Message, SOURCE_LOCALE } from "@destack/locale";
import type { Subject } from "@destack/sync";
import { Package, PackageId } from "@destack/package";
import { defineSchema, Duration, fromJsonSchema, present, schema } from "@destack/schema";
import { CallError } from "@destack/runtime/error";
import { NotificationMetadata } from "../declare/notification.ts";
import { Action, NotificationContentDefinition } from "./content.ts";
import { NotificationType } from "./notification.ts";

/** A notification as manifests describe it. */
export const NotificationDescription = defineSchema(
    NotificationMetadata.extend({
        /** The declaring package. */
        package: Package,
        /** The payload's JSON Schema. */
        payload: schema.jsonObject(),
        /** The content every channel shows, its arguments the payload's fields. */
        content: NotificationContentDefinition,
        /** The line summarizing a thread's notifications, its argument the count. */
        summary: Message,
        /** The actions, the first the default. */
        actions: schema.array(Action),
        /** How long an unread notification collapses later ones of its thread. */
        collapse: Duration.schema.exactOptional(),
    }),
);
/** A declared notification as manifests describe it. */
export type NotificationDescription = schema.Infer<typeof NotificationDescription>;

/** The notifications a release declares, with the catalogs its build ships. */
export class NotificationCatalog {
    /** The declaring package. */
    readonly packageId: PackageId;
    /** The declared notifications by name. */
    readonly #notifications: ReadonlyMap<string, NotificationType>;
    /** The catalogs translating the release's messages and its unpublished dependencies'. */
    readonly #catalogs: readonly Catalog[];

    /** Read the notifications a revision declares, with its translations. */
    static async read(
        context: Pick<MethodContext, "read">,
        revision: InstallationRevision,
        packageId: PackageId,
    ): Promise<NotificationCatalog> {
        const declared = await context.read(declaration).findMany({
            where: {
                revisionId: revision.id,
                kindPackageId: import.meta.destack.package.id,
                kind: "notification",
            },
        });
        const descriptions = declared.map((row) => NotificationDescription.parse(row.description));

        return NotificationCatalog.of(packageId, descriptions, revision.catalogs);
    }

    /** Create a release's notifications with its catalogs. */
    constructor(
        owner: PackageId,
        notifications: readonly NotificationType[],
        catalogs: readonly Catalog[],
    ) {
        this.packageId = owner;
        this.#notifications = new Map(
            notifications.map((notification) => [notification.name, notification]),
        );
        this.#catalogs = catalogs;
    }

    /** Read the notifications a revision's declarations describe, with its translations. */
    static of(
        owner: PackageId,
        descriptions: readonly NotificationDescription[],
        catalogs: readonly Catalog[],
    ): NotificationCatalog {
        // rebuild each notification's payload schema from its JSON Schema
        const notifications = descriptions.map((description) => {
            const { package: declaring, payload, actions, ...definition } = description;

            return new NotificationType(
                declaring,
                { ...definition, payload: fromJsonSchema(payload) },
                actions,
            );
        });

        return new NotificationCatalog(owner, notifications, catalogs);
    }

    /** Find a declared notification and refuse an undeclared one. */
    get(name: string): NotificationType {
        const notification = this.#notifications.get(name);
        if (notification === undefined) {
            throw new CallError("BAD_REQUEST", {
                message: `${this.packageId} declares no notification ${name}`,
            });
        }

        return notification;
    }

    /** Bind a locale to the release's catalogs. */
    localization(tag: LocaleTag): Localization {
        return Localization.of(tag, this.#catalogs);
    }

    /** Read the localization a principal reads: a person's profile's, else the messages' own. */
    async localize(context: Pick<MethodContext, "read">, subject: Subject): Promise<Localization> {
        // read the messages as authored for a principal without a profile, such as an agent
        if (!principal.user.is(subject)) {
            return this.localization(SOURCE_LOCALE);
        }

        // read a person's language from their public profile
        const profile = await context
            .read(user)
            .findFirst({ where: { id: user.identifier(subject.id) } });

        return this.localization(present(profile, `the profile of ${subject.id}`).locale);
    }
}
