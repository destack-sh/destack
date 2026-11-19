import { Failure } from "@destack/error";
import { argument, t } from "@destack/locale";
import { ListPage, PackageMount } from "@destack/object";
import { createClient } from "@destack/object/client";
import objectPackage from "@destack/object/package";
import { type PackageId, PackageName } from "@destack/package";
import registryPackage from "@destack/registry/package";
import { LATEST_TAG, type Package, tag } from "@destack/registry/object";
import { RegistryError } from "@destack/registry/error";
import type { Identifier } from "@destack/schema";
import { ObjectReference, RequestId } from "@destack/sync";
import { Button } from "@destack/ui/button";
import {
    Dialog,
    DialogContent,
    DialogDescription,
    DialogHeader,
    DialogTitle,
    DialogTrigger,
} from "@destack/ui/dialog";
import { Empty, EmptyDescription, EmptyHeader, EmptyTitle } from "@destack/ui/empty";
import {
    Item,
    ItemActions,
    ItemContent,
    ItemDescription,
    ItemGroup,
    ItemTitle,
} from "@destack/ui/item";
import {
    createSignal,
    For,
    Show,
    useEnvironment,
    useLocale,
    useOrigin,
    useView,
} from "@destack/view";
import { ScopePath } from "@destack/view/declare";

/** Offer the residency's apps to install into a space, marking those it has. */
export function AppDirectoryDialog(properties: {
    /** The space the apps install into. */
    space: Identifier<"space">;
    /** The packages the space has installed. */
    installed: readonly PackageId[];
}) {
    // call the residency home's registry objects and the space's installations below the home path
    const locale = useLocale();
    const environment = useEnvironment();
    const origin = useOrigin();
    const residency = useView().residencyHome;
    const registry = createClient(
        registryPackage,
        origin(`${ScopePath.of(residency)}${PackageMount.path(registryPackage.id)}`),
        environment,
    );
    const objects = createClient(
        objectPackage,
        origin(`${ScopePath.of(properties.space)}${PackageMount.path(objectPackage.id)}`),
        environment,
    );
    const [apps, setApps] = createSignal<readonly Package[]>();
    const [failure, setFailure] = createSignal<string | undefined>(undefined);
    const [installing, setInstalling] = createSignal<string | undefined>(undefined);

    // read the public apps by name when the dialog opens
    const read = () =>
        Array.fromAsync(
            ListPage.all(registry.package.list, {
                spaceId: residency,
                where: { visibility: "public", role: "app" },
                orderBy: { name: "asc" },
            }),
        ).then(
            (listed) => {
                setFailure(undefined);
                setApps(listed);
            },
            (error: unknown) => setFailure(Failure.of(error).message),
        );

    // install one following its latest tag, as `destack install` does
    const install = async (app: Package) => {
        // find the latest tag, refusing a package without one
        const { items } = await registry.tag.list({
            spaceId: residency,
            where: { parentId: app.id, name: LATEST_TAG },
            limit: 1,
        });
        const [latest] = items;
        if (latest === undefined) {
            throw RegistryError.of(
                "RELEASE_NOT_FOUND",
                `release '${LATEST_TAG}' of '${app.name}' not found`,
            );
        }

        // create the space's installation of it under the package's local name, following the tag, as `destack install` does
        await objects.installation.create({
            scope: properties.space,
            requestId: RequestId.create(environment),
            packageId: app.id,
            ...ObjectReference.columns("follows", tag.reference(latest.scope, latest.id)),
            name: PackageName.split(app.name).local,
        });
    };

    // mark the app installing until the space has it, which the space's list follows, or show why it failed
    const add = (app: Package) => {
        setInstalling(app.name);
        install(app).then(
            () => setInstalling(undefined),
            (error: unknown) => {
                setInstalling(undefined);
                setFailure(Failure.of(error).message);
            },
        );
    };

    return (
        <Dialog onOpenChange={(isOpen) => isOpen && void read()}>
            <DialogTrigger variant="outline">{locale.render(t`Add app`)}</DialogTrigger>
            <DialogContent>
                <DialogHeader>
                    <DialogTitle>{locale.render(t`Add an app`)}</DialogTitle>
                    <DialogDescription>
                        {locale.render(t`Apps published in your residency's forge.`)}
                    </DialogDescription>
                </DialogHeader>

                {/* Why the apps could not be read or installed */}
                <Show when={failure()}>
                    {(message) => <ItemDescription>{message()}</ItemDescription>}
                </Show>

                {/* The apps, each with its install button */}
                <Show when={apps()}>
                    {(listed) => (
                        <Show when={listed().length > 0} fallback={<NoApps />}>
                            <ItemGroup>
                                <For each={listed()}>
                                    {(app) => (
                                        <Item>
                                            <ItemContent>
                                                <ItemTitle>{app.name}</ItemTitle>
                                            </ItemContent>
                                            <ItemActions>
                                                <Show
                                                    when={!properties.installed.includes(app.id)}
                                                    fallback={locale.render(t`Installed`)}
                                                >
                                                    <Button
                                                        disabled={installing() !== undefined}
                                                        onClick={() => add(app)}
                                                    >
                                                        {installing() === app.name
                                                            ? locale.render(t`Installing`)
                                                            : locale.render(t`Install`)}
                                                    </Button>
                                                </Show>
                                            </ItemActions>
                                        </Item>
                                    )}
                                </For>
                            </ItemGroup>
                        </Show>
                    )}
                </Show>
            </DialogContent>
        </Dialog>
    );
}

/** Explain an empty directory. */
function NoApps() {
    const locale = useLocale();

    return (
        <Empty>
            <EmptyHeader>
                <EmptyTitle>{locale.render(t`No apps yet`)}</EmptyTitle>
                <EmptyDescription>
                    {locale.render(
                        t`Apps appear here once their packages are published as ${argument("visibility")}.`,
                        { visibility: "public" },
                    )}
                </EmptyDescription>
            </EmptyHeader>
        </Empty>
    );
}
