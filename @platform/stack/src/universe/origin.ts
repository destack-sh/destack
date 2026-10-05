import { ReleaseChannel } from "@destack/daemon/process";

/** The origin and sign-in issuer of the dev universe that `just universe` serves. */
export const UNIVERSE_ORIGIN = ReleaseChannel.of("dev").issuer;

/** The origin of the dev universe's relay, where hosts open their tunnels. */
export const RELAY_ORIGIN = "http://127.0.0.1:4101";
