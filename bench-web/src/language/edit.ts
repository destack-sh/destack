import { toCamelName } from "@/language/const";
import type { ReadNodeGraph } from "@/language/graph";
import {
  EDIT_TYPES,
  UNDO_EDIT_BY_TYPE,
  getTransactionBuffer,
  newChangeId,
  newEditId,
  txBuffers,
  type ChangeIn,
  type TransactionBuffer,
} from "@/language/transaction";
import { unpackBuiltinObject } from "@/language/value";
import {
  ChangeCategory,
  EditData,
  EditType,
  LogData,
  NodeReferenceData,
  ObjectType,
  Timestamp,
  ViewData,
  type AnyNodeData,
} from "@/proto/wire";
import {
  EMPTY_SCOPE,
  describeEdit,
  describeNode,
  makeDefaultObject,
  makeScope,
  toPlainNodeRef,
  unpackProtoJson,
  wrapSomeNode,
} from "@/proto/wiring";
import { origin, userPtr } from "@/system/client";
import { assertNever } from "@/utils/functools";
import { log } from "@/utils/log";
import { watch } from "vue";

/**
 * A stack of edits for undo/redo.
 * NOTE :UX: use the edit logs for cross-device undo/redo?
 * NOTE :UX: don't 'freeze' original debounced edits so that quick undo/redo is merged into one edit?
 *  (not sure how that would work.. maybe keep a separate stack for debounced not-yet-committed edits?)
 * */
class EditStack {
  private _editStack: EditData[] = [];
  private _editsById: Record<string, EditData> = {};
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

  /**
   * Apply the inverse of the last edit to the stack. Noop if impossible.
   * If there are multiple successive edits belonging to the same change, all of them are undone.
   */
  undo() {
    if (this._undoIndex <= 0) return;
    // accumulate edits from same change (edits are in reverse order)
    const edits: EditData[] = [this._editStack[this._undoIndex - 1]];
    if (edits[0].changeKey != null) {
      for (let i = this._undoIndex - 2; i >= 0; i--) {
        const edit = this._editStack[i];
        if (edit.changeKey != edits[0].changeKey) break;
        edits.push(edit);
      }
    }

    // make inverse edits to undo change
    const change: ChangeIn = { key: newChangeId(), title: `Undo` };
    const undoEdits: EditData[] = [];
    for (const edit of edits) {
      const undoEdit: EditData = { ...edit, id: newEditId(), editedAt: Timestamp.now(), changeKey: change.key };
      invertEdit(edit, undoEdit, "undo");
      undoEdits.push(undoEdit);
      this._undoIndex--;
      this._derivedEditsById[undoEdit.id] = undoEdit;

      // apply in same connection as original edits
      const buffer = getTransactionBuffer(edit.scope ?? EMPTY_SCOPE);
      const connectionId = this._connectionIdByEdit[edit.id!];
      if (connectionId == null) throw new Error(`missing connection for ${describeEdit(edit)}`);
      buffer.tx.with({ connectionId, change }).addEdit(undoEdit);
      buffer.tx.clearDebounce(edit.id!); // 'freeze' the original edit
    }
    log.info("edit.undo", { edits, undoEdits, undoIndex: this._undoIndex });
  }

  /**
   * Reapply the next undone edit (technically, the inverse of the inverse). Noop if impossible.
   * If there are multiple successive edits belonging to the same change, all of them are redone.
   */
  redo() {
    if (this._undoIndex >= this._editStack.length) return;
    // accumulate edits from same change (edits are in right order)
    const edits: EditData[] = [this._editStack[this._undoIndex]];
    if (edits[0].changeKey != null) {
      for (let i = this._undoIndex + 1; i < this._editStack.length; i++) {
        const edit = this._editStack[i];
        if (edit.changeKey != edits[0].changeKey) break;
        edits.push(edit);
      }
    }

    // make inverse edits to redo change
    const change: ChangeIn = { key: newChangeId(), title: `Redo` };
    const redoEdits: EditData[] = [];
    for (const edit of edits) {
      const redoEdit: EditData = { ...edit, id: newEditId(), editedAt: Timestamp.now(), changeKey: change.key };
      invertEdit(edit, redoEdit, "redo");
      redoEdits.push(redoEdit);
      this._undoIndex++;
      this._derivedEditsById[redoEdit.id] = redoEdit;

      // apply in same connection as original edits
      const buffer = getTransactionBuffer(edit.scope ?? EMPTY_SCOPE);
      const connectionId = this._connectionIdByEdit[edit.id!];
      if (connectionId == null) throw new Error(`missing connection for ${describeEdit(edit)}`);
      buffer.tx.with({ connectionId, change }).addEdit(redoEdit);
      buffer.tx.clearDebounce(edit.id!); // 'freeze' the original edit
    }
    log.info("edit.undo", { edits, redoEdits, undoIndex: this._undoIndex });
  }

