import { type CallInput, ListPage, type MethodContext } from "@destack/object";
import type { ImplementationInput } from "@destack/object/server";
import type { PackageId } from "@destack/package";
import { CallError } from "@destack/object";
import { Catalog, Localization, type LocaleTag } from "@destack/locale";
import { type Duration, fromJsonSchema, type Identifier, schema } from "@destack/schema";
import { catalog, installation, installationRevision, symbol } from "@destack/stack/object";
import {
    NotificationDescription,
    NotificationType,
    type NotifyInput,
} from "../notification/notification.ts";
import { Recipient } from "../message/recipient.ts";
import type { Subject } from "@destack/sync";
import type notificationPackage from "../package.ts";
import { implementActivity } from "./activity.ts";
import { implementAnnouncement } from "./announcement.ts";
import { HOME_FOLLOWS } from "./home.ts";
import { implementNotification } from "./notification.ts";
import { implementSummary } from "./summary.ts";
import type { NotificationOptions } from "./options.ts";
import { implementSubscription } from "./subscription.ts";

/** How long a push service stores a message for an offline browser by default: one day. */
const PUSH_TTL: Duration = { days: 1 };

/** The default email delay: a quarter hour, as chat and issue trackers commonly wait. */
const EMAIL_DELAY: Duration = { minutes: 15 };

/** The recipients of one announcement batch by default, as many as one notify call lists. */
const BATCH = 100;

/** The keys reconciled at once by default. */
const CONCURRENCY = 16;

/** The notifications a build declares by name, and the catalogs it ships. */
interface ReadBuild {
    /** The declared notifications by name. */
    readonly notifications: ReadonlyMap<string, NotificationType>;
    /** The catalogs translating its messages and its unpublished dependencies'. */
    readonly catalogs: readonly Catalog[];
}

/** The notification package's server in a store: its tuning, and the builds it read. */
export class NotificationServer {
    /** The recipients of one announcement batch. */
    readonly batch: number;
    /** The keys each controller reconciles at once. */
    readonly concurrency: number;
    /** How long a push service stores a message for an offline browser. */
    readonly pushTtl: Duration;
    /** How long an email waits for the notification to be read elsewhere. */
    readonly emailDelay: Duration;
    /** The notifications and translations of each build read so far. */
    readonly #builds = new Map<string, Promise<ReadBuild>>();

    /** Serve notifications with the given tuning, the defaults elsewhere. */
    constructor(options: NotificationOptions) {
        // take each option, else its default
        this.batch = options.batch ?? BATCH;
        this.concurrency = options.concurrency ?? CONCURRENCY;
        this.pushTtl = options.pushTtl ?? PUSH_TTL;
        this.emailDelay = options.emailDelay ?? EMAIL_DELAY;
    }

    /** Read a notification the space's installed release declares, refusing an undeclared one. */
    async notification(
        context: Pick<MethodContext, "read" | "query" | "self" | "scope">,
        packageId: PackageId,
        name: string,
    ): Promise<NotificationType> {
        // look the notification up in the release's build
        const { notifications } = await this.#read(context, packageId);
        const declared = notifications.get(name);
        if (declared === undefined) {
            throw new CallError("BAD_REQUEST", {
                message: `${packageId} declares no notification ${name}`,
            });
        }

        return declared;
    }

    /** Notify a notice's recipients of an occurrence, rendered in each person's locale. */
    async notify(context: MethodContext, notice: NotifyInput, occurredAt: number): Promise<void> {
        // render with the declaring release's catalogs in each recipient's locale
        const { packageId, name } = notice;
        const declared = await this.notification(context, packageId, name);
        const localize = async (person: Subject) =>
            this.localization(context, packageId, await Recipient.locale(context, person));
        await declared.notify(localize, context, notice, occurredAt);
    }

    /** Bind a locale to the translations of the release a call's space installs. */
    async localization(
        context: Pick<MethodContext, "read" | "query" | "self" | "scope">,
        packageId: PackageId,
        tag: LocaleTag,
    ): Promise<Localization> {
        const { catalogs } = await this.#read(context, packageId);

        return Localization.of(tag, catalogs);
    }

    /** Read the notifications and translations of the build the space's installation follows. */
    async #read(
        context: Pick<MethodContext, "read" | "query" | "self" | "scope">,
        packageId: PackageId,
    ): Promise<ReadBuild> {
        // read the revision the space's installation of the package follows
        const scope = schema.identifier("space").parse(context.scope);
        const [installed] = await context
            .read(installation)
            .findMany({ where: { scope, packageId } });
        const revision =
            installed === undefined || installed.revisionId === null
                ? undefined
                : await context
                      .read(installationRevision)
                      .findFirst({ where: { id: installed.revisionId } });
        if (revision === undefined) {
            throw new CallError("NOT_FOUND", {
                message: `no installation in ${context.scope} follows a build of ${packageId}`,
            });
        }

        // read its build's notifications and translations once per build, which never changes
        const known = this.#builds.get(revision.buildId);
        if (known !== undefined) {
            return known;
        }
        const reading = NotificationServer.#index(context, revision.buildId);
        this.#builds.set(revision.buildId, reading);

        return reading;
    }

    /** Read the notifications a build declares and the catalogs it ships, from its index. */
    static async #index(
        context: Pick<MethodContext, "query" | "self">,
        buildId: Identifier<"build">,
    ): Promise<ReadBuild> {
        // read every notification the build declares and every catalog it ships
        const where = { kindPackageId: import.meta.destack.package.id, kind: "notification" };
        const [declared, shipped] = await Promise.all([
            Array.fromAsync(
                ListPage.all(
                    (input: CallInput<typeof symbol, "list">) =>
                        context.query(context.self, symbol.call("list", input)),
                    { buildId, where },
                ),
            ),
            Array.fromAsync(
                ListPage.all(
                    (input: CallInput<typeof catalog, "list">) =>
                        context.query(context.self, catalog.call("list", input)),
                    { buildId },
                ),
            ),
        ]);

        // rebuild each notification's payload schema, and each catalog
        const notifications = declared.map((row) => {
            const {
                package: declaring,
                payload,
                actions,
                ...definition
            } = NotificationDescription.parse(row.description);

            return new NotificationType(
                declaring,
                { ...definition, payload: fromJsonSchema(payload) },
                actions,
            );
        });
        const catalogs = shipped.map(({ packageId, locale, messages, drafts }) => ({
            package: packageId,
            locale,
            messages,
            ...(drafts === null ? {} : { drafts }),
        }));

        return {
            notifications: new Map(notifications.map((each) => [each.name, each])),
            catalogs,
        };
    }

    /** Define the package's server: its types, their delivery and summaries, and home follows. */
    definition(): ImplementationInput<typeof notificationPackage> {
        return {
            objects: [
                implementActivity(this),
                implementAnnouncement(this),
                implementSubscription(),
                implementNotification(this),
                implementSummary(this),
            ],
            follows: HOME_FOLLOWS,
        };
    }
}
