import { Contact, principal } from "@destack/access";
import { space } from "@destack/account/object";
import { Identifier, present, schema } from "@destack/schema";
import { RequestId, Scope } from "@destack/sync";
import { type Verb, OptionValue } from "../command/command.ts";
import type { Context } from "../context/context.ts";
import { CliError } from "../error/index.ts";

/** The roles a space is shared through, lowest first. */
const ROLES = ["viewer", "commenter", "editor", "owner"] as const;

/** The role a person gets unless another is asked for. */
const DEFAULT_ROLE = "editor";

/** The prefix of a person's handle. */
const HANDLE_PREFIX = "@";

/** Invite a person to the space the terminal acts in, by their handle or email. */
export function shareCommand(context: Context): Verb {
    return {
        description: "Invite a person to the space, by @handle or email, as a role",
        arguments: [{ name: "person", usage: "@handle|email" }],
        options: {
            role: {
                type: "string",
                description: `The role: ${ROLES.join(", ")}; ${DEFAULT_ROLE} by default`,
                argument: "role",
            },
        },
        json: true,
        async run({ arguments: { person }, options, json }) {
            // read the role and the space with its account, whose scope its row lives in
            const asked = OptionValue.string(options["role"], "role") ?? DEFAULT_ROLE;
            const role = schema.enum(ROLES).safeParse(asked);
            if (!role.success) {
                throw CliError.of(
                    "USAGE_INVALID",
                    `role '${asked}' is invalid: use viewer, commenter, editor or owner`,
                );
            }
            const native = await context.native();
            const spaceId = await context.requireSpace();
            const accountId = (await native.locate(spaceId)).placement.scope;

            // address the person: the user of a handle's personal account, or an email's contact
            const named = present(person, "the person argument");
            const subject = named.startsWith(HANDLE_PREFIX)
                ? principal.user.reference(
                      Scope.universe.id,
                      Identifier.derive(
                          "user",
                          Identifier.uuid(await native.resolve(named.slice(HANDLE_PREFIX.length))),
                      ),
                  )
                : Contact.subject(Contact.email(schema.email().parse(named)));

            // invite them at the universe, which stores the space's row and its invitations
            const { session } = await context.launcher();
            const invitation = await session.objects().invitation.create({
                scope: accountId,
                requestId: RequestId.create(native.environment),
                object: space.reference(accountId, spaceId),
                relationship: { relation: role.data, subject },
            });
            await context.terminal.print(
                json
                    ? JSON.stringify(invitation)
                    : `Invited ${named} as ${role.data}. They accept with: destack accept ${invitation.id}`,
            );
        },
    };
}

/** List the invitations to the person signed in, or accept one of them. */
export function acceptCommand(context: Context): Verb {
    return {
        description: "List the invitations to you, or accept one",
        arguments: [{ name: "invitation", usage: "invitation-…", default: "" }],
        json: true,
        async run({ arguments: { invitation }, json }) {
            // read the invitations to the person and their verified email
            const native = await context.native();
            const { session, user } = await context.launcher();
            const { items } = await session.accounts().user.invited({ id: user.userId });

            // list them without an argument
            if (invitation === undefined || invitation === "") {
                const lines = items.map(({ id, relationship }) => {
                    const role =
                        "relation" in relationship ? relationship.relation : relationship.role;

                    return `${id}  ${role}  ${relationship.object.type} ${relationship.object.id}`;
                });
                await context.terminal.print(
                    json ? JSON.stringify(items) : lines.join("\n") || "No invitations.",
                );

                return;
            }

            // accept the one asked for in the scope it lives in
            const found = items.find((each) => each.id === invitation);
            if (found === undefined) {
                throw CliError.of(
                    "INVITATION_NOT_FOUND",
                    `invitation '${invitation}' not found: list yours with destack accept`,
                );
            }
            await session.objects().invitation.accept({
                scope: found.relationship.object.scope,
                requestId: RequestId.create(native.environment),
                id: found.id,
            });
            const { object } = found.relationship;
            await context.terminal.print(
                json
                    ? JSON.stringify(found)
                    : `Accepted. You now have ${object.type} ${object.id}.`,
            );
        },
    };
}
