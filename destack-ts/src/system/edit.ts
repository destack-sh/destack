import { canvas } from "@/globals";
import { HELPER_VIEW_TYPES, toCamelName } from "@/language/core/const";
import type { ReadNodeGraph } from "@/language/core/graph";
import {
  getTransactionBuffer,
  newChangeId,
  newEditId,
  txBuffers,
  type ChangeIn,
  type TransactionBuffer,
} from "@/language/core/transaction";
import {
  ChangeCategory,
  EditData,
  EditOperationData,
  EditOperationType,
  EditType,
  ObjectType,
  Timestamp,
  ViewData,
} from "@/proto/wire";
import { EMPTY_SCOPE, describeEdit } from "@/proto/wiring";
import { provideCommands } from "@/ui/command";
import { assertNever } from "@/utils/functools";
import { log } from "@/utils/log";
import { watch } from "vue";

/**
 * A stack of edits for undo/redo.
 * NOTE :UX: use the edit logs for cross-device undo/redo?
 * NOTE :UX: don't 'freeze' original debounced edits so that quick undo/redo is merged into one edit?
 * TODO :UX: keep more information per change/edit so we can undo/redo more elegantly (inspection, current focus, ...)
 * TODO :UX: isolate edit stacks per container/page/thread/..?
 *  (not sure how that would work.. maybe keep a separate stack for debounced not-yet-committed edits?)
 * */
class EditStack {
  private _editStack: EditData[] = [];
  private _editsById: Record<string, EditData> = {};
  private _editedAtByEditId: Record<string, Timestamp> = {};
  private _connectionIdByEdit: Record<string, number> = {};
  private _derivedEditsById: Record<string, EditData> = {};
  private _undoIndex: number = 0;
  private _filter: (edit: EditData) => boolean;

  constructor(filter: (edit: EditData) => boolean) {
    this._filter = filter;
  }

  get canUndo() {
    return this._undoIndex > 0;
  }

  get canRedo() {
    return this._undoIndex < this._editStack.length;
  }

  _onDid(direction: "undo" | "redo", edits: EditData[]) {
    // remove container views with removed nodes
    if (edits.some((e) => e.type === EditType.ARCHIVE || e.type === EditType.DELETE || e.type === EditType.ERASE)) {
      const views = canvas.views;
      const tx = canvas.tx();
      for (const e of edits) {
        if (e.type === EditType.ARCHIVE || e.type === EditType.DELETE || e.type === EditType.ERASE) {
          const viewsOfNode = views.filter((v) => v.nodePtr?.id == e.nodePtr?.id && !HELPER_VIEW_TYPES.has(v.type));
          for (const view of viewsOfNode) {
            tx.archive(view);
          }
          if (canvas.inspection?.id == e.nodePtr?.id) {
            tx.update(canvas.space.value!, { inspectionPtr: undefined });
          }
        }
      }
    }
  }

  /**
   * Apply the inverse of the last edit to the stack. Noop if impossible.
   * If there are multiple successive edits belonging to the same change, all of them are un-done.
   */
  undo() {
    if (this._undoIndex <= 0) {
      return;
    }
    // accumulate edits from same change (edits are in reverse order)
    const originalEdits: EditData[] = [this._editStack[this._undoIndex - 1]];
    if (originalEdits[0].changeKey != null) {
      for (let i = this._undoIndex - 2; i >= 0; i--) {
        const edit = this._editStack[i];
        if (edit.changeKey != originalEdits[0].changeKey) {
          break;
        }
        originalEdits.push(edit);
      }
    }

    // make inverse edits to undo change
    const change: ChangeIn = { key: newChangeId(), title: `Undo` };
    const undoEdits: EditData[] = [];
    const editedAt = Timestamp.now();
    for (const originalEdit of originalEdits) {
      // invert edit
      const undoEdit: EditData = { ...originalEdit, id: newEditId(), editedAt, changeKey: change.key };
      invertEdit(originalEdit, this._editedAtByEditId[originalEdit.id]!, undoEdit, "undo");
      this._undoIndex--;
      this._editedAtByEditId[originalEdit.id] = editedAt;
      undoEdits.push(undoEdit);
      this._derivedEditsById[undoEdit.id] = undoEdit;

      // apply in same connection as original edits
      const buffer = getTransactionBuffer(originalEdit.scope ?? EMPTY_SCOPE);
      const connectionId = this._connectionIdByEdit[originalEdit.id!];
      if (connectionId == null) {
        throw new Error(`missing connection for ${describeEdit(originalEdit)}`);
      }
      buffer.tx.with({ connectionId, change }).addEdit(undoEdit);
      buffer.tx.stopDebounce(originalEdit.nodePtr?.id!); // 'freeze' any pending edits
    }
    this._onDid("undo", undoEdits);
    log.trace("edit.undo", { originalEdits, undoEdits, undoIndex: this._undoIndex });
  }

