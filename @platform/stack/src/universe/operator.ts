import type { RoleRequest } from "@destack/access";
import { connect } from "@destack/account/client";
import { organisation, type ResidencyCode } from "@destack/account/object";
import { HostIdentity } from "@destack/host/identity";
import { MemoryKeychain } from "@destack/host/keychain";
import type { PackageId } from "@destack/package";
import { Identifier, schema } from "@destack/schema";
import { RequestId } from "@destack/service/request";
import { PLATFORM_HANDLE } from "./handle.ts";

/** A region of the universe: its code, display name and residency. */
export interface RegionDefinition {
    /** The short code, such as eu-central. */
    readonly code: string;
    /** The display name. */
    readonly name: string;
    /** The jurisdiction the region keeps data in. */
    readonly residencyId: ResidencyCode;
}

/** Where the account service answers, and how its sign-in links reach the operator. */
export interface OperatorService {
    /** The account service's URL. */
    readonly url: string;
    /** The origin answering sign-in. */
    readonly origin: string;
    /** Send a request to the account service. */
    readonly fetch: (request: Request) => Promise<Response>;
    /** Wait for the next sign-in link mailed to an address. */
    readonly link: (email: string) => Promise<string>;
}

/** The universe's operator, an owner of the platform organisation who keeps its regions, placements and roles. */
export class Operator {
    /** The account service, called as the operator. */
    readonly accounts: ReturnType<typeof connect>;
    /** The platform's own account, which owns the regions' hosts. */
    readonly accountId: Identifier<"account">;

    /** Keep a signed-in operator and the platform's account. */
    private constructor(accounts: ReturnType<typeof connect>, accountId: Identifier<"account">) {
        this.accounts = accounts;
        this.accountId = accountId;
    }

    /** Sign the operator in through a mailed link and accept the invitations the account service keeps for them, the platform organisation's among them. */
    static async open(service: OperatorService, email: string): Promise<Operator> {
        // follow a sign-in link mailed to the operator, keeping the session cookie and reading each answer to its end
        const link = service.link(email);
        const requested = await service.fetch(
            new Request(`${service.origin}/auth/sign-in/magic-link`, {
                method: "POST",
                headers: { origin: service.origin, "content-type": "application/json" },
                body: JSON.stringify({ email, name: "Operator", callbackURL: "/" }),
            }),
        );
        await requested.arrayBuffer();
        if (!requested.ok) {
            throw new Error(`the operator's sign-in failed with ${requested.status}`);
        }
        const signedIn = await service.fetch(new Request(await link, { redirect: "manual" }));
        await signedIn.arrayBuffer();
        const cookie = signedIn.headers
            .getSetCookie()
            .map((entry) => entry.slice(0, entry.indexOf(";")))
            .join("; ");

        // accept the organisation invitations to the operator's address, which only the platform sends
        const accounts = connect({
            url: service.url,
            headers: { cookie, origin: service.origin },
            fetch: service.fetch,
        });
        const { subject } = await accounts.authentication.current();
        const userId = schema.identifier("user").parse(subject.id);
        const { items } = await accounts.user.invited({ id: userId });
        const offers = items.filter((entry) => organisation.policy.is(entry.relationship.object));
        for (const { id, relationship } of offers) {
            await accounts.organisation.accept({
                id: relationship.object.id,
                invitationId: id,
                requestId: RequestId.create(),
            });
        }

        // find the platform's own account
        const accountId = await accounts.directory.account({ handle: PLATFORM_HANDLE });
        if (accountId === null) {
            throw new Error(`the account service keeps no platform account ${PLATFORM_HANDLE}`);
        }

        return new Operator(accounts, accountId);
    }

    /** Keep a region by its code once. */
    async region(definition: RegionDefinition): Promise<Identifier<"region">> {
        // keep an existing region
        const { items } = await this.accounts.region.list({ where: { code: definition.code } });
        const [kept] = items;
        if (kept !== undefined) {
            return kept.id;
        }

        // create the region
        const created = await this.accounts.region.create({
            requestId: RequestId.create(),
            ...definition,
        });

        return created.id;
    }

    /** Place a package's workload in a region once. */
    async place(
        packageId: PackageId,
        regionId: Identifier<"region">,
    ): Promise<Identifier<"placement">> {
        // keep an existing placement
        const { items } = await this.accounts.placement.list({ where: { packageId, regionId } });
        const [kept] = items;
        if (kept !== undefined) {
            return kept.id;
        }

        // place the workload
        const created = await this.accounts.placement.create({
            requestId: RequestId.create(),
            packageId,
            regionId,
        });

        return created.id;
    }

    /** Bind a universe role to a placed workload alone, replacing the role's permissions. */
    async grant(role: RoleRequest, placementId: Identifier<"placement">): Promise<void> {
        await this.accounts.placement.grant({
            requestId: RequestId.create(),
            id: placementId,
            role,
        });
    }

    /** Enroll a new host serving a region under the platform's account, revoking the region's earlier hosts. */
    async host(regionId: Identifier<"region">): Promise<HostIdentity> {
        // revoke the region's standing hosts, whose keys left with their processes
        const { items } = await this.accounts.host.list({
            accountId: this.accountId,
            where: { regionId },
        });
        for (const { id } of items.filter((entry) => entry.revokedAt === null)) {
            await this.accounts.host.revoke({
                accountId: this.accountId,
                id,
                requestId: RequestId.create(),
            });
        }

        // enroll this process as the region's host with a key it keeps in memory
        const identity = new HostIdentity(Identifier.create("host"), new MemoryKeychain());
        await identity.enroll(this.accounts, {
            accountId: this.accountId,
            requestId: RequestId.create(),
            name: "universe",
            kind: "cloud",
            regionId,
        });

        return identity;
    }
}
