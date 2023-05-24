import { getRandomAdjective } from "@/composables/useRandomName";
import { StatementType, SymbolType, type StatementContentFragment, TypeTag } from "@/gql/graphql";
import { FileEditor, useBenchState, type FileHeader, type StatementHeader } from "@/state/editor";
import { useObjects } from "@/state/object";
import { closeTransaction, openTransaction, useOperations, type Transaction } from "@/state/operations";
import { newDatasetRecordId, newStatementId, newTypeNodeId, newTypeNodeKey } from "@/state/operations/statement";
import { TypeFlag } from "@/state/runtime";
import { INTEGER_ZERO, generateKeyBetween, generateNKeysBetween } from "@/utils/fractional";
import { onBeforeUnmount, watchEffect, type Ref, ref, computed, inject, provide } from "vue";
import type StatementInterface from "@/components/StatementInterface.vue";

export const FILE_CONTEXT = "__fileContext__" as const;

export type FileState = {
  editor: FileEditor;
  focused: boolean;
  file: FileHeader;
  statementsUnordered: StatementHeader[]; // unordered
  statementsComponents: Record<string, InstanceType<typeof StatementInterface>>;
  navigateUp: () => void;
  navigateDown: () => void;
};

export type FileContext = FileState & {
  statements: StatementHeader[]; // ordered
  statementsById: Record<string, StatementHeader>;
  statementsComponents: Record<string, InstanceType<typeof StatementInterface>>;
  statementsByParentId: Record<string, StatementHeader[]>;
  statementPositions: Record<string, number>;
  positionedStatements: PositionedStatement[];
  depths: number[];
};

export type PositionedStatement = {
  depth: number;
  lineNumberBase: number;
  isFirstInGroup: boolean;
  isLastInGroup: boolean;
  statement: StatementContentFragment;
  ancestors: string[];
};

// There can only be one active file to provide file shortcuts,
// so we have a global reference here that is automatically set to the focused file.
// We can't just use singleton actions here because multiple files may have
// 'focused' set during moves or transition.
export const activeFileState: Ref<FileState> = ref<FileState | null>(null);
export const fileContexts = ref<Record<string, FileContext>>({});
export const navigationContexts = ref<Record<string, NavigationContext>>({});

