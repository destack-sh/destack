import type StatementComponent from "@/components/statements/InlineStatement.vue";
import type { StatementContentFragment } from "@/gql/graphql";
import { EditFilePanel, useBenchState, type FileHeader, type StatementHeader, type Action } from "@/state/bench";
import {
  orderStatements,
  TypeFlag,
  type OrderedStatement,
  type Field,
  type Statement,
  newNodeIdentity,
} from "@/state/module";
import { useObjects } from "@/state/blob";
import { useOperations, type Transaction } from "@/state/operations";
import { generateKeyBetween, generateNKeysBetween, INTEGER_ZERO } from "@/utils/fractional";
import { DocumentDuplicateIcon, TrashIcon } from "@heroicons/vue/24/outline";
import { computed, inject, onBeforeUnmount, provide, ref, watchEffect, type Ref, nextTick } from "vue";

export const FILE_CONTEXT = "__fileContext__" as const;

export type FileState = {
  panel: EditFilePanel;
  editing: boolean;
  focused: boolean;
  file: FileHeader;
  statementsUnordered: Statement[]; // unordered
  statementsComponents: Record<string, InstanceType<typeof StatementComponent>>;
  navigateUp: () => void;
  navigateDown: () => void;
};

export type FileContext = FileState & {
  statements: StatementHeader[]; // ordered
  statementsById: Record<string, Statement>;
  statementsByCk: Record<string, Statement>;
  statementsComponents: Record<string, InstanceType<typeof StatementComponent>>;
  statementsByParentId: Record<string, Statement[]>;
  statementPositions: Record<string, number>;
  positionedStatements: OrderedStatement<StatementContentFragment>[];
  depths: number[];
};

// There can only be one active file to provide file shortcuts,
// so we have a global reference here that is automatically set to the focused file.
// We can't just use singleton actions here because multiple files may have
// 'focused' set during moves or transition.
export const activeFileState: Ref<FileState | null> = ref(null);
export const fileContexts = ref<Record<string, FileContext>>({});
export const navigationContexts = ref<Record<string, NavigationContext>>({});

