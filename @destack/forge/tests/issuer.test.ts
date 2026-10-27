import { accessRelationship, principal } from "@destack/access";
import type { ResidencyCode } from "@destack/account/object";
import accountPackage from "@destack/account/package";
import { Issuer } from "@destack/connection/object";
import { eq } from "@destack/db";
import { Identifier } from "@destack/schema";
import { RequestId } from "@destack/sync";
import { expect, refusal, test } from "@destack/test";
import { appInstallation, issuer } from "../src/object/index.ts";
import { TestForge } from "../src/test/index.ts";

/** The fixture GitHub App's installation on the first account's GitHub organization. */
const INSTALLATION = "4242";

/** The fixture GitHub App's installation on the second account's GitHub organization. */
const OTHER_INSTALLATION = "5151";

test("declare the residency home's GitHub issuer at the id its home and declaration name derive, related to the residency", async () => {
    // read the issuer the home's stack declares, and its relationship to the residency
    const forge = await TestForge.open();
    const issuers = await forge.served.database.select({ id: issuer.table.id }).from(issuer.table);
    const related = await forge.served.database
        .select({
            objectId: accessRelationship.objectId,
            relation: accessRelationship.relation,
            subjectType: accessRelationship.subjectType,
            subjectId: accessRelationship.subjectId,
        })
        .from(accessRelationship)
        .where(eq(accessRelationship.objectType, issuer.name));
    const declared = await Issuer.declared({ id: forge.space }, "github", forge.environment.crypto);
    expect({ issuers, related }).toEqual({
        issuers: [{ id: declared }],
        related: [
            {
                objectId: declared,
                relation: "residency",
                subjectType: "residency",
                subjectId: "eu",
            },
        ],
    });
});

test("issue a GitHub installation's tokens to the resident account's installation that claimed it, refusing another installation, an unclaimed installation, a second claim and another residency's installation", async () => {
    // run the GitHub App against the stand-in, with two resident accounts and another residency's, a space each
    const forge = await TestForge.open();
    const accounts = forge.owner.client(accountPackage);
    const place = async (handle: string, residencyId: ResidencyCode) => {
        const created = await accounts.account.create({
            scope: forge.owner.subject.id,
            requestId: RequestId.create(forge.environment),
            handle,
            name: handle,
            kind: "shared",
            residencyId,
        });
        const space = await accounts.space.create({
            accountId: created.id,
            requestId: RequestId.create(forge.environment),
            name: "space",
        });

        return { accountId: created.id, spaceId: space.id };
    };
    const first = await place("first", "eu");
    const second = await place("second", "eu");
    const foreign = await place("foreign", "us");
    await forge.settle();
    const firstSpace = first.spaceId;
    const secondSpace = second.spaceId;
    const foreignSpace = foreign.spaceId;
    const installed = (spaceId: Identifier<"space">) =>
        forge.as(
            principal.installation.reference(
                spaceId,
                Identifier.create("installation", forge.environment),
            ),
        );
    const claimer = installed(firstSpace);
    const sibling = installed(firstSpace);
    const outsider = installed(secondSpace);
    const foreigner = installed(foreignSpace);
    const id = await Issuer.declared({ id: forge.space }, "github", forge.environment.crypto);
    const target = { spaceId: forge.space, id };

    // claim the first account's installation through its setup redirect, as its connection completes it
    const claim = (caller: typeof claimer, installation: string) =>
        caller.issuer.installation({
            ...target,
            requestId: RequestId.create(forge.environment),
            parameters: { installation_id: installation, setup_action: "install" },
        });
    const claimed = await claim(claimer, INSTALLATION);
    const issue = (caller: typeof claimer, installation: string) =>
        caller.issuer.issue({ ...target, installation });

    // issue to the claimer alone, and refuse another claim of the claimed installation
    expect({
        claimed,
        issued: (await issue(claimer, INSTALLATION)).accessToken,
        sibling: await refusal(issue(sibling, INSTALLATION)),
        outsider: await refusal(issue(outsider, INSTALLATION)),
        unclaimed: await refusal(issue(claimer, OTHER_INSTALLATION)),
        reclaimed: await refusal(claim(outsider, INSTALLATION)),
        foreign: await refusal(claim(foreigner, OTHER_INSTALLATION)),
        claims: await forge.served.database
            .select({
                installation: appInstallation.table.installation,
                accountId: appInstallation.table.accountId,
            })
            .from(appInstallation.table),
    }).toEqual({
        claimed: { installation: INSTALLATION },
        issued: "ghs_4242",
        sibling: [
            "FORBIDDEN",
            "INSTALLATION_NOT_CLAIMED",
            `the calling installation has not claimed GitHub installation ${INSTALLATION}`,
        ],
        outsider: [
            "FORBIDDEN",
            "INSTALLATION_NOT_CLAIMED",
            `the calling installation has not claimed GitHub installation ${INSTALLATION}`,
        ],
        unclaimed: [
            "FORBIDDEN",
            "INSTALLATION_NOT_CLAIMED",
            `the calling installation has not claimed GitHub installation ${OTHER_INSTALLATION}`,
        ],
        reclaimed: [
            "CONFLICT",
            "INSTALLATION_CLAIMED",
            `GitHub installation ${INSTALLATION} is claimed by another installation`,
        ],
        foreign: ["NOT_FOUND", "NOT_FOUND", `issuer ${id} not found`],
        claims: [{ installation: INSTALLATION, accountId: first.accountId }],
    });
});