export function provideFileState(file: Ref<FileState | null>) {
  // set active file if focused
  watchEffect(() => {
    if (file.value?.focused) {
      activeFileState.value = file.value;
    }
  });
  onBeforeUnmount(() => {
    if (activeFileState.value?.editor.id === file.value?.editor.id) {
      activeFileState.value = null;
    }
  });

  // file context
  const statementsUnordered = computed(() => file.value?.statementsUnordered ?? []);
  const rootStatements = computed(() => statementsUnordered.value.filter((statement) => statement.parent == null));

  const statementsById = computed(() => {
    const statementsById = {};
    statementsUnordered.value.forEach((statement) => {
      statementsById[statement.id] = statement;
    });
    return statementsById;
  });

  // statementsByParentId must be ordered like orderedStatements
  const statementsByParentId: Ref<Record<string, StatementHeader[]>> = computed(() => {
    const result: Record<string, StatementHeader[]> = {};
    for (const statement of statements.value) {
      const parentId = statement.parent?.id ?? "";
      if (!result[parentId]) result[parentId] = [];
      result[parentId].push(statement as StatementHeader);
    }
    return result;
  });

  const statementPositions = computed(() => {
    const result: Record<string, number> = {};
    for (let i = 0; i < statements.value.length; i++) {
      result[statements.value[i].id] = i;
    }
    return result;
  });

  /* Statements are hierarchical but laid out linearly (in one column) */

  const positionedStatements = computed(() => {
    const positionedStatements: PositionedStatement[] = [];
    let lineNumberBase = 0;

    // depth first traversal
    function walkDfs(statement: StatementContentFragment, ancestors: string[], isLast: boolean) {
      if (statement?.id == null) {
        // bail in case a bad statement ends in here due to some other bug to prevent recursion death
        console.warn("got bad statement with null id", statement, ancestors, isLast);
        return;
      }

      const children = statementsUnordered.value.filter((child) => child.parent?.id == statement.id);
      const isFirstInGroup = ancestors.length == 0;
      const isLastInRoot = isLast && children.length == 0;

      positionedStatements.push({
        depth: ancestors.length,
        lineNumberBase,
        statement,
        ancestors,
        isFirstInGroup,
        isLastInGroup: isLastInRoot,
      });
      lineNumberBase += 1;

      // walk children, sorted by order key
      ancestors = [...ancestors, statement.id];
      children.sort((a, b) => ((a.orderKey ?? INTEGER_ZERO) < (b.orderKey ?? INTEGER_ZERO) ? -1 : 1));
      children.forEach((child, i) => walkDfs(child, ancestors, isLast && i == children.length - 1));
    }

    // start with roots sorted by order key
    const roots = rootStatements.value;
    roots.sort((a, b) => ((a.orderKey ?? INTEGER_ZERO) < (b.orderKey ?? INTEGER_ZERO) ? -1 : 1));
    roots.forEach((root) => walkDfs(root, [], true));

    // group groupable sibling statements at root
    for (const [i, positioned] of positionedStatements.entries()) {
      if (
        positioned.statement.type == StatementType.Import ||
        positioned.statement.type == StatementType.Comment ||
        positioned.statement.type == StatementType.Blank ||
        positioned.statement.symbolType == SymbolType.Requirement
      ) {
        const next = positionedStatements[i + 1];
        if (next && next.depth == 0 && next?.statement.type == positioned.statement.type) {
          positioned.isLastInGroup = false;
          next.isFirstInGroup = false;
        }
      }
    }

    return positionedStatements;
  });
  const statements = computed(() => positionedStatements.value.map((positioned) => positioned.statement));
  const depths = computed(() => positionedStatements.value.map((positioned) => positioned.depth));

  // provide context
  const context = computed(() => {
    if (file.value == null) {
      return null;
    }
    return {
      ...file.value,
      statements: statements.value,
      statementsComponents: file.value.statementsComponents ?? {},
      statementsById: statementsById.value,
      statementsByParentId: statementsByParentId.value,
      statementPositions: statementPositions.value,
      positionedStatements: positionedStatements.value,
      depths: depths.value,
    } as FileContext;
  });
  provide(FILE_CONTEXT, context);
  watchEffect(() => {
    // we don't care if there's more than one since the state will be the same
    // (this is a bit inefficient but not that important for now)
    if (context.value != null) {
      fileContexts.value[file.value?.file?.id ?? ""] = context.value;
    }
  });
  onBeforeUnmount(() => {
    delete fileContexts.value[file.value?.file?.id ?? ""];
  });
  // also provide navigation context
  return provideNavigationContext(context);
}

export function useFileContext(): FileContext {
  const context = inject(FILE_CONTEXT) as FileContext | undefined;
  if (context == null) {
    throw new Error("File context not provided");
  }
  return context;
}

// navigation

export const NAVIGATION_CONTEXT = "__navigationContext__" as const;

export const CLIPBOARD_CONTENT_TYPE = "text/plain";
export type CopiedStatement = {
  id: string;
  parentId: string | null;
  parentInCopy?: boolean;
  orderKey: string;
};

export type StatementLocation = {
  fileId: string;
  parentId?: string | undefined;
  orderKey: string;
};

export type NavigationContext = FileContext & {
  current: CurrentNavigationContext;

  // utils
  getLocation(statement: StatementHeader): StatementLocation;
  getLocationRightAbove(statement: StatementHeader): StatementLocation;
  getLocationRightBelow(statement: StatementHeader): StatementLocation;
  getLocationStart(): StatementLocation;
  getLocationEnd(): StatementLocation;
  getSiblings(statement: StatementHeader): StatementHeader[];
  getPreviousSibling(statement: StatementHeader): StatementHeader | null;
  getNextSibling(statement: StatementHeader): StatementHeader | null;
  getLocalRoots(statements: StatementHeader[]): StatementHeader[];
  getDescendants(statement: StatementHeader): StatementHeader[];
  getAboveCurGroup(statement: StatementHeader): StatementHeader | null;
  getBelowCurGroup(statement: StatementHeader): StatementHeader | null;
  getAbove(statement: StatementHeader): StatementHeader | null;
  getBelow(statement: StatementHeader): StatementHeader | null;
  isDescendantOf(statement: StatementHeader, ancestor: StatementHeader): boolean;

  // indentation
  indent(statement: StatementHeader): void;
  unindent(statement: StatementHeader): void;
  indentBatch(statements: StatementHeader[]): void;
  unindentBatch(statements: StatementHeader[]): void;

  // movement
  moveUp(statement: StatementHeader): void;
  moveDown(statement: StatementHeader): void;
  moveTo(statement: StatementHeader, location: StatementLocation): void;
  moveBatchUp(statements: StatementHeader[]): void;
  moveBatchDown(statements: StatementHeader[]): void;

  // copy/paste
  copy(statements: StatementHeader[]): void;
  paste(statements?: CopiedStatement[], below?: StatementHeader): void;

  // selection
  getSelectedRoots(): StatementHeader[];
  getSelectionBottom(): StatementHeader | null;
};

