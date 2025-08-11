import { NodeType, StructType } from "@destack/language/core/builtin/builtin";
import type { Entity } from "@destack/language/core/builtin/entity";
import type { Event } from "@destack/language/core/builtin/event";
import type { NodeReference } from "@destack/language/core/builtin/relation";
import type { IsRunnable } from "@destack/language/core/builtin/trait";
import { Method, MethodDefinition } from "@destack/language/core/common/method";
import {
  registerNodeClass,
  registerStructClass,
  STRUCT_CLASS_BY_TYPE,
} from "@destack/language/registry";
import { hashBool, hashInt, hashString } from "@destack/utils/hash";

/* ==== DESTACK_GENERATED_START:STRUCT:40100 ==== */
/**
 * Definition of a builtin Action.
 */
export class ActionDefinition extends MethodDefinition {
  static metatype: StructType = StructType.ACTION_DEFINITION;
  static __isFrozen__: boolean = true;

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.id === other.id)) {
      return false;
    }
    if (!(this.type === other.type)) {
      return false;
    }
    if (!(this.name === other.name)) {
      return false;
    }
    if (!(this.description === other.description)) {
      return false;
    }
    if (this.properties.length != other.properties.length) {
      return false;
    }
    for (let i = 0; i < this.properties.length; i++) {
      if (!this.properties[i].equals(other.properties[i])) {
        return false;
      }
    }

    if (!(this.cardinality === other.cardinality)) {
      return false;
    }
    if (this.platforms.length != other.platforms.length) {
      return false;
    }
    for (let i = 0; i < this.platforms.length; i++) {
      if (!(this.platforms[i] === other.platforms[i])) {
        return false;
      }
    }

    if (this.languages.length != other.languages.length) {
      return false;
    }
    for (let i = 0; i < this.languages.length; i++) {
      if (!(this.languages[i] === other.languages[i])) {
        return false;
      }
    }

    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`id=${this.id}`);
      if (this.description != null) {
        propertyReprs.push(`description=${`"${this.description}"`}`);
      }
      // @ts-expect-error(readonly) */
      this._repr = `<ActionDefinition ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash != null) {
      return this._hash;
    }
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + hashInt(this.id)) & 0xffffffff;
    h = (h * 31 + this.type) & 0xffffffff;
    h = (h * 31 + hashString(this.name)) & 0xffffffff;
    if (this.description != null) {
      h = (h * 31 + hashString(this.description)) & 0xffffffff;
    }
    if (this.properties && this.properties.length > 0) {
      for (const _item of this.properties) {
        h = (h * 31 + _item.hash()) & 0xffffffff;
      }
    }
    h = (h * 31 + this.cardinality) & 0xffffffff;
    if (this.platforms && this.platforms.length > 0) {
      for (const _item of this.platforms) {
        h = (h * 31 + _item) & 0xffffffff;
      }
    }
    if (this.languages && this.languages.length > 0) {
      for (const _item of this.languages) {
        h = (h * 31 + _item) & 0xffffffff;
      }
    }
    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.ACTION_DEFINITION, ActionDefinition);
/* ==== DESTACK_GENERATED_END:STRUCT:40100 ==== */

/* ==== DESTACK_GENERATED_START:NODE:40100 ==== */
/**
 * An implementation of a unit of work, usually expressed with Code or some tool.
 * May defer to a builtin or some other service in a separate system.
 */
export class Action extends Method implements IsRunnable {
  static metatype: NodeType = NodeType.ACTION;

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this._type === other._type)) {
      return false;
    }
    if (
      (this._text == null) !== (other._text == null) ||
      (this._text != null && !this._text.equals(other._text))
    ) {
      return false;
    }
    if (!(this._cardinality === other._cardinality)) {
      return false;
    }
    if (this._platforms.length != other._platforms.length) {
      return false;
    }
    for (let i = 0; i < this._platforms.length; i++) {
      if (!(this._platforms[i] === other._platforms[i])) {
        return false;
      }
    }

    if (this._languages.length != other._languages.length) {
      return false;
    }
    for (let i = 0; i < this._languages.length; i++) {
      if (!(this._languages[i] === other._languages[i])) {
        return false;
      }
    }

    if (!(this.definitionRef?.id === other.definitionRef?.id)) {
      return false;
    }
    if (!(this._ownedByRef?.id === other._ownedByRef?.id)) {
      return false;
    }
    if (!(this._name === other._name)) {
      return false;
    }
    if (JSON.stringify(this._customValues) !== JSON.stringify(other._customValues)) {
      return false;
    }
    if (!(this._scriptRef?.id === other._scriptRef?.id)) {
      return false;
    }
    if (!(this.isExtensible === other.isExtensible)) {
      return false;
    }
    if (!(this.sourceRef?.id === other.sourceRef?.id)) {
      return false;
    }
    if (!(this._key === other._key)) {
      return false;
    }
    if (!(this.spaceRef.id === other.spaceRef.id)) {
      return false;
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this._type) & 0xffffffff;
    if (this._text != null) {
      h = (h * 31 + this._text.hash()) & 0xffffffff;
    }
    h = (h * 31 + this._cardinality) & 0xffffffff;
    if (this._platforms && this._platforms.length > 0) {
      for (const _item of this._platforms) {
        h = (h * 31 + _item) & 0xffffffff;
      }
    }
    if (this._languages && this._languages.length > 0) {
      for (const _item of this._languages) {
        h = (h * 31 + _item) & 0xffffffff;
      }
    }
    if (this.parentRef != null) {
      h = (h * 31 + hashString(this.parentRef.id)) & 0xffffffff;
    }
    if (this.definitionRef != null) {
      h = (h * 31 + hashString(this.definitionRef.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.createdAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    h = (h * 31 + hashString(this.createdByRef.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.updatedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    h = (h * 31 + hashString(this.updatedByRef.id)) & 0xffffffff;
    if (this.deletedAt != null) {
      h = (h * 31 + hashString(this.deletedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    }
    if (this._ownedByRef != null) {
      h = (h * 31 + hashString(this._ownedByRef.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this._name)) & 0xffffffff;
    h = (h * 31 + hashString(this.orderKey)) & 0xffffffff;
    if (this._customValues && Object.keys(this._customValues).length > 0) {
      for (const [_key, _value] of Object.entries(this._customValues)) {
        h = (h * 31 + hashString(_key.toString())) & 0xffffffff;
        h = (h * 31 + _value.hash()) & 0xffffffff;
      }
    }
    if (this._scriptRef != null) {
      h = (h * 31 + hashString(this._scriptRef.id)) & 0xffffffff;
    }
    if (this.isExtensible != null) {
      h = (h * 31 + hashBool(this.isExtensible)) & 0xffffffff;
    }
    if (this.sourceRef != null) {
      h = (h * 31 + hashString(this.sourceRef.id)) & 0xffffffff;
    }
    if (this._key != null) {
      h = (h * 31 + hashString(this._key)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;
    h = (h * 31 + hashString(this.spaceRef.id)) & 0xffffffff;

    return h;
  }

  __toRef__(): NodeReference {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new _NodeReference({
      type: NodeType.ACTION,
      id: this.id,
      spaceId: this.spaceRef.id,
      definitionId: this.definitionRef?.id ?? null,
      branchId: this.branchRef.id,
      snapshotId: this.snapshotRef.id,
      _session: this._session,
    });
  }

  get _pathKey(): string {
    return this.name;
  }

  get path(): string {
    const pathParts: string[] = [];
    let node: Entity | Event | null = this;
    let lastNode: Entity | Event | null = this;
    while (node != null) {
      pathParts.push(node._pathKey);
      lastNode = node;
      node = node.parent;
    }
    if (!lastNode.isRoot) {
      pathParts.push("<detached>");
    }
    return pathParts.reverse().join("/");
  }

  repr(): string {
    const propertyReprs: string[] = [];
    if (this.ownedBy != null) {
      propertyReprs.push(`ownedBy=${this.ownedBy?.repr()}`);
    }
    propertyReprs.push(`name=${`"${this.name}"`}`);
    return `<Action "${this.path}" ${propertyReprs.join(" ")}>`;
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.ACTION, Action);
/* ==== DESTACK_GENERATED_END:NODE:40100 ==== */
