import { Contact, principal } from "@destack/access";
import { Account, space } from "@destack/account/object";
import { Shareable } from "@destack/object";
import { present, schema } from "@destack/schema";
import { RequestId, Scope } from "@destack/sync";
import { type Verb, OptionValue } from "../command/command.ts";
import type { Context } from "../context/context.ts";
import { CliError } from "../error/index.ts";
import { Render } from "../object/render.ts";

/** The columns the invitations to a person list under. */
const INVITATION_COLUMNS = ["ROLE", "OBJECT", "ID"];

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
                description: `The role: ${Shareable.roles.join(", ")}; ${DEFAULT_ROLE} by default`,
                argument: "role",
            },
        },
        json: true,
        async run({ arguments: { person }, options, json }) {
            // read the role and the space with its account, whose scope its row lives in
            const asked = OptionValue.string(options["role"], "role") ?? DEFAULT_ROLE;
            const role = schema.enum(Shareable.roles).safeParse(asked);
            if (!role.success) {
                throw CliError.of(
                    "USAGE_INVALID",
                    `role '${asked}' is invalid: it must be one of '${Shareable.roles.join("', '")}'`,
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
                      Account.person(await native.resolve(named.slice(HANDLE_PREFIX.length))),
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
                    : [
                          context.terminal.event(
                              "Invited",
                              "done",
                              `'${named}'`,
                              `as ${role.data}`,
                          ),
                          context.terminal.hint("next", `destack accept ${invitation.id}`),
                      ].join("\n"),
            );
        },
    };
}

/** List the invitations to the person signed in. */
export function invitedCommand(context: Context): Verb {
    return {
        description: "List the invitations to you",
        json: true,
        async run({ json }) {
            // read the invitations to the person and their verified email
            const { session, user } = await context.launcher();
            const { items } = await session.accounts().user.invited({ id: user.userId });

            // list each with its role and what it shares, its identifier last
            const cells = items.map(({ id, relationship }) => {
                const role = "relation" in relationship ? relationship.relation : relationship.role;

                return [role, `${relationship.object.type} ${relationship.object.id}`, id];
            });
            const { terminal } = context;
            const none = terminal.isTerminal ? `  ${terminal.style("dim", "no invitations")}` : "";
            const listed =
                cells.length === 0 ? none : Render.grid(INVITATION_COLUMNS, cells, terminal, 2);
            await terminal.print(json ? JSON.stringify(items) : listed);
        },
    };
}

/** Accept an invitation to the person signed in. */
export function acceptCommand(context: Context): Verb {
    return {
        description: "Accept an invitation to you, which destack invited lists",
        arguments: [{ name: "invitation", usage: "invitation-…" }],
        json: true,
        async run({ arguments: { invitation }, json }) {
            // find the invitation among those to the person
            const native = await context.native();
            const { session, user } = await context.launcher();
            const { items } = await session.accounts().user.invited({ id: user.userId });
            const found = items.find((each) => each.id === invitation);
            if (found === undefined) {
                throw CliError.of(
                    "INVITATION_NOT_FOUND",
                    `invitation '${present(invitation, "the invitation argument")}' not found: list yours with destack invited`,
                );
            }

            // accept it in the scope it lives in
            await session.objects().invitation.accept({
                scope: found.relationship.object.scope,
                requestId: RequestId.create(native.environment),
                id: found.id,
            });
            const { object } = found.relationship;
            await context.terminal.print(
                json
                    ? JSON.stringify(found)
                    : context.terminal.event("Accepted", "done", `${object.type} '${object.id}'`),
            );
        },
    };
}
