import type { IconData } from "@/proto/wire";
import { fireAction, getAction, type Action, type ActionBuiltinId } from "@/system/action";
import { toValue } from "vue";

export type MenuInfo = {
  icon?: string | IconData;
  title?: string;
  text?: string;
  items: MenuItem[];
};

export type MenuItem = {
  id: string;
  icon?: string | IconData;
  title: string;
  category?: string;
  shortcuts?: string[];
  isDisabled?: boolean;
  isLoading?: boolean;
  action: (() => void) | MenuInfo;
};

export function menuItemFromAction(actionOrId: Action | ActionBuiltinId, override?: Partial<MenuItem>): MenuItem {
  const action = typeof actionOrId === "string" ? getAction(actionOrId) : actionOrId;
  return {
    id: action.id,
    icon: action.icon?.name,
    title: toValue(action.title),
    shortcuts: action.shortcuts,
    isDisabled: action.enabled != null && !action.enabled.value,
    category: action.category,
    action: () => fireAction(action),
    ...override,
  };
}
