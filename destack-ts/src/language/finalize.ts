import { NODE_DEFINITIONS } from "@destack/language/constants";
import { NODE_CLASS_BY_TYPE } from "@destack/language/registry";

let __isFinalized__ = false;

/** Finalize the Destack language SDK. */
export function finalize(): void {
  if (__isFinalized__) {
    return;
  }
  __isFinalized__ = true;

	// hookup all the property definitions
	for (const nodeDefinition of NODE_DEFINITIONS) {
		const nodeClass = NODE_CLASS_BY_TYPE[nodeDefinition.type];
		nodeClass.__properties__ = {};
		nodeClass.__propertiesById__ = {};
		for (const propertyDefinition of nodeDefinition.properties) {
			nodeClass.__properties__[propertyDefinition.name] = propertyDefinition;
			nodeClass.__propertiesById__[propertyDefinition.id] = propertyDefinition;
		}
	}
}
