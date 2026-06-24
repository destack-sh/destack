import type { StringId } from "../../../_generated/core/string.js";
import type { StaticKey } from "../../../_generated/dir/symbol/key.js";
import type { ExportSelector } from "../../../_generated/dir/symbol/export.js";
import type { SymbolKind } from "../../../_generated/dir/symbol/symbol.js";
import type { DependencyItem } from "../../../_generated/dir/tree/dependency.js";

export const DependencyItemImpl = {
    /** Return the local key introduced by an import binding. */
    localStringKey(item: DependencyItem): StringId | undefined {
        // non-binding items do not introduce local names
        if (item.kind !== "binding") {
            return undefined;
        }

        return item.alias ?? nameString(item.name);
    },

    /** Return the local alias name for an import binding. */
    localImportAliasName(item: DependencyItem): StringId | undefined {
        // non-binding items do not introduce local names
        if (item.kind !== "binding") {
            return undefined;
        }

        // default imports may use their imported name as the local alias
        if (item.binding === "default") {
            return nameString(item.name) ?? item.alias;
        }

        return item.alias;
    },

    /** Return the alias name for a default import binding. */
    defaultImportAliasName(item: DependencyItem): StringId | undefined {
        // only default bindings have default import aliases
        if (item.kind !== "binding" || item.binding !== "default") {
            return undefined;
        }

        return nameString(item.name);
    },

    /** Return the symbol key introduced by an import binding. */
    symbolKey(item: DependencyItem): StaticKey | undefined {
        // non-binding items do not introduce local names
        if (item.kind !== "binding") {
            return undefined;
        }

        // explicit aliases become the local symbol key
        if (item.alias !== undefined) {
            return { kind: "name", name: item.alias };
        }

        return nameStaticKey(item.name);
    },

    /** Return the symbol kind introduced by an import binding. */
    symbolKind(item: DependencyItem): SymbolKind | undefined {
        return item.kind === "binding" ? "import" : undefined;
    },

    /** Return the exported source key. */
    exportSourceKey(item: DependencyItem): StaticKey | undefined {
        return item.kind === "binding" ? nameStaticKey(item.name) : undefined;
    },

    /** Return the export selector represented by the item. */
    exportSelector(item: DependencyItem): ExportSelector | undefined {
        // non-binding items do not export a selected binding
        if (item.kind !== "binding") {
            return undefined;
        }

        // default bindings export the default selector
        if (item.binding === "default") {
            return { kind: "default" };
        }

        // namespace bindings export the namespace selector
        else if (item.binding === "namespace") {
            return { kind: "namespace" };
        }
        // named bindings export their named selector if one exists
        else {
            const key = nameStaticKey(item.name);

            return key === undefined ? undefined : { kind: "named", named: key };
        }
    },

    /** Return whether the item exports all names from another module. */
    isStarExport(item: DependencyItem): boolean {
        return item.kind === "binding" && item.binding === "namespace" && item.alias === undefined;
    },

    /** Return whether the item exports the imported default value. */
    isDefaultValueExport(item: DependencyItem): boolean {
        return item.kind === "binding" && item.binding === "default" && item.value !== undefined;
    },
};

/** Return the string value of one dependency name. */
function nameString(name: DependencyName | undefined): StringId | undefined {
    // missing and numeric names do not have string values
    if (name === undefined || name.kind === "index") {
        return undefined;
    }

    // identifiers store their string in the identifier field
    if (name.kind === "identifier") {
        return name.identifier;
    }
    // string names store their string in the string field
    else {
        return name.string;
    }
}

/** Return the static key value of one dependency name. */
function nameStaticKey(name: DependencyName | undefined): StaticKey | undefined {
    // missing names do not have static keys
    if (name === undefined) {
        return undefined;
    }

    // numeric names map to index keys
    else if (name.kind === "index") {
        return { kind: "index", index: name.index };
    }
    // string-like names map to name keys
    else {
        return { kind: "name", name: nameString(name)! };
    }
}

type DependencyName = Extract<DependencyItem, { readonly kind: "binding" }>["name"];
