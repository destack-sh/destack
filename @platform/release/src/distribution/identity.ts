import { ReleaseChannel } from "@destack/daemon/process";

/** Read the channel a build selects from its environment, local development by default. */
export function selectedChannel(): ReleaseChannel {
    return ReleaseChannel.of(process.env["DESTACK_RELEASE_CHANNEL"] ?? "dev");
}
