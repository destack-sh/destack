import { useClientOps } from "@/state/operations/client";
import { useFileOps } from "@/state/operations/file";
import { useObjectOps } from "@/state/operations/object";
import { useOrganizationOps } from "@/state/operations/organization";
import { useProjectOps } from "@/state/operations/project";
import { useSessionOps } from "@/state/operations/session";
import { useSecretOps } from "@/state/operations/secret";
import { useStatementOps } from "@/state/operations/statement";
import { useSymbolContentOps } from "@/state/operations/symbol";
import { useUserOps } from "@/state/operations/user";
import { useProjectVersionOps } from "@/state/operations/version";
import { captureException } from "@sentry/vue";
import { createSharedComposable } from "@vueuse/shared";
import { DateTime } from "luxon";
import { defineStore } from "pinia";
import { ref, type Ref } from "vue";

/** A single atomic(ish) operation (usually against the DB) */
export type Operation<T> = {
  tx?: Transaction | null;
  id?: string;
  type: string;
  startedAt?: DateTime;
  key?: string | Record<string, string>;
  stateless?: boolean; // whether the operation mutates synced state (the default)
  suppressErrors?: boolean; // whether to suppress errors (false by default)
  do(): Promise<T>;
  redo?(): Promise<T | unknown>;
  undo?(): Promise<unknown>;
};

/** A bundle of related operations (does not correspond to DB transactions (yet)) */
export type Transaction = {
  id: string;
  name?: string;
  startedAt: DateTime;
  closedAt?: DateTime;
  operations: Operation<unknown>[];
  blockPartialUndo?: boolean; // whether to block partial undo
  collapseUndoToFirst?: boolean; // whether undo/redo only apply to the first executed operation
  undo?(): Promise<unknown>; // tx-level undo of all operations
  redo?(): Promise<unknown>; // tx-level redo of all operations (both must be set if any)
};

export function openTransaction(
  options?: Pick<Transaction, "name" | "blockPartialUndo" | "collapseUndoToFirst" | "undo" | "redo">
): Transaction {
  return {
    ...options,
    id: Math.random().toString(16).substring(2, 8),
    startedAt: DateTime.now(),
    operations: [],
  };
}

export function closeTransaction(tx: Transaction) {
  if (tx.closedAt) {
    throw new Error(`transaction ${tx.id} already closed at ${tx.closedAt}`);
  }
  tx.closedAt = DateTime.now();
}

const COMPLETED_STACK_SIZE = 500;
const STALE_TIME_SECONDS = 2;

// keep reactive now (can't use useNow because it attaches to component)
const now: Ref<DateTime> = ref(DateTime.now());
setInterval(() => (now.value = DateTime.now()), 100);