  /**
   * Reapply the next undone edit (technically, the inverse of the inverse). Noop if impossible.
   * If there are multiple successive edits belonging to the same change, all of them are re-done.
   */
  redo() {
    if (this._undoIndex >= this._editStack.length) {
      return;
    }
    // accumulate edits from same change
    // ('edits' are the *original* edits in original order)
    const edits: EditData[] = [this._editStack[this._undoIndex]];
    if (edits[0].changeKey != null) {
      for (let i = this._undoIndex + 1; i < this._editStack.length; i++) {
        const edit = this._editStack[i];
        if (edit.changeKey != edits[0].changeKey) {
          break;
        }
        edits.push(edit);
      }
    }

    // make inverse edits to redo change
    const change: ChangeIn = { key: newChangeId(), title: `Redo` };
    const redoEdits: EditData[] = [];
    const editedAt = Timestamp.now();
    for (const edit of edits) {
      const redoEdit: EditData = { ...edit, id: newEditId(), editedAt, changeKey: change.key };
      invertEdit(edit, this._editedAtByEditId[edit.id]!, redoEdit, "redo");
      redoEdits.push(redoEdit);
      this._undoIndex++;
      this._editedAtByEditId[edit.id] = editedAt;
      this._derivedEditsById[redoEdit.id] = redoEdit;

      // apply in same connection as original edits
      const buffer = getTransactionBuffer(edit.scope ?? EMPTY_SCOPE);
      const connectionId = this._connectionIdByEdit[edit.id!];
      if (connectionId == null) {
        throw new Error(`missing connection for ${describeEdit(edit)}`);
      }
      buffer.tx.with({ connectionId, change }).addEdit(redoEdit);
      buffer.tx.stopDebounce(edit.nodePtr?.id!); // 'freeze' any pending edits
    }
    this._onDid("redo", redoEdits);
    log.trace("edit.redo", { edits, redoEdits, undoIndex: this._undoIndex });
  }

  subscribeToBuffer(buffer: TransactionBuffer): () => void {
    const bufferedSub = buffer.subscribeBuffer((event) => {
      let hasNewEdits = false;
      for (const edit of event.bufferedEdits) {
        // skip if already processed
        if (this._editsById[edit.id] || this._derivedEditsById[edit.id]) {
          continue;
        }

        // skip if irrelevant
        if (!this._filter(edit)) {
          continue;
        }

        // add to stack
        this._editStack.push(edit);
        this._editsById[edit.id] = edit;
        this._editedAtByEditId[edit.id] = edit.editedAt!;
        if (event.connectionIdByEditId[edit.id] == null) {
          throw new Error(`missing connection id for ${describeEdit(edit)}`);
        }
        this._connectionIdByEdit[edit.id] = event.connectionIdByEditId[edit.id];
        hasNewEdits = true;
      }

      if (hasNewEdits) {
        // reset undo/redo stack
        this._undoIndex = this._editStack.length;
        // NOTE :Performance: prune edit stack eventually (after do)?
      }
    });
    return bufferedSub;
  }
}

