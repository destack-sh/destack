/** Application identity shared by builds, installers and update verification. */
export class ReleaseIdentity {
    /** Stable, nightly or local development selection. */
    readonly channel: "stable" | "nightly" | "dev";

    /** Validate an explicit application selection. */
    constructor(channel: string) {
        // reject identities without an authored distribution
        if (channel !== "stable" && channel !== "nightly" && channel !== "dev") {
            throw new Error("select stable, nightly or dev application identity");
        }
        this.channel = channel;
    }

    /** Artwork and launcher suffix. */
    get suffix(): string {
        return this.channel === "stable" ? "" : `-${this.channel}`;
    }

    /** Native application identifier. */
    get applicationIdentifier(): string {
        return `app.destack.desktop${this.channel === "stable" ? "" : `.${this.channel}`}`;
    }

    /** Display name in native application menus. */
    get title(): string {
        return this.channel === "stable"
            ? "Destack"
            : this.channel === "nightly"
              ? "Destack Nightly"
              : "Destack Dev";
    }
}