type StackState = {
  id: string;
  undoStack: Operation<unknown>[];
  redoStack: Operation<unknown>[];
};

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
        const stalenessCutoff = filters.stale ? now.value.minus({ seconds: STALE_TIME_SECONDS }) : null;
        return state.inflight.filter(
          (op) =>
            (!filters.types || filters.types.includes(op.type)) &&
            (!filters.typesNot || !filters.typesNot.includes(op.type)) &&
            (!filters.keys || (op.key != null && JSON.stringify(op.key).match(new RegExp(filters.keys.join("|"))))) &&
            (!filters.stale || (op.startedAt != null && stalenessCutoff != null && op.startedAt < stalenessCutoff)) &&
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
    stackState(state): StackState {
      return {
        id: Math.random().toString(16).substring(2, 8),
        undoStack: state.undoStack.slice(),
        redoStack: state.redoStack.slice(),
      };
    },
  },
  actions: {
    reset(): void {
      this.$reset();
    },

    restoreStackState(state: StackState): void {
      console.log(`restore stack state ${state.id}`);
      this.undoStack = state.undoStack;
      this.redoStack = state.redoStack;
    },

    async _do<T>(operation: Operation<T>, mode: "do" | "redo" | "undo"): Promise<T | void | null> {
      operation = { ...operation, startedAt: DateTime.now() };
      this.inflight.push(operation);
      try {
        let ret;
        if (mode == "do") {
          ret = await operation.do();
        } else if (mode == "redo") {
          ret = await (operation.redo ?? operation.do)();
        } else if (mode == "undo") {
          if (operation.undo == null) {
            throw new Error(`operation ${operation.type} cannot be undone`);
          }
          ret = (await operation.undo()) as T;
        } else {
          throw new Error(`unknown operation mode ${mode}`);
        }
        onResponse(operation, ret);
        this.completed.push({ ...operation });
        // trim completed stack
        if (this.completed.length > COMPLETED_STACK_SIZE) {
          this.completed = this.completed.slice(this.completed.length - COMPLETED_STACK_SIZE);
        }
        return ret as T | void | null;
      } catch (e) {
        if (!operation.suppressErrors) {
          onError(operation, e);
        }
        return Promise.resolve(null);
      } finally {
        this.inflight = this.inflight.filter((op) => op.id !== operation.id);
      }
    },

    async perform<T>(operation: Operation<T>): Promise<T | null> {
      operation = { ...operation, id: operation.id ?? Math.random().toString(16).substring(2, 8) };
      console.debug(`perform ${operation.type} (id=${operation.id}, tx=${operation.tx?.id ?? "<none>"})`); // nocheckin
      // add operation to transaction if any
      if (operation.tx != null) {
        if (typeof operation.tx != "object") {
          throw new Error(`invalid transaction ${operation.tx}`); // (probably forgot first argument)
        }
        if (operation.tx.closedAt != null) {
          throw new Error(`transaction ${operation.tx.id} is already closed`);
        }
        if (operation.tx.operations.find((op) => op.id === operation.id)) {
          throw new Error(`operation ${operation.id} is already in transaction ${operation.tx.id}`);
        }
        operation.tx.operations.push(operation);
        // collapse undo/redo to first performed operation if requested
        if (operation.tx.collapseUndoToFirst && operation.undo == null) {
          operation.tx.undo = operation.undo;
          operation.tx.redo = operation.redo;
        }
      }
      // enable undo even before the operation is performed (for responsiveness)
      if (operation.undo != null) {
        this.undoStack.push(operation);
      }
      const ret = await this._do(operation, "do");
      if (!operation.stateless) {
        // console.debug(`reset redo stack for ${operation.type} (id=${operation.id})`);
        this.redoStack = []; // reset redo stack, maybe store a redo branch backup?
      }
      return ret as T | null; // cannot be void because it's not undo
    },

    async undo(): Promise<void> {
      const operation = this.undoStack.pop();
      if (operation == null) {
        throw new Error("nothing to undo");
      }
      if (operation.tx?.blockPartialUndo && operation.tx.closedAt == null) {
        this.undoStack.push(operation); // re-add operation to undo stack
        return; // ignore
      }
      // TODO @Performance: collapse transaction operations that do the same thing (e.g. update record on same id)
      // TODO @Performance: batch transaction operations if length > 1
      // TODO @Performance: do we really need to apply operations one be one in redo/undo?
      // Probably, since we can't guarantee that the same server is used in sequence (or can we?).
      if (operation.tx?.undo != null) {
        // tx atomic undo
        console.log(`undo tx ${operation.tx.id} (id=${operation.id})`);
        const stacks = this.stackState;
        await operation.tx.undo();
        this.restoreStackState(stacks);
        this.undoStack = this.undoStack.filter((op) => op.tx?.id !== operation.tx?.id);
        this.redoStack.push(...operation.tx.operations.reverse());
      } else if (operation.tx != null) {
        // tx multi undo
        const operationsToUndo = [...this.undoStack.filter((op) => op.tx?.id === operation.tx?.id), operation];
        for (const op of operationsToUndo.reverse()) {
          console.log(`undo ${op.type} (id=${op.id}, tx=${op.tx?.id ?? "<none>"})`);
          await this._do(op, "undo");
        }
        this.undoStack = this.undoStack.filter((op) => op.tx?.id !== operation.tx?.id);
        this.redoStack.push(...operationsToUndo.reverse());
      } else {
        // single operation undo
        console.log(`undo ${operation.type} (id=${operation.id}, tx=<none>)`);
        await this._do(operation, "undo");
        this.redoStack.push(operation);
      }
    },

    async redo(): Promise<void> {
      const operation = this.redoStack.pop();
      if (operation == null) {
        throw new Error("nothing to redo");
      }
      if (operation.tx?.undo != null) {
        // tx atomic redo
        if (operation.tx.redo == null) {
          throw new Error(`transaction ${operation.tx.id} has undo but no redo`);
        }
        console.log(`redo tx ${operation.tx.id} (id=${operation.id})`);
        const stacks = this.stackState;
        await operation.tx.redo();
        this.restoreStackState(stacks);
        this.undoStack.push(...operation.tx.operations);
        this.redoStack = this.redoStack.filter((op) => op.tx?.id !== operation.tx?.id);
      } else if (operation.tx != null) {
        // tx multi redo
        const operationsToRedo = [...this.redoStack.filter((op) => op.tx?.id === operation.tx?.id), operation];
        for (const op of operationsToRedo) {
          console.log(`redo ${op.type} (id=${op.id}, tx=${op.tx?.id ?? "<none>"})`);
          await this._do(op, "redo");
        }
        this.undoStack.push(...operationsToRedo);
        this.redoStack = this.redoStack.filter((op) => op.tx?.id !== operation.tx?.id);
      } else {
        // single operation redo
        console.log(`redo ${operation.type} (id=${operation.id}, tx=<none>)`);
        await this._do(operation, "redo");
        this.undoStack.push(operation);
      }
    },
  },
});

export function _useOperations() {
  const state = useOperationsStore();
  return {
    user: useUserOps(),
    client: useClientOps(),
    organization: useOrganizationOps(),
    project: useProjectOps(),
    file: useFileOps(),
    content: useSymbolContentOps(),
    statement: useStatementOps(),
    symbol: useSymbolContentOps(),
    session: useSessionOps(),
    version: useProjectVersionOps(),
    object: useObjectOps(),
    secret: useSecretOps(),
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

function onError(operation: Operation<unknown>, error: unknown) {
  console.trace(`operation ${operation.type} ${operation.id} failed`, error);
  errorListeners.forEach((listener) => listener(operation, error));
  // capture with sentry
  captureException(error);
}
export const errorListeners: ((operation: Operation<unknown>, error: unknown) => void)[] = [];