// one edit stack for all destack tx buffers
// NOTE :UX: we currently only have one shared edit stack, should probably be per container/page/thread/...?
const editStack = new EditStack((e) => e.category != ChangeCategory.SPACE && e.category != ChangeCategory.RUNTIME);
const _editStackSubs: Array<() => void> = [];
watch(
  txBuffers,
  () => {
    _editStackSubs.forEach((sub) => sub());
    Object.values(txBuffers.value).forEach((buffer) => {
      const sub = editStack.subscribeToBuffer(buffer);
      _editStackSubs.push(sub);
    });
  },
  { immediate: true },
);

/** Gets the relevant edit stack in the given view. */
export function getEditStack(graph: ReadNodeGraph, focusedView: ViewData | null): EditStack {
  // only one for now (see above)
  return editStack;
}

export function invertEditOperation(op: EditOperationData): EditOperationData {
  if (op.type == EditOperationType.SET || op.type == EditOperationType.CLEAR) {
    return {
      metatype: ObjectType.EDIT_OPERATION,
      type: op.oldValuePacked == null ? EditOperationType.CLEAR : EditOperationType.SET,
      path: op.path,
      newValuePacked: op.oldValuePacked,
      oldValuePacked: op.newValuePacked,
    };
  } else {
    throw new Error(`cannot invert operation ${op.type}`);
  }
}

/** Map edit type to inverted edit type */
export const UNDO_EDIT_BY_TYPE: Partial<Record<EditType, EditType>> = {
  [EditType.CREATE]: EditType.DELETE,
  [EditType.UPSERT]: EditType.DELETE,
  [EditType.UPDATE]: EditType.UPDATE,
  [EditType.MOVE]: EditType.MOVE,
  [EditType.ARCHIVE]: EditType.UNARCHIVE,
  [EditType.UNARCHIVE]: EditType.ARCHIVE,
  [EditType.DELETE]: EditType.RESTORE,
  [EditType.RESTORE]: EditType.DELETE,
};

/** Inverts an edit as an undo/redo of the given edit (in place). */
function invertEdit(originalEdit: EditData, editedAt: Timestamp, invertedEdit: EditData, mode: "undo" | "redo"): void {
  const undoType = UNDO_EDIT_BY_TYPE[invertedEdit.type!];
  if (undoType == null) throw new Error(`cannot undo edit ${toCamelName(EditType, invertedEdit.type!)}}`);

  // edit type
  if (mode == "undo") {
    invertedEdit.type = undoType;
  } else if (mode == "redo") {
    const redoType = UNDO_EDIT_BY_TYPE[undoType!];
    if (redoType == null) throw new Error(`cannot redo edit ${toCamelName(EditType, undoType!)}}`);
    invertedEdit.type = redoType;
  } else {
    assertNever(mode);
  }

  // edit content
  invertedEdit.nodeData = originalEdit.nodeData;
  if (invertedEdit.type == EditType.UNARCHIVE || invertedEdit.type == EditType.RESTORE) {
    invertedEdit.oldEditedAt = editedAt;
  }

  // edit operations
  if (mode == "undo") {
    invertedEdit.operations = originalEdit.operations.map((op) => invertEditOperation(op)).reverse();
  }
}

// history
export const HISTORY_COMMANDS = provideCommands<"space.history">({
  "space.history.undo": {
    icon: "fas fa-arrow-turn-left",
    title: "Undo",
    text: "Undo the last command or edit",
    shortcuts: ["mod+z"],
    isEnabled: () => getEditStack(canvas.graph, canvas.focusedView).canUndo,
    command: (command, ctx) => {
      const stack = getEditStack(canvas.graph, canvas.focusedView);
      stack.undo();
    },
  },
  "space.history.redo": {
    icon: "fas fa-arrow-turn-right",
    title: "Redo",
    text: "Redo the last undone command or edit",
    shortcuts: ["mod+shift+z", "mod+y"],
    isEnabled: () => getEditStack(canvas.graph, canvas.focusedView).canRedo,
    command: (command, ctx) => {
      const stack = getEditStack(canvas.graph, canvas.focusedView);
      stack.redo();
    },
  },
});
