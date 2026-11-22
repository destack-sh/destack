import {
    Contact,
    principal,
    accessRelationship,
    INVITATION_PAGE,
    Invitation,
} from "@destack/access";
import type { MethodContextOf, ResultOf } from "@destack/object";
import { Scope } from "@destack/sync";
import { SpaceAddress } from "../address/address.ts";
import { account } from "../object/account.ts";
import { space } from "../object/space.ts";
import { user } from "../object/user.ts";

/** List the addresses of the spaces a user has a role in, their own accounts' aside. */
export async function shared(
    context: MethodContextOf<typeof user, "shared">,
): Promise<ResultOf<typeof user, "shared">> {
    // read the user's own relationships to spaces, leaving those anyone has
    const person = principal.user.reference(Scope.universe.id, context.target.id);
    const held = await context.read(accessRelationship).findMany({
        where: {
            subjectPackageId: person.packageId,
            subjectType: person.type,
            subjectScope: person.scope,
            subjectId: person.id,
            subjectRelation: { isNull: true },
            objectPackageId: space.typeReference.packageId,
            objectType: space.name,
        },
    });
    const ids = [...new Set(held.map((row) => space.identifier(row.objectId)))];

    // read each space's name and its account's handle
    const spaces = await context.read(space).findMany({ where: { id: { in: ids } } });
    const accounts = await context
        .read(account)
        .findMany({ where: { id: { in: spaces.map((each) => each.scope) } } });
    const handles = new Map<string, string>(accounts.map((each) => [each.id, each.handle]));
    const items = spaces.flatMap((each) => {
        const handle = handles.get(each.scope);

        return handle === undefined ? [] : [SpaceAddress.of({ space: each.name, handle })];
    });

    return { items: items.toSorted() };
}

/** List the pending invitations addressed to a user, or to the user's email once verified. */
export async function invited(
    context: MethodContextOf<typeof user, "invited">,
): Promise<ResultOf<typeof user, "invited">> {
    // address the user and the email's contact
    const { target } = context;
    const person = principal.user.reference(Scope.universe.id, target.id);
    const contacts =
        target.email === null || target.emailVerified !== true
            ? []
            : [Contact.subject(Contact.email(target.email))];

    // read a page of the pending invitations to either
    const items = await Invitation.addressed(
        context,
        [person, ...contacts],
        { limit: INVITATION_PAGE },
        context.now,
    );

    return { items };
}
