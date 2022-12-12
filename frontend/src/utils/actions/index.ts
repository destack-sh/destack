import { useOperationsActions } from "@/utils/actions/editor";
import { useFileActions } from "@/utils/actions/file";
import { useEditorActions } from "@/utils/actions/operations";
import { useVersionActions } from "@/utils/actions/version";
import { defineStore } from "pinia";
import { computed, onMounted, onUnmounted, ref, watchEffect, type Ref } from "vue";

export type Action = {
  id: string;
  label: string;
  shortcuts: string[];
  enabled: boolean;
  apply: () => void;
};

export const useActionsStore = defineStore("actions", {
  state: () => ({
    actions: [] as Action[],
  }),
  getters: {
    all(state): Action[] {
      return state.actions;
    },
    available(state): Action[] {
      return state.actions.filter((a) => a.enabled);
    },
    action(state): (id: string) => Action {
      return (id: string) => {
        const action = state.actions.find((a) => a.id === id);
        if (action == null) {
          throw new Error(`action with id ${id} does not exist`);
        }
        return action;
      };
    },
    has(state): (id: string) => boolean {
      return (id: string) => state.actions.find((a) => a.id === id) != null;
    },
  },
  actions: {
    add(action: Action): void {
      // check if action with id already exists
      if (this.actions.find((a) => a.id === action.id) != null) {
        throw new Error(`action with id ${action.id} already exists`);
      }

      this.actions.push(action);
    },
    update(action: Action): void {
      const index = this.actions.findIndex((a) => a.id === action.id);
      if (index === -1) {
        throw new Error(`action with id ${action.id} does not exist`);
      }
      this.actions[index] = action;
    },
    remove(action: Action | string): void {
      if (typeof action === "string") {
        this.actions = this.actions.filter((a) => a.id !== action);
      } else {
        this.actions = this.actions.filter((a) => a !== action);
      }
    },
  },
});

export type RegisteredAction = {
  id: string;
  label: string | Ref<string>;
  shortcuts: string[];
  enabled?: Ref<boolean>;
  apply: () => void;
};

export function provideSharedAction(action: RegisteredAction): Ref<Action> {
  console.log(`provide shared action ${action.id}`);
  return provideAction(action, true);
}

export function provideAction(action: RegisteredAction, shared?: boolean): Ref<Action> {
  const actionsStore = useActionsStore();

  // if this action can be reused just return a ref to the existing action
  if (shared && actionsStore.has(action.id)) {
    // TODO @Cleanup: error if registered actions are different
    return computed(() => actionsStore.action(action.id));
  }

  const mounted = ref(false);

  function toResolvedAction() {
    return {
      id: action.id,
      label: typeof action.label === "string" ? action.label : action.label.value,
      shortcuts: action.shortcuts,
      enabled: action.enabled ? action.enabled.value : true,
      apply: action.apply,
    };
  }
  const resolvedAction: Ref<Action> = ref(toResolvedAction());

  actionsStore.add(resolvedAction.value);
  onMounted(() => {
    mounted.value = true;
  });
  // keep action updated in store
  watchEffect(() => {
    if (!mounted.value) return;
    resolvedAction.value = toResolvedAction();
    actionsStore.update(resolvedAction.value);
  });
  onUnmounted(() => {
    mounted.value = false;
    actionsStore.remove(action.id);
  });

  return resolvedAction;
}

export function useActions() {
  return {
    editor: useEditorActions(),
    operations: useOperationsActions(),
    version: useVersionActions(),
    file: useFileActions(),
  };
}
