import { Filter } from "@destack/db";
import {
    COMMAND_KIND,
    Command,
    CommandDescription,
    InstallationMount,
    ObjectTypePool,
    ObjectType,
    PackageMount,
    Presentation,
} from "@destack/object";
import { type ClientOptions, createClient, DynamicObjectClient } from "@destack/object/client";
import { DeclarationName, type PackageId } from "@destack/package";
import type { Localization } from "@destack/locale";
import { type Identifier, type JsonObject, type JsonValue, schema, Text } from "@destack/schema";
import { ObjectReference } from "@destack/sync";
import stackPackage from "@destack/stack/package";
import { Declaration } from "@destack/stack/client";
import { HOME_PATH, View } from "@destack/view/declare";
import { type CommandEntry, Palette, type PaletteEntry } from "@destack/view/palette";
import homePackage from "../package.ts";
import type { Environment } from "@destack/environment";
import objectPackage from "@destack/object/package";

/** The view of Home the command palette's page mounts, which it leaves out of its views. */
const PALETTE_VIEW = "command";

/** The most objects a picker lists at once: a screenful. */
const PICKER_LIMIT = 20;

/** A space of the person the palette reads: its identifier and its address. */
export interface PaletteSpace {
    /** The space. */
    readonly id: Identifier<"space">;
    /** The space's address. */
    readonly address: string;
}

/** An installation of a space the palette reaches, with the commands and views it declares. */
interface CatalogInstallation {
    /** The installation's name in its space. */
    readonly name: DeclarationName;
    /** The installed package. */
    readonly packageId: PackageId;
    /** The commands it declares. */
    readonly commands: readonly Command[];
    /** The views it declares. */
    readonly views: readonly DeclarationName[];
    /** The object types of its package, as the space's store describes them. */
    readonly objects: readonly ObjectType[];
}

/** An object a command's picker offers. */
export interface PickedObject {
    /** The object's identifier. */
    readonly id: string;
    /** The text naming it. */
    readonly title: string;
}

/** The installations of one of the person's spaces with their commands, views and object types. */
export class SpaceCatalog {
    /** The space. */
    readonly space: PaletteSpace;
    /** The enabled installations applying a revision, with what they declare. */
    readonly installations: readonly CatalogInstallation[];
    /** Reach a path below the page's origin, which forwards the space's calls as the person. */
    readonly #origin: (path: string) => ClientOptions;
    /** The environment the space's calls run in. */
    readonly #environment: Environment;

    /** Keep a space's installations. */
    constructor(
        space: PaletteSpace,
        installations: readonly CatalogInstallation[],
        connection: {
            readonly origin: (path: string) => ClientOptions;
            readonly environment: Environment;
        },
    ) {
        // set the space, its installations and the page's origin
        this.space = space;
        this.installations = installations;
        this.#origin = connection.origin;
        this.#environment = connection.environment;
    }

    /** Read what a space's installations declare and the types its store serves, at its home. */
    static async read(
        space: PaletteSpace,
        origin: (path: string) => ClientOptions,
        environment: Environment,
    ): Promise<SpaceCatalog> {
        // read the commands and views the space's installations declare, from their builds' index
        const home = (path: string) => origin(`${HOME_PATH}/${space.id}${path}`);
        const stack = createClient(
            stackPackage,
            home(PackageMount.path(stackPackage.id)),
            environment,
        );
        const objects = createClient(
            objectPackage,
            home(PackageMount.path(objectPackage.id)),
            environment,
        );
        const declarations = await Declaration.read(stack, objects, space.id, [
            COMMAND_KIND.identity,
            View.kind,
        ]);
        const commands = declarations.filter(
            ({ declaration }) => declaration.kind === COMMAND_KIND.identity.kind,
        );
        const views = declarations.filter(({ declaration }) => declaration.kind === View.kind.kind);

        // declare every type the space's store describes, each installation's by its package
        const store = new DynamicObjectClient({
            objects: [],
            scope: space.id,
            endpoint: home(""),
            environment,
        });
        const types = ObjectTypePool.of([]).declare(await store.describe());

        // list each installation's declared commands and views
        const declaring = new Map(
            declarations.map(({ rollout }) => [rollout.installation.name, rollout.installation]),
        );
        const installations = [...declaring.values()]
            .toSorted((left, right) => Text.compare(left.name, right.name))
            .map((installation) => {
                // read each command on the type it calls among the store's
                const { packageId } = installation;
                const declared = commands
                    .filter((entry) => entry.rollout.installation.name === installation.name)
                    .map(({ declaration }) => {
                        // find the type the command's description calls
                        const description = CommandDescription.parse(declaration.description);
                        const type = types.find(
                            (each) =>
                                each.package.id === description.packageId &&
                                each.name === description.type,
                        );
                        if (type === undefined) {
                            throw new TypeError(
                                `${space.address} serves no ${description.type} objects for ${declaration.name}`,
                            );
                        }

                        return Command.of(declaration.name, description, type);
                    });

                return {
                    name: DeclarationName.parse(installation.name),
                    packageId,
                    commands: declared,
                    views: views
                        .filter((entry) => entry.rollout.installation.name === installation.name)
                        .map(({ declaration }) => DeclarationName.parse(declaration.name)),
                    objects: types.filter((type) => type.package.id === packageId),
                };
            });

        return new SpaceCatalog(space, installations, { origin, environment });
    }

