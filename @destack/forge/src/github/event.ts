import { schema } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import type { WebhookDelivery } from "@destack/service/trigger";

/** The events that the forge's GitHub App subscribes to and that change references. */
const REFERENCE_EVENTS: ReadonlySet<string> = new Set(["push", "create", "delete"]);

/** The repository and installation a repository event names. */
const Source = schema.looseObject({
    repository: schema.looseObject({ id: schema.number().int() }),
    installation: schema.looseObject({ id: schema.number().int() }),
});

/** The GitHub repository with the references one delivery changed. */
export interface GitHubEvent {
    /** GitHub's repository identifier, as text. */
    readonly repositoryId: string;
    /** The app installation the delivery is for, as text. */
    readonly installationId: string;
}

/** The events of GitHub's push, create and delete webhooks. */
export const GitHubEvent = {
    /** Read which repository a delivery says changed, null for the ping GitHub sends when a webhook is created. */
    read(delivery: WebhookDelivery): GitHubEvent | null {
        // acknowledge the ping without a change
        if (delivery.event === "ping") {
            return null;
        }
        // read the repository and installation of a reference change
        else if (REFERENCE_EVENTS.has(delivery.event)) {
            const source = Source.parse(delivery.payload);

            return {
                repositoryId: String(source.repository.id),
                installationId: String(source.installation.id),
            };
        }
        // refuse events the app does not subscribe to
        else {
            throw new ServiceError("BAD_REQUEST", {
                message: `unsubscribed github event: ${delivery.event}`,
            });
        }
    },
};
