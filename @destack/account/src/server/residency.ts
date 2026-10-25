import type { MethodContextOf, ResultOf } from "@destack/object";
import {
    anyone,
    InstallationId,
    PermissionReference,
    principal,
    Relationship,
} from "@destack/access";
import { Claim, claimTable, PackageDescription } from "@destack/object";
import { present } from "@destack/schema";
import { Scope } from "@destack/sync";
import { Account, account } from "../object/account.ts";
import { Endpoint, endpoint } from "../object/endpoint.ts";
import { machine } from "../object/machine.ts";
import { Residency, residency, type ResidencyInstallation } from "../object/residency.ts";
import { StoreRelationship } from "./relationship.ts";
import type { ReleaseRegistry } from "./server.ts";
import { space } from "../object/space.ts";
import { AccountError } from "../error/error.ts";
import { UniverseRole } from "./universe.ts";

/** Implement residencies, opening the releases their homes' installations follow. */
export function implementResidency(releases: ReleaseRegistry): typeof residency {
    return residency.implement({
        methods: {
            // show a new residency to everyone choosing one while signing up
            create: {
                creation: () => ({
                    relationships: [{ relation: "public", subject: anyone.reference("*", "*") }],
                }),
            },
            touch: (context) => touch(context, releases),
            revoke,
            locate,
            machines,
            publisher,
        },
    });
}

/** Relate an installation its home declares, whose release asks for what the relation gives. */
async function touch(
    context: MethodContextOf<typeof residency, "touch">,
    releases: ReleaseRegistry,
): Promise<ResultOf<typeof residency, "touch">> {
    // require the installation its home declares under the name
    const { packageId, release } = context.input;
    await requireDeclared(context, context.input);

    // require the release to declare its package, asking for exactly what the relation gives in the universe
    const description = await PackageDescription.of(await releases.open(packageId, release));
    if (description === undefined) {
        throw AccountError.of("PACKAGE_UNDECLARED", `${packageId}@${release} declares no package`);
    }
    const requested = description.permissions.installation?.universe ?? [];
    const asked = permissionKeys(requested);
    const given = permissionKeys(Residency.installedPermissions());
    const isGiven =
        asked.length === given.length && asked.every((key, index) => key === given[index]);
    if (!isGiven) {
        const named = requested.map(({ type, name }) => `${type}.${name}`).toSorted();
        throw AccountError.of(
            "UNIVERSE_PERMISSIONS_MISMATCH",
            `${packageId}@${release} asks for ${named.join(", ") || "nothing"} in the universe, not what a residency's installation holds`,
        );
    }

    // relate it once
    await StoreRelationship.touch(context, residencyInstallation(context.target, context.input));

    return context.target;
}

/** Unrelate an installation its home no longer declares, leaving one it never related. */
async function revoke(
    context: MethodContextOf<typeof residency, "revoke">,
): Promise<ResultOf<typeof residency, "revoke">> {
    // require the installation its home declares under the name
    await requireDeclared(context, context.input);

    // revoke its relationship once, as its home converges on each pass
    const fields = residencyInstallation(context.target, context.input);
    const { object, relation, subject } = fields;
    const [related] = await Relationship.readOn(context, object, { relation, subject });
    if (related !== undefined) {
        await StoreRelationship.delete(context, fields);
    }

    return context.target;
}

/** Refuse an installation other than the one the residency's space declares under its name. */
async function requireDeclared(
    context: MethodContextOf<typeof residency, "touch" | "revoke">,
    input: Pick<ResidencyInstallation, "installation" | "name">,
): Promise<void> {
    // read the residency's space below its account
    const spaceId = Residency.requireSpace(context.target);
    const row = await context.read(space).findFirst({ where: { id: spaceId } });
    const served = space.reference(present(row, `the space ${spaceId}`).scope, spaceId);

    // derive the installation from the space and its name
    const crypto = context.environment.crypto;
    const declared = await InstallationId.declared(served, input.name, crypto);
    if (declared !== input.installation) {
        throw AccountError.of(
            "INSTALLATION_NOT_DECLARED",
            `installation ${input.installation} is not ${input.name} of the space ${spaceId}`,
        );
    }
}

/** List permissions by their keys, in order. */
function permissionKeys(permissions: readonly PermissionReference[]): string[] {
    return permissions.map((permission) => PermissionReference.key(permission)).toSorted();
}

/** Read an installation's relationship to the residency its space serves. */
function residencyInstallation(
    target: Residency,
    input: Pick<ResidencyInstallation, "installation">,
) {
    return {
        object: residency.reference(Scope.universe.id, target.id),
        relation: "installation",
        subject: principal.installation.reference(
            Residency.requireSpace(target),
            input.installation,
        ),
    };
}

/** Locate the space serving the residency an account stores its data in. */
async function locate(
    context: MethodContextOf<typeof residency, "locate">,
): Promise<ResultOf<typeof residency, "locate">> {
    return Account.locate(context, context.input.account);
}

/** List a residency's enabled shared cloud machines with their URLs, in identity order. */
async function machines(
    context: MethodContextOf<typeof residency, "machines">,
): Promise<ResultOf<typeof residency, "machines">> {
    // read the residency's enabled shared machines and the endpoints they published
    const serving = await context.read(machine).findMany({
        where: {
            scope: Scope.universe.id,
            residencyId: context.target.id,
            status: "enabled",
            revokedAt: { isNull: true },
        },
        orderBy: { id: "asc" },
    });
    const published = await context
        .read(endpoint)
        .findMany({ where: { id: { in: serving.map((row) => row.id) } } });

    // list the machines that answer at a URL now
    return serving.flatMap((row) => {
        const found = published.find((entry) => entry.id === row.id);

        return found === undefined || !Endpoint.isAnswering(found, context.now)
            ? []
            : [{ machine: row.id, url: found.url }];
    });
}

/** Find the account selling a handle's packages to the buyers of a residency's home. */
async function publisher(
    context: MethodContextOf<typeof residency, "publisher">,
): Promise<ResultOf<typeof residency, "publisher">> {
    // find the handle's holder, none for a free handle
    const holder = await Claim.owner(
        context.read(claimTable),
        Claim.key(account, "handle", [context.input.handle]),
    );
    if (holder === undefined) {
        return null;
    }

    // sell a name or account of the platform's organization through the residency space's account
    const platform = await UniverseRole.operating(context);
    const isPlatform = holder.objectId === platform || holder.owner === platform;
    if (isPlatform) {
        return (await Residency.isSpace(context, context.input.space))
            ? Account.ofHome(context.input.space)
            : null;
    }

    return account.identifies(holder.objectId) ? account.identifier(holder.objectId) : null;
}