export type CurrentNavigationContext = {
  statement: StatementHeader | null;
  component: InstanceType<typeof StatementInterface> | null;
  orderKey: string | null;
  previousSibling: StatementHeader | null;
  children: StatementHeader[];
  position: number;
  location: { fileId: string; parentId: string | null; orderKey: string };
  above: StatementHeader | null;
  below: StatementHeader | null;
  aboveCurGroup: StatementHeader | null;
  belowCurGroup: StatementHeader | null;
};

export function provideNavigationContext(file: Ref<FileContext | null>) {
  const bench = useBenchState();
  const ops = useOperations();

  const statements = computed(() => file.value?.statements ?? []);
  const depths = computed(() => file.value?.depths ?? []);
  const statementsById: Ref<Record<string, StatementHeader>> = computed(() => file.value?.statementsById ?? {});
  const statementsByParentId: Ref<Record<string, StatementHeader[]>> = computed(
    () => file.value?.statementsByParentId ?? {}
  );
  const statementPositions: Ref<Record<string, number>> = computed(() => file.value?.statementPositions ?? {});

  // utils

  function getLocation(statement: StatementHeader): StatementLocation {
    return {
      fileId: file.value?.file?.id,
      parentId: statement.parent?.id,
      orderKey: statement?.orderKey ?? INTEGER_ZERO,
    };
  }

  function getLocationRightAbove(statement: StatementHeader): StatementLocation {
    const above = getAbove(statement as StatementHeader);
    const statementParentId = statementsById.value[statement.id]?.parent?.id;
    const orderKey = generateKeyBetween(
      above != null && above.parent?.id == statementParentId ? above?.orderKey : null,
      statement.orderKey
    );
    return {
      fileId: file.value?.file?.id,
      parentId: statement.parent?.id,
      orderKey: orderKey,
    };
  }

  function getLocationRightBelow(statement: StatementHeader): StatementLocation {
    const below = getBelow(statement as StatementHeader);
    const statementParentId = statementsById.value[statement.id]?.parent?.id;
    const orderKey = generateKeyBetween(
      statement.orderKey,
      below != null && below.parent?.id == statementParentId ? below?.orderKey : null
    );
    return {
      fileId: file.value?.file?.id,
      parentId: statement.parent?.id,
      orderKey: orderKey,
    };
  }

  function getLocationStart(): StatementLocation {
    const roots = statementsByParentId.value[""] ?? [];
    const orderKey = roots[0]?.orderKey ?? INTEGER_ZERO;
    return {
      fileId: file.value?.file?.id,
      parentId: undefined,
      orderKey: generateKeyBetween(null, orderKey),
    };
  }

  function getLocationEnd(): StatementLocation {
    const roots = statementsByParentId.value[""] ?? [];
    const orderKey = roots.slice(-1)[0]?.orderKey ?? INTEGER_ZERO;
    return {
      fileId: file.value?.file?.id,
      parentId: undefined,
      orderKey: generateKeyBetween(orderKey, null),
    };
  }

  function getSiblings(statement?: StatementHeader): StatementHeader[] {
    return statementsByParentId.value[statement?.parent?.id ?? ""] ?? [];
  }

  function getPreviousSibling(statement: StatementHeader): StatementHeader | null {
    const siblings = getSiblings(statement);
    return siblings[siblings.findIndex((s) => s.id === statement.id) - 1];
  }

  function getNextSibling(statement: StatementHeader): StatementHeader | null {
    const siblings = getSiblings(statement);
    return siblings[siblings.findIndex((s) => s.id === statement.id) + 1];
  }

  function getLocalRoots(statements: StatementHeader[]): StatementHeader[] {
    // Gets the local roots of a set of statements (ordered by position)
    // (it can happen that we first select a child, then expand to parent, we only want parent)
    let localRoots = statements.map((s) => s.id);
    // trim statements whose parents are also selected
    for (const statement of statements) {
      const children = statementsByParentId.value[statement.id] ?? [];
      localRoots = localRoots.filter((r) => !children.find((s) => s.id == r));
    }
    return localRoots
      .sort((a, b) => statementPositions.value[a] - statementPositions.value[b])
      .map((s) => statementsById.value[s]);
  }

  function getDescendants(statement: StatementHeader): StatementHeader[] {
    const result: StatementHeader[] = [];
    for (const child of statementsByParentId.value[statement.id] ?? []) {
      result.push(child);
      result.push(...getDescendants(child));
    }
    return result.sort((a, b) => statementPositions.value[a.id] - statementPositions.value[b.id]);
  }

  function getAboveCurGroup(statement: StatementHeader): StatementHeader | null {
    // previous statement before this with depth <= this depth
    for (let i = statementPositions.value[statement.id] - 1; i >= 0; i--) {
      if (depths.value[i] <= depths.value[statementPositions.value[statement.id]]) {
        return statements.value[i];
      }
    }
    return null;
  }

  function getBelowCurGroup(statement: StatementHeader): StatementHeader | null {
    // next statement after this with depth <= this depth
    for (let i = statementPositions.value[statement.id] + 1; i < statements.value.length; i++) {
      if (depths.value[i] <= depths.value[statementPositions.value[statement.id]]) {
        return statements.value[i];
      }
    }
    return null;
  }

  function getAbove(statement: StatementHeader): StatementHeader | null {
    return statements.value[statementPositions.value[statement.id] - 1];
  }

  function getBelow(statement: StatementHeader): StatementHeader | null {
    return statements.value[statementPositions.value[statement.id] + 1];
  }

  function isDescendantOf(statement: StatementHeader, ancestor: StatementHeader): boolean {
    return getDescendants(ancestor).find((s) => s.id == statement.id) != null;
  }

  // indentation

  async function indent(statement: StatementHeader, tx?: Transaction) {
    const previousSibling = getPreviousSibling(statement);
    if (previousSibling == null) {
      return;
    }
    const previousSiblingChildren = statementsByParentId.value[previousSibling.id] ?? [];
    const previousSiblingChildrenLast = previousSiblingChildren.slice(-1)[0];
    await ops.statement.move(tx ?? null, statement.id, getLocation(statement), {
      fileId: file.value?.file.id,
      parentId: previousSibling.id,
      orderKey: generateKeyBetween(previousSiblingChildrenLast?.orderKey ?? null, null),
    });
  }

  async function unindent(statement: StatementHeader, tx?: Transaction) {
    // move to after parent in grandparent's children
    const parent = statementsById.value[statement.parent?.id];
    const grandparent = statementsById.value[parent.parent?.id];
    const parentSiblings = statementsByParentId.value[grandparent?.id ?? ""];
    const parentNextSibling = parentSiblings.find((s) => s.orderKey > parent.orderKey);
    await ops.statement.move(tx ?? null, statement.id, getLocation(statement), {
      fileId: file.value?.file.id,
      parentId: grandparent?.id,
      orderKey: generateKeyBetween(parent.orderKey, parentNextSibling?.orderKey ?? null),
    });
  }

  async function indentBatch(statements: StatementHeader[]) {
    const roots = getLocalRoots(statements);
    const previousSibling = getPreviousSibling(roots[0]);
    if (previousSibling == null) {
      return;
    }
    const previousSiblingChildren = statementsByParentId.value[previousSibling.id] ?? [];
    const previousSiblingChildrenLast = previousSiblingChildren.slice(-1)[0];
    // insert all roots in order after previous sibling's children
    const ids = roots.map((r) => r.id);
    const oldLocations = roots.map((s) => getLocation(s));
    const insertOrderKeys = generateNKeysBetween(previousSiblingChildrenLast?.orderKey ?? null, null, roots.length);
    await ops.statement.batchMove(
      null,
      ids,
      oldLocations,
      insertOrderKeys.map((k) => ({ fileId: file.value?.file.id, parentId: previousSibling.id, orderKey: k }))
    );
  }

  async function unindentBatch(statements: StatementHeader[]) {
    const roots = getLocalRoots(statements);
    const parent = statementsById.value[roots[0].parent?.id];
    const grandparent = statementsById.value[parent.parent?.id];
    const parentSiblings = statementsByParentId.value[grandparent?.id ?? ""];
    const parentNextSibling = parentSiblings.find((s) => s.orderKey > parent.orderKey);
    // insert all roots in order after parent
    const ids = roots.map((r) => r.id);
    const oldLocations = roots.map((s) => getLocation(s));
    const insertOrderKeys = generateNKeysBetween(parent.orderKey, parentNextSibling?.orderKey ?? null, roots.length);
    await ops.statement.batchMove(
      null,
      ids,
      oldLocations,
      insertOrderKeys.map((k) => ({ fileId: file.value?.file.id, parentId: grandparent?.id, orderKey: k }))
    );
  }

  // movement

  async function moveTo(statement: StatementHeader, location: StatementLocation) {
    await ops.statement.move(null, statement.id, getLocation(statement), location);
  }

  async function moveUp(statement: StatementHeader) {
    // insert between above and above prev sibling (if any)
    const above = getAbove(statement);
    if (above == null) return;
    const aboveSiblings = statementsByParentId.value[above.parent?.id ?? ""];
    const abovePrevSibling = aboveSiblings
      .slice()
      .reverse()
      .find((s) => s.orderKey < above.orderKey);
    const orderKey = generateKeyBetween(abovePrevSibling?.orderKey ?? null, above.orderKey);
    const targetLocation = {
      fileId: file.value?.file.id,
      parentId: above?.parent?.id,
      orderKey,
    };
    await ops.statement.move(null, statement.id, getLocation(statement), targetLocation);
  }

  async function moveDown(statement: StatementHeader) {
    const belowCurGroup = getBelowCurGroup(statement);
    if (belowCurGroup == null) return;
    // insert between the next group below and its next sibling (if any)
    const belowSiblings = statementsByParentId.value[belowCurGroup.parent?.id ?? ""];
    const belowNextSibling = belowSiblings.find((s) => s.orderKey > (belowCurGroup as StatementHeader).orderKey);
    const orderKey = generateKeyBetween(belowCurGroup.orderKey ?? null, belowNextSibling?.orderKey ?? null);
    const targetLocation = {
      fileId: file.value?.file.id,
      parentId: belowCurGroup?.parent?.id,
      orderKey,
    };
    await ops.statement.move(null, statement.id, getLocation(statement), targetLocation);
  }

  async function moveBatchUp(statements: StatementHeader[]) {
    // move selected roots in line with the topmost selected statement
    // (insert between above and above prev sibling (if any))
    const roots = getLocalRoots(statements);
    const above = file.value?.statements[statementPositions.value[roots[0].id] - 1];
    if (above == null) return;
    const aboveSiblings = statementsByParentId.value[above.parent?.id ?? ""];
    const abovePrevSibling = aboveSiblings
      .slice()
      .reverse()
      .find((s) => s.orderKey < above.orderKey);
    const ids = roots.map((r) => r.id);
    const oldLocations = roots.map((s) => getLocation(s));
    const orderKeys = generateNKeysBetween(abovePrevSibling?.orderKey ?? null, above.orderKey, roots.length);
    const targetLocations = orderKeys.map((k) => ({
      fileId: file.value?.file.id,
      parentId: above.parent?.id,
      orderKey: k,
    }));
    await ops.statement.batchMove(null, ids, oldLocations, targetLocations);
  }

  async function moveBatchDown(statements: StatementHeader[]) {
    // move selected roots in line with the bottommost selected statement
    // (insert between the next group below and its next sibling (if any))
    const roots = getLocalRoots(statements);
    const belowCurGroup = getBelowCurGroup(roots[roots.length - 1]);
    if (belowCurGroup == null) return;
    const belowSiblings = statementsByParentId.value[belowCurGroup.parent?.id ?? ""];
    const belowNextSibling = belowSiblings.find((s) => s.orderKey > belowCurGroup.orderKey);
    const ids = roots.map((r) => r.id);
    const oldLocations = roots.map((s) => getLocation(s));
    const orderKeys = generateNKeysBetween(belowCurGroup.orderKey, belowNextSibling?.orderKey ?? null, roots.length);
    const targetLocations = orderKeys.map((k) => ({
      fileId: file.value?.file.id,
      parentId: belowCurGroup.parent?.id,
      orderKey: k,
    }));
    await ops.statement.batchMove(null, ids, oldLocations, targetLocations);
  }

  // selection

  const statement = computed(() => statementsById.value[bench.focusedStatementId as string]);

  function getSelectedRoots(): StatementHeader[] {
    // Gets the in-selection roots of selected statements (ordered by position)
    // (it can happen that we first select a child, then expand to parent, we only want parent)
    return getLocalRoots(
      bench.focusedFile?.selectedElementIds?.map((s) => statementsById.value[s]).filter((s) => s != null) ?? []
    );
  }

  function getSelectionBottom(): StatementHeader | undefined {
    if (file.value?.editor.hasSelection) {
      const selectedRoots = getSelectedRoots();
      return selectedRoots[selectedRoots.length - 1];
    } else if (statement.value != null) {
      return statement.value;
    } else {
      return statements.value[statements.value.length - 1];
    }
  }

  // copy/paste

  async function copy(statements: StatementHeader[]) {
    const selectedRoots = getLocalRoots(statements);
    const copiedStatements = []; // include selected roots and all descendants
    for (const root of selectedRoots) {
      // getDescendants is ordered already
      copiedStatements.push(root);
      copiedStatements.push(...getDescendants(root));
    }
    const copiedStatementsIds = new Set<string>();
    copiedStatements.forEach((s) => copiedStatementsIds.add(s.id));
    // write to clipboard as text/_bench-v0
    const sourceStatements = copiedStatements.map(
      (s) =>
        ({
          id: s.id,
          parentId: s.parent?.id,
          parentInCopy: s.parent == null ? undefined : copiedStatementsIds.has(s.parent?.id ?? ""),
          orderKey: s.orderKey,
        } as CopiedStatement)
    );
    const clipboardItem = [
      new ClipboardItem({
        [CLIPBOARD_CONTENT_TYPE]: new Blob([JSON.stringify(sourceStatements)], { type: CLIPBOARD_CONTENT_TYPE }),
      }),
    ];
    try {
      await navigator.clipboard.write(clipboardItem);
      console.log("copied " + copiedStatements.length + " statements");
    } catch (err) {
      console.error("failed to copy statements", err); // TODO @UX: handle copy error properly
    }
  }

  async function paste(sourceStatements?: CopiedStatement[], below?: StatementHeader | null) {
    try {
      if (sourceStatements == null) {
        const cliboardItems = await navigator.clipboard.read();
        const clipboardDataStr = await (await cliboardItems[0].getType(CLIPBOARD_CONTENT_TYPE)).text();
        sourceStatements = JSON.parse(clipboardDataStr) as CopiedStatement[];
      }

      // insert at bottom of current selection or file (like in insertBelow, below bottom and its next sibling)
      const bottom = below ?? getSelectionBottom();
      const nextSibling = bottom != null ? getNextSibling(bottom) : undefined;
      // project source ids and locations to target at insert point (with new ids)
      const sourceIds = sourceStatements.map((s) => s.id);
      const targetIds: Record<string, string> = {};
      sourceStatements.forEach((s) => (targetIds[s.id] = newStatementId()));
      const targetParentIds = sourceStatements.map((s) =>
        s.parentInCopy && s.parentId != null ? targetIds[s.parentId] : bottom?.parent?.id
      );
      // order keys for root are between bottom and next sibling, all other orders are reset
      const sourceStatementsByParentId: Record<string, string[]> = {};
      sourceStatements.forEach((s) => {
        // group children by parents
        const parentId = s.parentInCopy && s.parentId != null ? s.parentId : "";
        if (sourceStatementsByParentId[parentId] == null) {
          sourceStatementsByParentId[parentId] = [];
        }
        sourceStatementsByParentId[parentId].push(s.id);
      });
      const orderKeysByParentId: Record<string, string[]> = {}; // assign order keys by parent
      Object.entries(sourceStatementsByParentId).forEach(([parentId, childIds]) => {
        if (parentId == "") {
          orderKeysByParentId[""] = generateNKeysBetween(
            bottom?.orderKey ?? null,
            nextSibling?.orderKey ?? null,
            childIds.length
          );
        } else {
          orderKeysByParentId[parentId] = generateNKeysBetween(null, null, childIds.length);
        }
      });
      const targetOrderKeys: string[] = sourceStatements.map((s) => {
        const parentId = s.parentInCopy && s.parentId != null ? s.parentId : "";
        const orderKeys = orderKeysByParentId[parentId];
        const childIndex = sourceStatementsByParentId[parentId].indexOf(s.id);
        return orderKeys[childIndex];
      });

      await ops.statement.batchPaste(
        sourceIds,
        sourceIds.map((id) => targetIds[id]),
        file.value?.file.id,
        targetParentIds,
        targetOrderKeys
      );
      console.log("pasted " + sourceStatements.length + " statements");
      // select the pasted stuff
      if (bench.focusedStatementId != null && sourceIds.includes(bench.focusedStatementId)) {
        file.value?.editor.focusElement({ id: targetIds[bench.focusedStatementId], __typename: "Statement" });
      } else {
        file.value?.editor.blurElement();
      }
      file.value?.editor.clearSelection();
      Object.values(targetIds).forEach((targetId) => file.value?.editor.addToSelection({ id: targetId }));
    } catch (err) {
      console.error("failed to parse clipboard data", err);
      return;
    }
  }

  const context = computed(() => {
    if (file.value == null) return null;

    // current
    const current = {
      statement: statementsById.value[file.value.editor.activeStatementId as string],
      component: file.value.statementsComponents[file.value.editor.activeStatementId as string],
      orderKey: statement.value?.orderKey ?? INTEGER_ZERO,
      previousSibling: statement.value == null ? null : getPreviousSibling(statement.value),
      children: statementsByParentId.value[statement.value?.id ?? ""] ?? [],
      position: statementPositions.value[statement.value?.id],
      location: statement.value == null ? undefined : getLocation(statement.value),
      above: statement.value == null ? undefined : getAbove(statement.value),
      below: statement.value == null ? undefined : getBelow(statement.value),
      aboveCurGroup: statement.value == null ? undefined : getAboveCurGroup(statement.value),
      belowCurGroup: statement.value == null ? undefined : getBelowCurGroup(statement.value),
    } as CurrentNavigationContext;

    return {
      ...file.value,
      // utils
      getLocation,
      getLocationRightAbove,
      getLocationRightBelow,
      getLocationStart,
      getLocationEnd,
      getSiblings,
      getPreviousSibling,
      getNextSibling,
      getLocalRoots,
      getDescendants,
      getAboveCurGroup,
      getBelowCurGroup,
      getAbove,
      getBelow,
      isDescendantOf,

      // indentation
      indent,
      unindent,
      indentBatch,
      unindentBatch,

      // movement
      moveUp,
      moveDown,
      moveTo,
      moveBatchUp,
      moveBatchDown,

      // copy/paste
      copy,
      paste,

      // current
      current,

      // selection
      getSelectionBottom,
      getSelectedRoots,
    } as NavigationContext;
  });
  provide(NAVIGATION_CONTEXT, context);
  watchEffect(() => {
    if (context.value != null) {
      navigationContexts.value[file.value?.file.id] = context.value;
    }
  });
  onBeforeUnmount(() => {
    delete navigationContexts.value[file.value?.file.id];
  });
  return context;
}

