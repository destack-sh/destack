import type { MethodContext } from "@destack/object";
import type { ServerDefinition } from "@destack/object/server";
import type { PackageId } from "@destack/package";
import { CallError } from "@destack/runtime/error";
import { type Duration, schema } from "@destack/schema";
import { installation, installationRevision } from "@destack/stack/object";
import { ScheduledCall } from "@destack/workflow";
import { NotificationCatalog } from "../notification/catalog.ts";
import { deliver, notification, scheduled, summarize } from "../object/notification.ts";
import type notificationPackage from "../package.ts";
import { implementActivity } from "./activity.ts";
import { implementAnnouncement } from "./announcement.ts";
import { Delivery } from "./delivery.ts";
import { HOME_FOLLOWS } from "./home.ts";
import { implementNotification } from "./notification.ts";
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

/** The notification package's server in a store: its tuning, and the catalogs it read. */
export class NotificationServer {
    /** The recipients of one announcement batch. */
    readonly batch: number;
    /** The keys each controller reconciles at once. */
    readonly concurrency: number;
    /** How long a push service stores a message for an offline browser. */
    readonly pushTtl: Duration;
    /** How long an email waits for the notification to be read elsewhere. */
    readonly emailDelay: Duration;
    /** The notifications and catalogs of each release read so far. */
    readonly #catalogs = new Map<string, Promise<NotificationCatalog>>();

    /** Serve notifications with the given tuning, the defaults elsewhere. */
    constructor(options: NotificationOptions) {
        // take each option, else its default
        this.batch = options.batch ?? BATCH;
        this.concurrency = options.concurrency ?? CONCURRENCY;
        this.pushTtl = options.pushTtl ?? PUSH_TTL;
        this.emailDelay = options.emailDelay ?? EMAIL_DELAY;
    }

    /** Read the notifications and translations of the release a call's space installs. */
    async catalog(
        context: Pick<MethodContext, "read" | "scope">,
        packageId: PackageId,
    ): Promise<NotificationCatalog> {
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

        // describe its notifications and translations once per revision, which never changes
        const known = this.#catalogs.get(revision.id);
        if (known !== undefined) {
            return known;
        }
        const reading = NotificationCatalog.read(context, revision, packageId);
        this.#catalogs.set(revision.id, reading);

        return reading;
    }

    /** Define the package's server: its types, an occurrence's delivery, and a home's follows. */
    definition(): ServerDefinition<typeof notificationPackage> {
        // deliver each occurrence, send each scheduled summary once due, and make scheduled calls
        const workflows = [
            deliver.implement((step, input, run) => new Delivery(this, step, input, run).run()),
            summarize.implement(async (step, { at }, run) => {
                await step.sleepUntil("wait", at);
                const summary = notification.call("summarize", {
                    ...notification.in(run.scope),
                    at,
                });
                await step.call("summarize", summary);
            }),
            scheduled.implement((step, input) => ScheduledCall.run(step, input)),
        ];

        return {
            objects: [
                implementActivity(this),
                implementAnnouncement(this),
                implementSubscription(),
                implementNotification(this),
                ...workflows.flatMap((each) => each.objects),
            ],
            follows: HOME_FOLLOWS,
            controllers: workflows.flatMap((each) => each.controllers),
        };
    }
}