export function provideFileState(file: Ref<FileState | null>) {
  // set active file if focused
  watchEffect(() => {
    if (file.value?.focused) {
      if (activeFileState.value?.panel.id !== file.value?.panel.id) {
        activeFileState.value = file.value;
      }
    } else if (activeFileState.value?.panel.id === file.value?.panel.id) {
      activeFileState.value = null;
    }
  });
  onBeforeUnmount(() => {
    if (activeFileState.value?.panel.id === file.value?.panel.id) {
      activeFileState.value = null;
    }
  });

  // file context
  const statementsUnordered = computed(() => file.value?.statementsUnordered ?? []);
  const statementsById = computed(() => {
    const statementsById: globalThis.Record<string, Statement> = {};
    statementsUnordered.value.forEach((statement) => {
      statementsById[statement.id] = statement;
    });
    return statementsById;
  });
  const statementsByCk = computed(() => {
    const statementsByCk: globalThis.Record<string, Statement> = {};
    statementsUnordered.value.forEach((statement) => {
      statementsByCk[statement.ck] = statement;
    });
    return statementsByCk;
  });

  const statementPositions = computed(() => {
    const result: Record<string, number> = {};
    for (let i = 0; i < statements.value.length; i++) {
      result[statements.value[i].id] = i;
    }
    return result;
  });

  // TODO @Cleanup: use module.orderStatements here (like in StatementExplorer)
  const ordered = computed(() => orderStatements(file.value?.statementsUnordered ?? []));
  const statements = computed(() => ordered.value.ordered.map((positioned) => positioned.statement));
  const depths = computed(() => ordered.value.ordered.map((positioned) => positioned.depth));

  // provide context
  const context = computed(() => {
    if (file.value == null) return null;
    return {
      ...file.value,
      statements: statements.value,
      statementsComponents: file.value.statementsComponents ?? {},
      statementsById: statementsById.value,
      statementsByCk: statementsByCk.value,
      statementsByParentId: ordered.value.statementsByParentId,
      statementPositions: statementPositions.value,
      positionedStatements: ordered.value.ordered,
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
  ck: string;
  parentId: string | null;
  parentInCopy?: boolean;
  orderKey: string;
};

export type StatementLocation = {
  fileId: string;
  parentId?: string | undefined;
  orderKey: string;
};

// TODO @Cleanup @Architecture: decouple navigation from file context? it's all a bit convoluted..
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
  getAboveGroup(statement: StatementHeader): StatementHeader | null;
  getBelowGroup(statement: StatementHeader): StatementHeader | null;
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
  moveBatchTo(statements: StatementHeader[], pos: "above" | "below", target: StatementHeader): void;

  // copy/paste
  copy(statements: StatementHeader[]): void;
  paste(statements?: CopiedStatement[], below?: StatementHeader): void;

  // selection
  getSelectedRoots(): StatementHeader[];
  getSelectionBottom(): StatementHeader | null;
  selectionActions: Action<void>[];
};

export type CurrentNavigationContext = {
  statement: StatementHeader | null;
  component: InstanceType<typeof StatementComponent> | null;
  orderKey: string | null;
  previousSibling: StatementHeader | null;
  children: StatementHeader[];
  position: number;
  location: { fileId: string; parentId: string | null; orderKey: string };
  above: StatementHeader | null;
  below: StatementHeader | null;
  aboveGroup: StatementHeader | null;
  belowGroup: StatementHeader | null;
};

export function provideNavigationContext(file: Ref<FileContext | null>) {
  const bench = useBenchState();
  const ops = useOperations();

  const statements = computed(() => file.value?.statements ?? []);
  const depths = computed(() => file.value?.depths ?? []);
  const statementsById: Ref<Record<string, StatementHeader>> = computed(() => file.value?.statementsById ?? {});
  const statementsByCk: Ref<Record<string, StatementHeader>> = computed(() => file.value?.statementsByCk ?? {});
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
    const above = getPreviousSibling(statement);
    const orderKey = generateKeyBetween(above != null ? above?.orderKey : null, statement.orderKey);
    return {
      fileId: file.value?.file?.id,
      parentId: statement.parent?.id,
      orderKey: orderKey,
    };
  }

  function getLocationRightBelow(statement: StatementHeader): StatementLocation {
    const below = getNextSibling(statement);
    const orderKey = generateKeyBetween(statement.orderKey, below != null ? below?.orderKey : null);
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

  function getAboveGroup(statement: StatementHeader): StatementHeader | null {
    // previous statement before this with depth <= this depth
    for (let i = statementPositions.value[statement.id] - 1; i >= 0; i--) {
      if (depths.value[i] <= depths.value[statementPositions.value[statement.id]]) {
        return statements.value[i];
      }
    }
    return null;
  }

  function getBelowGroup(statement: StatementHeader): StatementHeader | null {
    // next statement after this with depth <= this depth
    for (let i = statementPositions.value[statement.id] + 1; i < statements.value.length; i++) {
      if (depths.value[i] <= depths.value[statementPositions.value[statement.id]]) {
        return statements.value[i];
      }
    }
    return null;
  }

  function getAboveInOrder(statement: StatementHeader): StatementHeader | null {
    return statements.value[statementPositions.value[statement.id] - 1];
  }

  function getBelowInOrder(statement: StatementHeader): StatementHeader | null {
    return statements.value[statementPositions.value[statement.id] + 1];
  }

  function isDescendantOf(statement: StatementHeader, ancestor: StatementHeader): boolean {
    return getDescendants(ancestor).find((s) => s.id == statement.id) != null;
  }

  function getDepth(statement: StatementHeader): number {
    return depths.value[statementPositions.value[statement.id]];
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
    if (parent == null) return;
    const grandparent = statementsById.value[parent?.parent?.id] ?? file.value?.file;
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
    if (parent == null) return;
    const grandparent = statementsById.value[parent?.parent?.id] ?? file.value?.file;
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
    const above = getAboveInOrder(statement);
    if (above == null) return;
    if (getDepth(above) > getDepth(statement)) return await indent(statement);
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
    const belowGroup = getBelowGroup(statement);
    if (belowGroup == null) return;
    // insert between the next group below and its next sibling (if any)
    const belowSiblings = statementsByParentId.value[belowGroup.parent?.id ?? ""];
    const belowNextSibling = belowSiblings.find((s) => s.orderKey > (belowGroup as StatementHeader).orderKey);
    if (getDepth(belowGroup) < getDepth(statement)) return await unindent(statement);
    const orderKey = generateKeyBetween(belowGroup.orderKey ?? null, belowNextSibling?.orderKey ?? null);
    const targetLocation = {
      fileId: file.value?.file.id,
      parentId: belowGroup?.parent?.id,
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
    if (getDepth(above) > getDepth(roots[0])) return await indentBatch(statements);
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
    const belowCurGroup = getBelowGroup(roots[roots.length - 1]);
    if (belowCurGroup == null) return;
    if (getDepth(belowCurGroup) < getDepth(roots[0])) return await unindentBatch(statements);
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

  async function moveBatchTo(statements: StatementHeader[], pos: "above" | "below", target: StatementHeader) {
    // move selected roots in line with the topmost selected statement to location
    // (location is for topmost selected root)
    const roots = getLocalRoots(statements);
    const ids = roots.map((r) => r.id);
    const oldLocations = roots.map((s) => getLocation(s));
    let targetLocations;
    if (pos == "above") {
      // exactly like moveBatchUp except we insert between target and target prev sibling (if any, else null)
      const targetSiblings = statementsByParentId.value[target.parent?.id ?? ""];
      const targetPrevSibling = targetSiblings
        .slice()
        .reverse()
        .find((s) => s.orderKey < target.orderKey);
      const orderKeys = generateNKeysBetween(targetPrevSibling?.orderKey ?? null, target.orderKey, roots.length);
      targetLocations = orderKeys.map((k) => ({
        fileId: file.value?.file.id,
        parentId: target.parent?.id,
        orderKey: k,
      }));
    } else {
      // exactly like moveBatchDown except we insert between target and target next sibling (if any, else null)
      const targetSiblings = statementsByParentId.value[target.parent?.id ?? ""];
      const targetNextSibling = targetSiblings.find((s) => s.orderKey > target.orderKey);
      const orderKeys = generateNKeysBetween(target.orderKey, targetNextSibling?.orderKey ?? null, roots.length);
      targetLocations = orderKeys.map((k) => ({
        fileId: file.value?.file.id,
        parentId: target.parent?.id,
        orderKey: k,
      }));
    }
    await ops.statement.batchMove(null, ids, oldLocations, targetLocations);
  }

  // selection

  const statement = computed(() => statementsByCk.value[file.value?.panel.activeStatementCk as string]);

  function getSelectedRoots(): StatementHeader[] {
    // Gets the in-selection roots of selected statements (ordered by position)
    // (it can happen that we first select a child, then expand to parent, we only want parent)
    return getLocalRoots(
      bench.focusedFile?.selectedElementIds?.map((s) => statementsById.value[s]).filter((s) => s != null) ?? []
    );
  }

  function getSelectionBottom(): StatementHeader | undefined {
    if (file.value?.panel.hasSelection) {
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
    // :ClipboardSchema
    const sourceStatements = copiedStatements.map(
      (s) =>
        ({
          id: s.id,
          ck: s.ck,
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
        sourceStatements = JSON.parse(clipboardDataStr) as CopiedStatement[]; //  :ClipboardSchema
      }

      // insert at bottom of current selection or file (like in insertBelow, below bottom and its next sibling)
      const bottom = below ?? getSelectionBottom();
      const nextSibling = bottom != null ? getNextSibling(bottom) : undefined;

      // project source ids and locations to target at insert point (with new ids/cks)
      // (pre-generate these identities so we know them ahead of time)
      const sourceIds = sourceStatements.map((s) => s.id);
      const sourceCks = sourceStatements.map((s) => s.ck);
      const targetIds: Record<string, string> = {};
      const targetCks: Record<string, string> = {};
      sourceStatements.forEach((s) => {
        const identity = newNodeIdentity(bench.projectVersionId as string, "Statement");
        targetIds[s.id] = identity.id;
        targetCks[s.ck] = identity.ck;
      });
      const targetParentIds = sourceStatements.map((s) =>
        s.parentInCopy && s.parentId != null ? targetIds[s.parentId] : bottom?.parent?.id
      );

      // group children by parents
      const sourceStatementsByParentId: Record<string, string[]> = {};
      sourceStatements.forEach((s) => {
        const parentId = s.parentInCopy && s.parentId != null ? s.parentId : "";
        if (sourceStatementsByParentId[parentId] == null) {
          sourceStatementsByParentId[parentId] = [];
        }
        sourceStatementsByParentId[parentId].push(s.id);
      });

      // order keys for root are between bottom and next sibling, all other orders are reset
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

      // add pasting indicator to bottom
      const component = file.value?.statementsComponents[bottom?.id] as InstanceType<typeof StatementComponent>;
      component.pasting = true;

      // do the paste
      const targetRootIds = sourceStatements.filter((s) => !s.parentInCopy).map((s) => targetIds[s.id]);
      await ops.statement.batchPaste(
        sourceIds,
        sourceIds.map((id) => targetIds[id]),
        sourceCks.map((ck) => targetCks[ck]),
        file.value?.file.id,
        targetParentIds,
        targetOrderKeys,
        targetRootIds
      );
      component.pasting = false;
      console.log("pasted " + sourceStatements.length + " statements");

      // select the pasted stuff
      if (bench.focusedStatementId != null && sourceIds.includes(bench.focusedStatementId)) {
        file.value?.panel.focusElement({
          id: targetIds[bench.focusedStatementId],
          ck: targetCks[bench.focusedStatementCk as string],
          __typename: "Statement",
        });
      } else {
        file.value?.panel.blurElement();
      }
      file.value?.panel.clearSelection();
      Object.values(targetIds).map((id, i) =>
        file.value?.panel.addToSelection({ id, ck: targetCks[i], __typename: "Statement" })
      );
    } catch (err) {
      console.error("failed to parse clipboard data", err);
      return;
    }
  }

  const selectionActions: Action<void>[] = [
    {
      groupId: "edit-core",
      label: "Copy",
      icon: DocumentDuplicateIcon,
      hideInline: true,
      action: () => {
        copy(getSelectedRoots());
      },
    },
    {
      groupId: "edit-core",
      label: "Duplicate",
      icon: DocumentDuplicateIcon,
      hideInMenu: bench.readonly,
      disabled: bench.readonly,
      action: () => {
        copy(getSelectedRoots());
        paste();
      },
    },
    {
      groupId: "edit-core",
      label: "Delete",
      icon: TrashIcon,
      hideInMenu: bench.readonly,
      disabled: bench.readonly,
      action: () => {
        if (file.value?.panel.selectedElementIds == null) return;
        ops.statement.batchSoftDelete(file.value?.panel.selectedElementIds);
      },
    },
  ];

  const context = computed(() => {
    if (file.value == null) return null;

    // current
    const current = {
      statement: statement.value,
      component: file.value.statementsComponents[statement.value?.id],
      orderKey: statement.value?.orderKey ?? INTEGER_ZERO,
      previousSibling: statement.value == null ? null : getPreviousSibling(statement.value),
      children: statementsByParentId.value[statement.value?.id ?? ""] ?? [],
      position: statementPositions.value[statement.value?.id],
      location: statement.value == null ? undefined : getLocation(statement.value),
      above: statement.value == null ? undefined : getAboveInOrder(statement.value),
      below: statement.value == null ? undefined : getBelowInOrder(statement.value),
      aboveGroup: statement.value == null ? undefined : getAboveGroup(statement.value),
      belowGroup: statement.value == null ? undefined : getBelowGroup(statement.value),
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
      getAboveGroup,
      getBelowGroup,
      getAbove: getAboveInOrder,
      getBelow: getBelowInOrder,
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
      moveBatchTo,

      // copy/paste
      copy,
      paste,

      // current
      current,

      // selection
      getSelectionBottom,
      getSelectedRoots,
      selectionActions,
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

export function canPaste(data: string) {
  try {
    const sourceStatements = JSON.parse(data) as CopiedStatement[];
    // check if all statements have the proper format  :ClipboardSchema
    const valid = sourceStatements.every(
      (s) =>
        s.id != null &&
        s.parentId != null &&
        s.orderKey != null &&
        (s.parentInCopy == null || typeof s.parentInCopy == "boolean")
    );
    return valid;
  } catch (err) {
    console.error("failed to parse clipboard data", err);
    return false;
  }
}

export function useNavigationContext(required = true): Ref<NavigationContext> | null {
  const context = inject(NAVIGATION_CONTEXT, null) as Ref<NavigationContext> | null;
  if (context == null && required) {
    throw new Error("File context not provided");
  }
  return context;
}

export function useMagicActions(statement: Ref<StatementHeader | null>) {
  // "magic" because I don't know what to call these yet, they're not like the other statement actions (which are actual 'actions')
  const nav = useNavigationContext(false);
  const ops = useOperations();
  const objects = useObjects();
  const bench = useBenchState();

  async function insertBelow(focus?: boolean) {
    if (statement.value == null || nav == null) return;
    const below = nav?.value?.getLocationRightBelow(statement.value as StatementHeader);
    const newStatement = { __typename: "Statement", ...newNodeIdentity(bench.projectVersionId as string, "Statement") };
    ops.statement.create(null, newStatement.id, newStatement.ck, below.fileId, below.parentId, below.orderKey);
    if (focus) {
      nav?.value?.panel.editElement(newStatement as StatementHeader);
      nextTick(() => nav?.value?.statementsComponents[newStatement.id]?.focus("first"));
    }
  }

  async function insertAbove(focus?: boolean) {
    if (statement.value == null || nav == null) return;
    const above = nav?.value?.getLocationRightAbove(statement.value as StatementHeader);
    const newStatement = { __typename: "Statement", ...newNodeIdentity(bench.projectVersionId as string, "Statement") };
    ops.statement.create(null, newStatement.id, newStatement.ck, above.fileId, above.parentId, above.orderKey);
    if (focus) {
      nav?.value?.panel.editElement(newStatement as StatementHeader);
      nextTick(() => nav?.value?.statementsComponents[newStatement.id]?.focus("first"));
    }
  }

  async function copy() {
    if (nav == null) throw new Error("nav context not provided");
    if (statement.value == null) return;
    nav?.value?.copy([statement.value as StatementHeader]);
  }

  async function duplicate() {
    if (nav == null) throw new Error("nav context not provided");
    if (statement.value == null) return;
    nav?.value?.copy([statement.value as StatementHeader]);
    nav?.value?.paste(undefined, statement.value as StatementHeader);
  }

  async function delete_() {
    if (statement.value == null) return;
    ops.statement.softDelete(null, statement.value.id);
  }

  async function moveFocusUp() {
    if (nav == null) throw new Error("nav context not provided");
    if (statement.value == null) return;
    const above = nav?.value?.getAbove(statement.value);
    if (above == null) return;
    nav?.value?.panel.focusElement(above);
  }

  async function moveFocusDown() {
    if (nav == null) throw new Error("nav context not provided");
    if (statement.value == null) return;
    const below = nav?.value?.getBelow(statement.value);
    if (below == null) return;
    nav?.value?.panel.focusElement(below);
  }

  return {
    insertBelow,
    insertAbove,
    copy,
    duplicate,
    delete: delete_,
    moveFocusUp,
    moveFocusDown,
  };
}