  subscribeToBuffer(buffer: TransactionBuffer): () => void {
    const bufferedSub = buffer.subscribeBuffered((event) => {
      let hasNewEdits = false;
      for (const edit of event.newEdits) {
        if (this._filter(edit) && !this._editsById[edit.id] && !this._derivedEditsById[edit.id]) {
          this._editStack.push(edit);
          this._editsById[edit.id] = edit;
          if (event.connectionIdByEditId[edit.id] == null)
            throw new Error(`missing connection id for ${describeEdit(edit)}`);
          this._connectionIdByEdit[edit.id] = event.connectionIdByEditId[edit.id];
          hasNewEdits = true;
        }
      }
      if (hasNewEdits) {
        // reset undo/redo stack
        this._undoIndex = this._editStack.length;
      }
    });
    return bufferedSub;
  }
}

// one edit stack for all bench tx buffers
// NOTE :UX: we currently only have one shared edit stack, should probably be per view root?
const editStack = new EditStack((e) => e.category != ChangeCategory.SPACE && e.category != ChangeCategory.SESSION);
const editStackSubs: Array<() => void> = [];
watch(
  txBuffers,
  () => {
    editStackSubs.forEach((sub) => sub());
    Object.values(txBuffers.value).forEach((buffer) => {
      const sub = editStack.subscribeToBuffer(buffer);
      editStackSubs.push(sub);
    });
  },
  { immediate: true },
);

/** Gets the relevant edit stack in the given view. */
export function getEditStack(graph: ReadNodeGraph, focusedView: ViewData | null): EditStack {
  // only one for now (see above)
  return editStack;
}

/** Inverts an edit as an undo/redo of the given edit (in place). */
function invertEdit(originalEdit: EditData | LogData, newEdit: EditData, mode: "undo" | "redo"): void {
  const undoType = UNDO_EDIT_BY_TYPE[newEdit.type!];
  if (undoType == null) throw new Error(`cannot undo edit ${toCamelName(EditType, newEdit.type!)}}`);
  if (mode == "undo") {
    newEdit.type = undoType;
    [newEdit.oldNode, newEdit.newNode] = [newEdit.newNode, newEdit.oldNode];
  } else if (mode == "redo") {
    const redoType = UNDO_EDIT_BY_TYPE[undoType!];
    if (redoType == null) throw new Error(`cannot redo edit ${toCamelName(EditType, undoType!)}}`);
    newEdit.type = redoType;
  } else {
    assertNever(mode);
  }
  if (newEdit.type == EditType.RESTORE) {
    newEdit.oldEditedAt = (originalEdit as EditData).editedAt ?? (originalEdit as LogData).createdAt;
  } else {
    newEdit.oldEditedAt = undefined;
  }
}

/** Turns a logged edit back into an edit (to redo/undo) */
export function makeEditFromLog(
  log: LogData,
  mode: "redo" | "undo",
  options?: {
    category?: ChangeCategory;
    subjectPtr?: NodeReferenceData;
  },
): EditData {
  // unpack
  if (log.nodePtr == null) throw new Error(`missing node for ${log.type}: ${describeNode(log)}`);
  const nodeType = log.nodePtr.type;
  const editType = log.type as unknown as EditType | undefined;
  if (!EDIT_TYPES.includes(editType!)) throw new Error(`unexpected edit ${editType}: ${describeNode(log)}`);
  const oldNode =
    log.oldNodePacked != null
      ? (makeDefaultObject(
          unpackBuiltinObject(unpackProtoJson(log.oldNodePacked), nodeType as unknown as ObjectType),
        ) as AnyNodeData)
      : null;
  const newNode =
    log.newNodePacked != null
      ? (makeDefaultObject(
          unpackBuiltinObject(unpackProtoJson(log.newNodePacked), nodeType as unknown as ObjectType),
        ) as AnyNodeData)
      : null;

  // make edit
  const subjectPtr = options?.subjectPtr ?? userPtr.value;
  if (subjectPtr == null) throw new Error(`missing subject for ${log.type}: ${describeNode(log)}`);
  const scope = makeScope({ benchId: log.benchPtr?.id, packageId: log.packagePtr?.id });
  const edit: EditData = {
    metatype: ObjectType.EDIT,
    id: newEditId(),
    type: editType!,
    nodePtr: log.nodePtr,
    scope: scope,
    properties: log.properties,
    oldNode: oldNode != null ? wrapSomeNode(oldNode) : undefined,
    newNode: newNode != null ? wrapSomeNode(newNode) : undefined,
    origin: origin.value,
    category: options?.category ?? log.category,
    subjectPtr: subjectPtr,
    editedAt: Timestamp.now(),
    undoOfPtr: mode == "undo" ? toPlainNodeRef(log) : undefined,
  };

  // invert edit
  invertEdit(log, edit, mode);

  return edit;
}
