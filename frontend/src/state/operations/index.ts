import { useFileOps } from "@/state/operations/file";
import { useRuntimeOps } from "@/state/operations/runtime";
import { useStatementOps } from "@/state/operations/statement";
import { useSymbolContentOps } from "@/state/operations/symbol";
import { useUserOps } from "@/state/operations/user";
import { useProjectVersionOps } from "@/state/operations/version";
import { createSharedComposable } from "@vueuse/shared";
import { DateTime } from "luxon";
import { defineStore } from "pinia";
import { ref, type Ref } from "vue";

export type Operation<T> = {
  id?: string;
  type: string;
  startedAt?: DateTime;
  key?: string | Record<string, string>;
  stateless?: boolean; // whether the operation mutates synced state (true by default)
  do(): Promise<T>;
  undo?(): Promise<unknown>;
};

const COMPLETED_STACK_SIZE = 500;
const STALE_TIME_SECONDS = 1;

// keep reactive now (can't use useNow because it attaches to component)
const now: Ref<DateTime> = ref(DateTime.now());
setInterval(() => (now.value = DateTime.now()), 100);

export const useOperationsStore = defineStore("operations", {
  state: () => ({
    inflight: [] as Operation<unknown>[],
    completed: [] as Operation<unknown>[],
    undoStack: [] as Operation<unknown>[],
    redoStack: [] as Operation<unknown>[],
  }),
  getters: {
    inflightLike(
      state
    ): (filters: {
      types?: string[];
      typesNot?: string[];
      keys?: string[];
      stale?: boolean;
      stateless?: boolean;
    }) => Operation<unknown>[] {
      return (filters) => {
        const oneSecondAgo = filters.stale ? now.value.minus({ seconds: STALE_TIME_SECONDS }) : null;
        return state.inflight.filter(
          (op) =>
            (!filters.types || filters.types.includes(op.type)) &&
            (!filters.typesNot || !filters.typesNot.includes(op.type)) &&
            (!filters.keys || (op.key != null && JSON.stringify(op.key).match(new RegExp(filters.keys.join("|"))))) &&
            (!filters.stale || (op.startedAt != null && op.startedAt < oneSecondAgo)) &&
            (filters.stateless === undefined || (op.stateless ?? false) == filters.stateless)
        );
      };
    },
    hasInflightLike(
      state
    ): (filters: {
      types?: string[];
      typesNot?: string[];
      keys?: string[];
      stale?: boolean;
      stateless?: boolean;
    }) => boolean {
      return (filters) => this.inflightLike(filters).length > 0;
    },
    hasInflight(state): boolean {
      return state.inflight.length > 0;
    },
    completedLike(state): (type: string, key?: string) => Operation<unknown> {
      return (type, key) => state.completed.filter((op) => op.type === type && (!key || op.key === key))[0];
    },
    hasCompletedLike(state): (type: string, key?: string) => boolean {
      return (type, key) => state.completed.some((op) => op.type === type && (!key || op.key === key));
    },
    canUndo(state): boolean {
      return state.undoStack.length > 0;
    },
    canRedo(state): boolean {
      return state.redoStack.length > 0;
    },
  },
  actions: {
    reset(): void {
      this.$reset();
    },

    async _do<T>(operation: Operation<T>, undo?: boolean): Promise<T | void | null> {
      operation = { ...operation, startedAt: DateTime.now() };
      this.inflight.push(operation);
      try {
        let ret;
        if (!undo) {
          ret = await operation.do();
        } else {
          if (operation.undo == null) {
            throw new Error(`operation ${operation.type} cannot be undone`);
          }
          ret = (await operation.undo()) as T;
        }
        onResponse(operation, ret);
        this.completed.push({ ...operation });
        // trim completed stack
        if (this.completed.length > COMPLETED_STACK_SIZE) {
          this.completed = this.completed.slice(this.completed.length - COMPLETED_STACK_SIZE);
        }
        return ret;
      } catch (e) {
        onError(operation, e);
        return Promise.resolve(null);
      } finally {
        this.inflight = this.inflight.filter((op) => op.id !== operation.id);
      }
    },

    async perform<T>(operation: Operation<T>): Promise<T | null> {
      operation = { ...operation, id: operation.id ?? Math.random().toString(16).substring(2, 8) };
      console.log(`perform ${operation.type} (id=${operation.id})`);

      // enable undo even before the operation is performed (for responsiveness)
      if (operation.undo != null) {
        this.undoStack.push(operation);
      }
      const ret = await this._do(operation);
      this.redoStack = []; // reset redo stack, maybe store a redo branch backup?
      return ret as T | null; // cannot be void because it's not undo
    },

    async undo(): Promise<void> {
      const operation = this.undoStack.pop();
      if (operation == null || operation.undo == null) {
        return;
      }
      console.log(`undo ${operation.type} (id=${operation.id})`);
      await this._do(operation, true);
      this.redoStack.push(operation);
    },

    async redo(): Promise<void> {
      const operation = this.redoStack.pop();
      if (operation == null) {
        return;
      }
      console.log(`redo ${operation.type} (id=${operation.id})`);
      await this._do(operation);
      this.undoStack.push(operation);
    },
  },
});

export function _useOperations() {
  const state = useOperationsStore();
  return {
    user: useUserOps(),
    file: useFileOps(),
    content: useSymbolContentOps(),
    statement: useStatementOps(),
    symbol: useSymbolContentOps(),
    runtime: useRuntimeOps(),
    version: useProjectVersionOps(),
    state,
  };
}

export const useOperations = createSharedComposable(_useOperations);

function onResponse(operation: Operation<unknown>, ret: unknown) {
  // check for error response
  if ((ret as any)?.data != null) {
    // get only field of data (which is the mutation response)
    ret = Object.values((ret as any).data)[0];
    if ((ret as any).__typename == "OperationInfo") {
      onError(operation, ret);
    }
  }
}

// rewrite so that onError has an array of errorListeners
function onError(operation: Operation<unknown>, error: unknown) {
  console.error(`operation ${operation.type} ${operation.id} failed`, error);
  errorListeners.forEach((listener) => listener(operation, error));
}
export const errorListeners: ((operation: Operation<unknown>, error: unknown) => void)[] = [];
