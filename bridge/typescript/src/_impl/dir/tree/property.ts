import type { ScopeKind } from "../../../_generated/dir/symbol/scope.js";
import type { StaticKey } from "../../../_generated/dir/symbol/key.js";
import type { SymbolKind, SymbolSpace } from "../../../_generated/dir/symbol/symbol.js";
import type { Member, MemberSlot, Property } from "../../../_generated/dir/tree/property.js";
import { KeyImpl } from "./key.js";

export const PropertyImpl = {
    /** Return whether a property has a static modifier. */
    hasStaticModifier(_property: Property): boolean {
        return false;
    },

    /** Return whether a property receives an implicit receiver. */
    hasImplicitReceiver(property: Property): boolean {
        return property.kind === "method";
    },

    /** Return whether a property belongs to an instance shape. */
    isInstanceMember(property: Property): boolean {
        return property.kind === "field" || property.kind === "method" || property.kind === "spread";
    },
};

export const MemberImpl = {
    /** Return the slot occupied by a declaration member. */
    slot(member: Member): MemberSlot | undefined {
        // associated members use their declared name as the member slot
        if (member.kind === "associatedType" || member.kind === "associatedConst") {
            return { kind: "key", key: { kind: "name", name: member.name } };
        }

        // fields use direct static keys when available
        else if (member.kind === "field") {
            const key = KeyImpl.directStaticKey(member.key);

            return key === undefined ? undefined : { kind: "key", key };
        }

        // keyed methods use direct static keys when available
        else if (member.kind === "method" && member.key !== undefined) {
            const key = KeyImpl.directStaticKey(member.key);

            return key === undefined ? undefined : { kind: "key", key };
        }

        // signature roles occupy their own well-known slots
        else if (member.kind === "method" && member.signature.role !== undefined) {
            if (member.signature.role === "constructor") {
                return { kind: "constructor" };
            }
            else if (member.signature.role === "new") {
                return { kind: "new" };
            }
            else if (member.signature.role === "call") {
                return { kind: member.signature.role };
            }
            else {
                return undefined;
            }
        }
        // other member forms do not occupy named slots
        else {
            return undefined;
        }
    },

    /** Return the symbol key declared by a member. */
    symbolKey(member: Member): StaticKey | undefined {
        const slot = MemberImpl.slot(member);

        return slot?.kind === "key" ? slot.key : undefined;
    },

    /** Return the symbol kind declared by a member. */
    symbolKind(member: Member): SymbolKind | undefined {
        // associated types declare associated type symbols
        if (member.kind === "associatedType") {
            return "associatedType";
        }

        // associated constants declare associated constant symbols
        else if (member.kind === "associatedConst") {
            return "associatedConst";
        }

        // fields declare variable symbols
        else if (member.kind === "field") {
            return "variable";
        }

        // keyed methods declare function symbols
        else if (member.kind === "method" && member.key !== undefined) {
            return "function";
        }
        // other members do not declare symbols
        else {
            return undefined;
        }
    },

    /** Return the symbol space declared by a member. */
    symbolSpace(member: Member): SymbolSpace | undefined {
        const kind = MemberImpl.symbolKind(member);

        // associated types bind in type space
        if (kind === "associatedType") {
            return "type";
        }

        // values bind in value space
        else if (kind === "associatedConst" || kind === "variable" || kind === "function") {
            return "value";
        }
        // members without symbols do not bind in a symbol space
        else {
            return undefined;
        }
    },

    /** Return the scope kind opened by a member. */
    symbolScopeKind(member: Member): ScopeKind | undefined {
        // associated types open type scopes
        if (member.kind === "associatedType") {
            return "type";
        }

        // methods open function scopes
        else if (member.kind === "method") {
            return "function";
        }
        // other members do not open scopes
        else {
            return undefined;
        }
    },

    /** Return whether a member has a static modifier. */
    hasStaticModifier(member: Member): boolean {
        // field and method members store staticness directly
        if (member.kind === "field" || member.kind === "method") {
            return member.isStatic;
        }

        // static blocks are always static
        else if (member.kind === "staticBlock") {
            return true;
        }
        // other members are not static
        else {
            return false;
        }
    },

    /** Return whether a member receives an implicit receiver. */
    hasImplicitReceiver(member: Member): boolean {
        return member.kind === "field" || member.kind === "method";
    },

    /** Return whether a member belongs to an instance shape. */
    isInstanceMember(member: Member): boolean {
        return MemberImpl.hasImplicitReceiver(member) && !MemberImpl.hasStaticModifier(member);
    },
};