    /** List the commands and views the space's installations declare, in the person's language. */
    entries(
        keybindings: Readonly<Record<string, string | null>>,
        localization: Localization,
    ): PaletteEntry[] {
        return this.installations.flatMap((installed) => {
            // offer each command its installation declares
            const { name: installation, packageId } = installed;
            const commands = installed.commands.map((command): PaletteEntry => ({
                kind: "command",
                space: this.space.id,
                installation,
                packageId,
                name: DeclarationName.parse(command.name),
                title: localization.render(command.title),
                category:
                    command.category === undefined ? null : localization.render(command.category),
                typePackageId: command.object.package.id,
                type: command.object.name,
                method: command.method,
                keybinding: Palette.keybinding(
                    { packageId, name: command.name },
                    command.keybinding,
                    keybindings,
                ),
            }));

            // offer each view it mounts but the palette's own page
            const views = installed.views
                .filter((view) => !(packageId === homePackage.id && view === PALETTE_VIEW))
                .map((view): PaletteEntry => ({
                    kind: "view",
                    space: this.space.id,
                    installation,
                    packageId,
                    name: view,
                    title: `${installation} ${view}`,
                }));

            return [...commands, ...views];
        });
    }

    /** Find the command an entry offers among its installation's. */
    command(entry: CommandEntry): Command {
        // find the entry's installation and the command among its own
        const installed = this.installations.find((each) => each.name === entry.installation);
        const command = installed?.commands.find((each) => each.name === entry.name);
        if (command === undefined) {
            throw new TypeError(
                `${entry.installation} in ${this.space.address} declares no ${entry.name}`,
            );
        }

        return command;
    }

    /** List the objects a command applies to whose title contains a text. */
    async objects(entry: CommandEntry, title: string): Promise<PickedObject[]> {
        // name the field titling the type's objects
        const command = this.command(entry);
        const type = command.object;
        const { presentation } = type;
        const titled =
            presentation === undefined || title === ""
                ? undefined
                : Filter.parse(`${presentation.title}:${JSON.stringify(title)}`);

        // list the objects the command applies to whose title contains the text
        const { where, permission } = command.query();
        const rows = await this.#client(entry.installation, type).list(type.name, {
            where: titled === undefined ? where : { AND: [where, titled] },
            permission,
            limit: PICKER_LIMIT,
        });

        return rows.map((row) => ({
            id: schema.string().parse(row["id"]),
            title: Presentation.title(presentation, row),
        }));
    }

    /** Title objects of the space by their types' presentations, leaving out those none lists. */
    async titles(objects: readonly ObjectReference[]): Promise<Map<string, string>> {
        // list each installed type's objects by identifier through its installation
        const listed = await Promise.all(
            this.installations.flatMap((installed) =>
                installed.objects.map(async (type) => {
                    // find the objects of the type, listing none without any
                    const ofType = objects.filter(
                        (object) =>
                            object.packageId === type.package.id && object.type === type.name,
                    );
                    if (ofType.length === 0) {
                        return [];
                    }
                    const rows = await this.#client(installed.name, type).list(type.name, {
                        where: { id: { in: ofType.map((object) => object.id) } },
                    });

                    // title each object still listed by the type's presentation
                    return ofType.flatMap((object) => {
                        const row = rows.find((each) => each["id"] === object.id);

                        return row === undefined
                            ? []
                            : [
                                  [
                                      ObjectReference.key(object),
                                      Presentation.title(type.presentation, row),
                                  ] as const,
                              ];
                    });
                }),
            ),
        );

        return new Map(listed.flat());
    }

    /** Report whether a command applies to all of some objects, as the person may call it. */
    async appliesTo(entry: CommandEntry, ids: readonly string[]): Promise<boolean> {
        const command = this.command(entry);
        const rows = await this.#client(entry.installation, command.object).list(
            command.object.name,
            { ...command.query(ids), limit: ids.length },
        );

        return rows.length === ids.length;
    }

    /** Call a command with the input the person gave, on one target or on its scope. */
    call(entry: CommandEntry, input: JsonObject, target: string | undefined): Promise<JsonValue> {
        const command = this.command(entry);
        const given = target === undefined ? input : { ...input, id: target };

        return this.#client(entry.installation, command.object).call(
            command.object.name,
            command.method,
            command.input(given),
        );
    }

    /** Open an object type's client through an installation by name. */
    #client(name: string, type: ObjectType): DynamicObjectClient {
        return new DynamicObjectClient({
            objects: [type],
            scope: this.space.id,
            endpoint: this.#origin(
                `${HOME_PATH}/${this.space.id}${InstallationMount.path(this.space.id, name)}${PackageMount.path(type.package.id)}`,
            ),
            homes: (scope) => this.#origin(`${HOME_PATH}/${scope}`),
            environment: this.#environment,
        });
    }
}