export function useNavigationContext(): Ref<NavigationContext> {
  const context = inject(NAVIGATION_CONTEXT) as Ref<NavigationContext> | undefined;
  if (context == null) {
    throw new Error("File context not provided");
  }
  return context;
}

export function useMagicActions(statement: Ref<StatementHeader | null>) {
  // "magic" because I don't know what to call these yet, they're not like the other statement actions (which are actual 'actions')
  const nav = useNavigationContext();
  const ops = useOperations();
  const objects = useObjects();
  const bench = useBenchState();

  async function insertBelow(focus?: boolean) {
    if (statement.value == null) return;
    const below = nav.value.getLocationRightBelow(statement.value as StatementHeader);
    const newStatement = { __typename: "Statement", id: newStatementId() };
    ops.statement.create(null, newStatement.id, below.fileId, below.parentId, below.orderKey);
    if (focus) {
      nav.value.editor.editElement(newStatement as StatementHeader);
    }
  }

  async function insertAbove(focus?: boolean) {
    if (statement.value == null) return;
    const above = nav.value.getLocationRightAbove(statement.value as StatementHeader);
    const newStatement = { __typename: "Statement", id: newStatementId() };
    ops.statement.create(null, newStatement.id, above.fileId, above.parentId, above.orderKey);
    if (focus) {
      nav.value.editor.editElement(newStatement as StatementHeader);
    }
  }

  async function duplicate() {
    if (statement.value == null) return;
    nav.value.copy([statement.value as StatementHeader]);
    nav.value.paste(undefined, statement.value as StatementHeader);
  }

  async function delete_() {
    if (statement.value == null) return;
    ops.statement.softDelete(null, statement.value.id);
  }

  async function moveFocusUp() {
    if (statement.value == null) return;
    const above = nav.value.getAbove(statement.value);
    if (above == null) return;
    nav.value.editor.focusElement(above);
  }

  async function moveFocusDown() {
    if (statement.value == null) return;
    const below = nav.value.getBelow(statement.value);
    if (below == null) return;
    nav.value.editor.focusElement(below);
  }

  async function insertFilesAsRecords(key: string, orderKeys: string[], files: File[], as?: string) {
    /** Insert files as records into this statement */
    if (!as && statement.value?.symbolType != SymbolType.Data) {
      throw new Error("can only insert records into data statements");
    }
    const newRecordIds = files.map(() => newDatasetRecordId());
    const tx = openTransaction({
      name: "insertFilesAsRecords",
      blockPartialUndo: true,
      undo: async () => {
        await ops.symbol.batchSoftDeleteRecord(newRecordIds);
      },
      redo: async () => {
        await ops.symbol.batchRestoreRecord(newRecordIds);
      },
    });
    const uploads = [];
    for (let i = 0; i < files.length; i++) {
      const newRecordId = newRecordIds[i];
      const file = files[i];
      const orderKey = orderKeys[i];
      const remoteObject = await ops.object.prepareUpload(bench.currentProjectId as string, file);
      const data = { [key]: remoteObject };
      ops.symbol.createRecord(tx, newRecordId, as ?? statement.value?.id, orderKey, data);
      uploads.push(
        objects.upload(bench.currentProjectId as string, file, (updatedObject) => {
          const newData = { ...data, [key]: updatedObject };
          ops.symbol.updateRecord(tx, newRecordId, data, newData);
        })
      );
    }
    await Promise.all(uploads);
    closeTransaction(tx);
  }

  async function insertFilesAsDataset(location: "above" | "below" | StatementLocation, files: File[]) {
    /** Insert files as a new dataset above/below this statement */
    // get location above/below
    if (location == "above" || location == "below") {
      if (statement.value == null) {
        throw new Error(`statement must be set if using relative location`);
      }
      location =
        location == "above"
          ? nav.value.getLocationRightAbove(statement.value)
          : nav.value.getLocationRightBelow(statement.value);
    }
    const dataset = {
      id: newStatementId(),
      parentId: location.parentId,
      orderKey: location.orderKey,
      fileId: nav.value.file.id,
      symbolType: SymbolType.Data,
      rootTypeTag: TypeTag.Struct,
      rootTypeFlags: TypeFlag.IsArray,
      name: getRandomAdjective() + " documents",
    };
    const tx = openTransaction({
      name: "insertFilesAsDataset",
      blockPartialUndo: true,
      undo: async () => {
        await ops.statement.softDelete(null, dataset.id);
      },
      redo: async () => {
        await ops.statement.restore(null, dataset.id);
      },
    });
    // create new dataset
    // TODO @UX: insert files tx should be reduced to soft delete/restore statement for undo/redo
    ops.statement.createDefinition(tx, dataset);
    // create 'content' column with file type
    const contentKey = newTypeNodeKey();
    ops.symbol.createTypeNode(tx, dataset.id, {
      statementId: dataset.id,
      id: newTypeNodeId(),
      key: contentKey,
      name: "content",
      tag: TypeTag.File,
      orderKey: INTEGER_ZERO,
    });
    closeTransaction(tx);

    // insert files into dataset
    const orderKeys = generateNKeysBetween(null, null, files.length);
    await insertFilesAsRecords(contentKey, orderKeys, files, dataset.id);
  }

  return {
    insertBelow,
    insertAbove,
    duplicate,
    delete: delete_,
    moveFocusUp,
    moveFocusDown,
    insertFilesAsDataset,
    insertFilesAsRecords,
  };
}
