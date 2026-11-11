import { OAUTH_SCOPES } from "@destack/account/object";
import type { GrantRequest, RequestedPermission } from "@destack/account/authentication";
import { argument, type Message, t } from "@destack/locale";
import { Permission, PermissionGroup, PermissionList } from "@destack/ui/consent";
import { For, Show, useLocale } from "@destack/view";

/** What each scope the issuer grants lets an application do, as a person reads it. */
const SCOPE_DESCRIPTIONS: Readonly<Record<string, Message>> = {
    openid: t`Know who you are`,
    profile: t`See your name and picture`,
    email: t`See your email address`,
    offline_access: t`Stay signed in`,
} satisfies Readonly<Record<(typeof OAUTH_SCOPES)[number], Message>>;

/** Where a permission applies. */
type Place = RequestedPermission["within"];

/** The places a grant's permissions apply in, in the order a person reads them. */
const PLACES: readonly Place[] = ["space", "home", "account", "universe"];

/** A permission of a grant, marked once the person granted it before. */
type ShownPermission = RequestedPermission & { readonly isGranted?: boolean };

/** Render what an application may do: its grant's permissions by place, else its scopes. */
export function GrantPermissions(properties: {
    /** The grant, null for an application asking scopes alone. */
    readonly grant:
        | (Omit<GrantRequest, "permissions"> & {
              readonly permissions: readonly ShownPermission[];
          })
        | null;
    /** The scopes, shown for an application without a grant. */
    readonly scopes: readonly string[];
}) {
    // read the person's locale
    const locale = useLocale();

    // title each place a grant's permissions apply in
    const place = (within: Place, space: string) => {
        if (within === "space") {
            return locale.render(t`In ${argument("space")}`, { space });
        } else if (within === "home") {
            return locale.render(t`In your home`);
        } else if (within === "account") {
            return locale.render(t`In your account`);
        } else {
            return locale.render(t`Everywhere`);
        }
    };

    // read a permission with how far it reaches: one object, one method, asking each time
    const read = (permission: ShownPermission) => {
        // narrow the description to one object
        const described = locale.render(permission.description);
        const narrowed =
            permission.objectId === undefined
                ? described
                : locale.render(t`${argument("permission")}, of one item only`, {
                      permission: described,
                  });

        // narrow it to one method
        const called =
            permission.method === undefined
                ? narrowed
                : locale.render(t`${argument("permission")}, through ${argument("method")}`, {
                      permission: narrowed,
                      method: permission.method,
                  });

        // ask each time for a permission in approval mode
        return permission.mode === "approval"
            ? locale.render(t`${argument("permission")}, asking you each time`, {
                  permission: called,
              })
            : called;
    };

    // describe a scope, refusing one the issuer does not grant
    const scope = (name: string) => {
        const message = SCOPE_DESCRIPTIONS[name];
        if (message === undefined) {
            throw new TypeError(`the issuer grants no scope '${name}'`);
        }

        return locale.render(message);
    };

    return (
        <PermissionList>
            <Show
                when={properties.grant}
                fallback={
                    <PermissionGroup title={locale.render(t`Your account`)}>
                        <For each={properties.scopes}>
                            {(name) => <Permission description={scope(name)} />}
                        </For>
                    </PermissionGroup>
                }
            >
                {(grant) => (
                    <For
                        each={PLACES.filter((within) =>
                            grant().permissions.some((each) => each.within === within),
                        )}
                    >
                        {(within) => (
                            <PermissionGroup title={place(within, grant().space)}>
                                <For
                                    each={grant().permissions.filter(
                                        (each) => each.within === within,
                                    )}
                                >
                                    {(permission) => (
                                        <Permission
                                            description={read(permission)}
                                            isGranted={permission.isGranted}
                                        />
                                    )}
                                </For>
                            </PermissionGroup>
                        )}
                    </For>
                )}
            </Show>
        </PermissionList>
    );
}
