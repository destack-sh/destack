import { Contact, INVITATION_PAGE, Invitation, principal } from "@destack/access";
import type { MethodContextOf, ResultOf } from "@destack/object";
import { Scope } from "@destack/sync";
import { user } from "../object/user.ts";

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
