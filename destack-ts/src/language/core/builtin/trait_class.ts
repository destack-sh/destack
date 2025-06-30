import { TraitType } from "@destack/language/core/builtin/common";
import type { PropertyDefinition, TraitDefinition } from "@destack/language/core/common/meta";

/** Internal base class for Trait companion objects.*/
export class TraitClass<N = any, T extends TraitType = TraitType> {
  readonly metatype: T;
  __definition__: TraitDefinition;
  __properties__: Record<string, PropertyDefinition>;
  __propertiesById__: Record<number, PropertyDefinition>;

  constructor(metatype: any) {
    this.metatype = metatype;
    this.__definition__ = null as any; // set later;
    this.__properties__ = {};
    this.__propertiesById__ = {};
  }

  /** Get a PropertyDefinition or CustomProperty by name. */
  property(name: string): PropertyDefinition {
    const prop = this.__properties__[name];
    if (!prop) {
      throw new Error(`Property ${name} not found on ${this.constructor.name}`);
    }
    return prop;
  }
}
