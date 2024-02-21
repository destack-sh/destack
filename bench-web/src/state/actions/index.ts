import { useBenchActions } from "@/state/actions/bench";
import { useFileActions } from "@/state/actions/file";
import { useOperationsActions } from "@/state/actions/operations";
import { useUserActions } from "@/state/actions/user";
import { defineStore } from "pinia";
import { computed, onBeforeUnmount, onMounted, ref, watch, type Ref } from "vue";

export type Action = {
  id: string;
  label: string;
  shortcuts: string[];
  registered: boolean;
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

      action.registered = true;
      this.actions.push(action);
    },
    update(action: Action): void {
      const index = this.actions.findIndex((a) => a.id === action.id);
      if (index === -1) {
        throw new Error(`action with id ${action.id} does not exist`);
      }
      this.actions[index] = action;
    },
    upsert(action: Action): void {
      const index = this.actions.findIndex((a) => a.id === action.id);
      if (index === -1) {
        this.actions.push(action);
      } else {
        this.actions[index] = action;
      }
    },
    remove(action: Action | string): void {
      if (typeof action === "string") {
        const actionObj = this.actions.find((a) => a.id === action);
        this.actions = this.actions.filter((a) => a.id !== action);
        if (actionObj != null) {
          actionObj.registered = false;
        }
      } else {
        this.actions = this.actions.filter((a) => a !== action);
        action.registered = false;
      }
    },
  },
});

export type RegisteredAction = {
  id: string;
  label: string | Ref<string>;
  shortcuts: string[];
  registered?: Ref<boolean>;
  enabled?: Ref<boolean | undefined>;
  apply: () => void;
};

export function provideGlobalAction(action: RegisteredAction): Ref<Action> {
  return provideAction(action, "global");
}

export function provideSingletonAction(action: RegisteredAction): Ref<Action> {
  return provideAction(action, "singleton");
}

export function provideAction(action: RegisteredAction, mode: "global" | "singleton" = "singleton"): Ref<Action> {
  const actionsStore = useActionsStore();

  // if this action can be reused just return a ref to the existing action
  if (mode == "global" && actionsStore.has(action.id)) {
    // TODO :Robustness: error if registered actions are different
    return computed(() => actionsStore.action(action.id));
  }

  console.debug(`provide action ${action.id} (${mode})`);
  const mounted = ref(false);

  function toResolvedAction() {
    return {
      id: action.id,
      label: typeof action.label === "string" ? action.label : action.label.value,
      shortcuts: action.shortcuts,
      registered: action.registered ? action.registered.value : true,
      enabled: action.enabled ? action.enabled.value ?? false : true,
      apply: action.apply,
    };
  }
  const resolvedAction: Ref<Action> = ref(toResolvedAction());

  if (action.registered == null || action.registered.value) {
    actionsStore.add(resolvedAction.value);
  }
  onMounted(() => {
    mounted.value = true;
  });
  // keep action updated in store
  watch(
    () => [mounted, action],
    () => {
      if (!mounted.value) return;
      if (!action.registered || action.registered.value) {
        resolvedAction.value = toResolvedAction();
        actionsStore.upsert(resolvedAction.value);
      } else {
        actionsStore.remove(resolvedAction.value);
      }
    },
    { deep: true }
  );
  onBeforeUnmount(() => {
    mounted.value = false;
    if (mode == "singleton") {
      // remove by instance, not id (in case of singleton actions)
      actionsStore.remove(resolvedAction.value);
    } else {
      actionsStore.remove(action.id);
    }
  });

  return resolvedAction;
}

export function useActions() {
  function get(id: string): Action | null {
    const actionsStore = useActionsStore();
    if (!actionsStore.has(id)) {
      return null;
    }
    return actionsStore.action(id);
  }

  function apply(id: string) {
    const actionsStore = useActionsStore();
    const action = actionsStore.action(id);
    action.apply();
  }

  return {
    user: useUserActions(),
    editor: useBenchActions(),
    operations: useOperationsActions(),
    file: useFileActions(),
    get,
    apply,
  };
}
