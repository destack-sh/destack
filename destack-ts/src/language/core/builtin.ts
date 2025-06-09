import { makeScope, nodeReference } from "@/proto/wiring";
import { NodeType } from "@/proto/wire";

// builtin destackes :Builtins
// destack
export const DESTACK_SLUG = "destack";
export const DESTACK_ID = "11111111-1111-1111-1111-000000000000";
export const DESTACK_PTR = nodeReference(NodeType.SPACE, DESTACK_ID, {
  destackId: DESTACK_ID,
});
export const DESTACK_DESTACK_PACKAGE_ID = "11111111-1111-1111-1111-000000000001";
export const DESTACK_DESTACK_PACKAGE_PTR = nodeReference(NodeType.PACKAGE, DESTACK_DESTACK_PACKAGE_ID, {
  destackId: DESTACK_ID,
});
export const DESTACK_SCOPE = makeScope(DESTACK_PTR);
// system
export const SYSTEM_SLUG = "system";
export const SYSTEM_ID = "22222222-2222-2222-2222-000000000000";
export const SYSTEM_PTR = nodeReference(NodeType.SPACE, SYSTEM_ID, {
  destackId: SYSTEM_ID,
});
export const SYSTEM_SYSTEM_PACKAGE_ID = "22222222-2222-2222-2222-000000000001";
export const SYSTEM_SYSTEM_PACKAGE_PTR = nodeReference(NodeType.PACKAGE, SYSTEM_SYSTEM_PACKAGE_ID, {
  destackId: SYSTEM_ID,
});
export const SYSTEM_SCOPE = makeScope(SYSTEM_SYSTEM_PACKAGE_PTR);

// specific :Builtins
export const DESTACK_DESTACK_AGENT_ID = "6d642818-83ac-50ef-b662-32643ca3393a";
export const DESTACK_DESTACK_AGENT_PTR = nodeReference(NodeType.AGENT, DESTACK_DESTACK_AGENT_ID, {
  ck: DESTACK_DESTACK_AGENT_ID,
  destackId: DESTACK_ID,
});
export const DESTACK_DESTACK_UBUNTU_DESKTOP_ID = "66533e7a-782e-516c-8846-282843480f31";
export const DESTACK_DESTACK_UBUNTU_DESKTOP_PTR = nodeReference(NodeType.COMPUTER, DESTACK_DESTACK_UBUNTU_DESKTOP_ID, {
  ck: DESTACK_DESTACK_UBUNTU_DESKTOP_ID,
  destackId: DESTACK_ID,
});
