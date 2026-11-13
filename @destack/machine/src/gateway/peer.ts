import { type Authentication, Caller } from "@destack/access";
import { Bearer } from "@destack/access/token";
import { space } from "@destack/account/object";
import { PackageId } from "@destack/package";
import { schema } from "@destack/schema";
import { Rollout, type InstallationRollout } from "@destack/stack/object";
import stackPackage from "@destack/stack/package";
import type { GatewaySpace } from "./gateway.ts";

/** A call another space signed with its key, as its installation calls one served here. */
export class PeerCall {
    /** The call's request. */
    readonly request: Request;
    /** The space's bearer token of the request. */
    readonly token: string;

    /** Store a request with a space's token. */
    private constructor(request: Request, token: string) {
        this.request = request;
        this.token = token;
    }

    /** Read a call another space signed, absent for a request with another token or none. */
    static of(request: Request): PeerCall | undefined {
        const token = Bearer.read(request.headers);
        const issuer = token === undefined ? undefined : Bearer.issuer(token);

        return token !== undefined && issuer !== undefined && space.identifies(issuer)
            ? new PeerCall(request, token)
            : undefined;
    }

    /** Take a call to an installation the host serves, inside its caller's account chain. */
    async receive(
        receiving: Pick<GatewaySpace, "database" | "contain" | "universe" | "machine">,
        installed: { readonly spaceId: string; readonly installation: string },
    ): Promise<{
        readonly rollout: InstallationRollout;
        readonly authentication: Authentication;
    }> {
        // verify the token for the stack package receiving calls to the space's installations, before reading any
        const { machine } = receiving;
        const verified = await machine.verifier.verify(this.token, {
            audience: stackPackage.id,
            partition: schema.identifier("space").parse(installed.spaceId),
            now: machine.environment.clock.now(),
        });

        // require the installation the space serves
        const served = await Rollout.requireServed(receiving.database, installed);
        const audience = PackageId.parse(served.installation.packageId);
        const credential = { ...verified.credential, audience };

        // call as a person acting through the calling installation under their grant, as the universe describes them
        if (Caller.isDelegated(verified.caller)) {
            return { rollout: served, authentication: { caller: verified.caller, credential } };
        }

        // call as the space or its installation, inside its account's chain
        const contained = await receiving.contain(verified);

        return { rollout: served, authentication: { caller: contained.caller, credential } };
    }
}
