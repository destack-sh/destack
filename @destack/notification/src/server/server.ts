import type { MethodContext } from "@destack/object";
import type { ImplementationInput } from "@destack/object/server";
import type { PackageId } from "@destack/package";
import { NotificationError } from "../error/error.ts";
import { Localization, type LocaleTag } from "@destack/locale";
import { type Duration, fromJsonSchema } from "@destack/schema";
import {
    NOTIFICATION_KIND,
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
import type {} from "@destack/package/import-meta";

/** How long a push service stores a message for an offline browser by default: one day. */
const PUSH_TTL: Duration = { days: 1 };

/** The default email delay: a quarter hour, as chat and issue trackers commonly wait. */
const EMAIL_DELAY: Duration = { minutes: 15 };

/** The recipients of one announcement batch by default, as many as one notify call lists. */
const BATCH = 100;

/** The keys reconciled at once by default. */
const CONCURRENCY = 16;

/** The notification package's server in a store: its tuning. */
export class NotificationServer {
    /** The recipients of one announcement batch. */
    readonly batch: number;
    /** The keys each controller reconciles at once. */
    readonly concurrency: number;
    /** How long a push service stores a message for an offline browser. */
    readonly pushTtl: Duration;
    /** How long an email waits for the notification to be read elsewhere. */
    readonly emailDelay: Duration;

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
        context: Pick<MethodContext, "declarations">,
        packageId: PackageId,
        name: string,
    ): Promise<NotificationType> {
        // look the notification up in the release's declarations
        const declarations = await context.declarations(NOTIFICATION_KIND, packageId);
        const declared = declarations.find((each) => each.name === name);
        if (declared === undefined) {
            throw NotificationError.of(
                "NOTIFICATION_UNKNOWN",
                `${packageId} declares no notification ${name}`,
            );
        }

        // rebuild its payload schema
        const { package: declaring, payload, actions, ...definition } = declared.description;

        return new NotificationType(
            declaring,
            { ...definition, payload: fromJsonSchema(payload) },
            actions,
        );
    }

    /** Notify a notice's recipients of an occurrence, rendered in each person's locale. */
    async notify(
        context: MethodContext,
        notice: NotifyInput,
        at: { readonly occurredAt: number; readonly actor: Subject },
    ): Promise<void> {
        // render with the declaring release's catalogs in each recipient's locale
        const { packageId, name } = notice;
        const declared = await this.notification(context, packageId, name);
        const localize = async (person: Subject) =>
            this.localization(context, packageId, await Recipient.locale(context, person));
        await declared.notify(localize, context, notice, at);
    }

    /** Bind a locale to the translations of the release a call's space installs. */
    async localization(
        context: Pick<MethodContext, "catalogs">,
        packageId: PackageId,
        tag: LocaleTag,
    ): Promise<Localization> {
        return Localization.of(tag, await context.catalogs(packageId));
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
