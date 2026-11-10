import * as style from "@destack/style";
import { t } from "@destack/locale";
import { text } from "@destack/theme/text";
import { color, space } from "@destack/theme/tokens.stylex";
import { type JSX, Show, useLocale } from "@destack/view";
import { Badge } from "../badge/index.ts";
import { Button } from "../button/index.ts";
import {
    Item,
    ItemActions,
    ItemContent,
    ItemDescription,
    ItemFooter,
    ItemGroup,
    ItemTitle,
} from "../item/index.ts";
import { Stack } from "../stack/index.ts";

/** The styles of a permission list. */
const styles = style.create({
    title: { color: color.mutedForeground, margin: 0 },
    list: { margin: 0, paddingInlineStart: 0, listStyle: "none" },
    permission: { display: "flex", gap: space[2], alignItems: "baseline" },
});

/** The properties of a consent prompt: what is asked, and the person's answer. */
export interface ConsentPromptProperties {
    /** Whether a decision is on its way, which disables the buttons. */
    readonly isPending?: boolean | undefined;
    /** Allow what is asked until the person removes it. */
    readonly onAllow: () => void;
    /** Allow it for this sign-in alone, absent where only a lasting answer fits. */
    readonly onAllowOnce?: (() => void) | undefined;
    /** Refuse it. */
    readonly onDeny: () => void;
    /** What is asked, usually a `PermissionList`. */
    readonly children?: JSX.Element;
}

/** Render what an application asks for, then allow it, allow it once, or refuse it. */
export function ConsentPrompt(properties: ConsentPromptProperties): JSX.Element {
    const locale = useLocale();
    const isPending = () => properties.isPending === true;

    return (
        <Stack space="4">
            {properties.children}
            <Stack space="2">
                <Button disabled={isPending()} onClick={() => properties.onAllow()}>
                    {locale.render(t`Allow`)}
                </Button>
                <Show when={properties.onAllowOnce}>
                    {(allowOnce) => (
                        <Button
                            variant="outline"
                            disabled={isPending()}
                            onClick={() => allowOnce()()}
                        >
                            {locale.render(t`Only this time`)}
                        </Button>
                    )}
                </Show>
                <Button variant="ghost" disabled={isPending()} onClick={() => properties.onDeny()}>
                    {locale.render(t`Don't allow`)}
                </Button>
            </Stack>
        </Stack>
    );
}

/** The properties of a permission list. */
export interface PermissionListProperties {
    /** The permissions, one `PermissionGroup` per place they apply in. */
    readonly children?: JSX.Element;
}

/** Render permissions grouped by where they apply. */
export function PermissionList(properties: PermissionListProperties): JSX.Element {
    return <Stack space="3">{properties.children}</Stack>;
}

/** The properties of a group of permissions applying in one place. */
export interface PermissionGroupProperties {
    /** Where the permissions apply, as the person reads it, such as "In your home". */
    readonly title: string;
    /** The permissions, one `Permission` each. */
    readonly children?: JSX.Element;
}

/** Render the permissions applying in one place under its title. */
export function PermissionGroup(properties: PermissionGroupProperties): JSX.Element {
    return (
        <Stack space="1" role="group" aria-label={properties.title}>
            <p {...style.attributes([text.caption, styles.title])}>{properties.title}</p>
            <ul {...style.attributes(styles.list)}>{properties.children}</ul>
        </Stack>
    );
}

/** The properties of one permission. */
export interface PermissionProperties {
    /** What the permission lets the application do, as the person reads it. */
    readonly description: string;
    /** Whether the person allowed it before, which marks it. */
    readonly isGranted?: boolean | undefined;
}

/** Render one permission, marked when the person allowed it before. */
export function Permission(properties: PermissionProperties): JSX.Element {
    const locale = useLocale();

    return (
        <li {...style.attributes([text.callout, styles.permission])}>
            {properties.description}
            <Show when={properties.isGranted === true}>
                <Badge variant="secondary">{locale.render(t`Already allowed`)}</Badge>
            </Show>
        </li>
    );
}

/** The properties of a grant list. */
export interface GrantListProperties {
    /** The grants, one `Grant` each. */
    readonly children?: JSX.Element;
}

/** Render the applications a person lets act for them. */
export function GrantList(properties: GrantListProperties): JSX.Element {
    return <ItemGroup>{properties.children}</ItemGroup>;
}

/** The properties of one grant. */
export interface GrantProperties {
    /** The application's name. */
    readonly name: string;
    /** Where it is installed and since when, as the person reads it. */
    readonly description: string;
    /** Whether the removal is on its way, which disables the button. */
    readonly isPending?: boolean | undefined;
    /** Remove the application's access. */
    readonly onRemove: () => void;
    /** What it may do, usually a `PermissionList`. */
    readonly children?: JSX.Element;
}

/** Render an application a person lets act for them, what it may do, and its removal. */
export function Grant(properties: GrantProperties): JSX.Element {
    const locale = useLocale();

    return (
        <Item variant="outline">
            <ItemContent>
                <ItemTitle>{properties.name}</ItemTitle>
                <ItemDescription>{properties.description}</ItemDescription>
            </ItemContent>
            <ItemActions>
                <Button
                    variant="outline"
                    size="sm"
                    disabled={properties.isPending === true}
                    onClick={() => properties.onRemove()}
                >
                    {locale.render(t`Remove access`)}
                </Button>
            </ItemActions>
            <Show when={properties.children}>
                <ItemFooter>{properties.children}</ItemFooter>
            </Show>
        </Item>
    );
}
