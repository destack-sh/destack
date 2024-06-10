import { log } from "@/utils/log";
import { computed, ref, type Ref } from "vue";

export function useElementRefs<RefType = HTMLInputElement>(
  elements?: Ref<{ id: string }[]>,
  options?: {
    onRegister?: (id: string, ref: RefType) => void;
    onUnregister?: (id: string, ref: RefType) => void;
    navigateLeft?: (index: number) => void;
    navigateRight?: (index: number) => void;
  },
) {
  const refs: Ref<Record<string, RefType>> = ref({});

  function registerRef(id: string, ref: RefType | undefined) {
    if (ref != undefined) {
      refs.value[id] = ref;
      if (options?.onRegister) {
        options.onRegister(id, ref);
      }
    } else {
      if (options?.onUnregister) {
        options.onUnregister(id, refs.value[id]);
      }
      delete refs.value[id];
    }
  }

  function getRef(id: string): RefType {
    return refs.value[id];
  }

  function focus(id: string | number) {
    id = typeof id == "number" ? elements?.value?.[id].id ?? "" : id;
    (refs.value[id] as { focus?: () => void })?.focus?.();
  }

  function navigateRight(id: string | number) {
    if (elements == null) throw new Error("elements not set");
    const index = typeof id == "number" ? id : elements?.value?.findIndex((m) => m.id === id) ?? -1;
    if (index == elements?.value?.length - 1) {
      options?.navigateRight?.(index);
    } else {
      focus(elements?.value?.[index + 1].id);
    }
  }

  function navigateLeft(id: string | number) {
    if (elements == null) throw new Error("elements not set");
    const index = typeof id == "number" ? id : elements?.value?.findIndex((m) => m.id === id) ?? -1;
    if (index == 0) {
      options?.navigateLeft?.(index);
    } else {
      focus(elements?.value?.[index - 1].id);
    }
  }

  return {
    registerRef,
    refs: computed(() => Object.values(refs.value)),
    getRef,
    focus,
    navigateLeft,
    navigateRight,
  };
}

export function useNavigationGrid<ColumnType = string, RefType = HTMLInputElement>(
  columnsInOrder: Ref<ColumnType[]>,
  rows: Ref<{ id: string }[]>,
  options: {
    gridNavigateUp?: (index: number, column: ColumnType, columnIndex: number) => void;
    gridNavigateDown?: (index: number, column: ColumnType, columnIndex: number) => void;
    gridNavigateLeft?: (index: number, column: ColumnType, columnIndex: number) => void;
    gridNavigateRight?: (index: number, column: ColumnType, columnIndex: number) => void;
    nowrapLeft?: boolean;
    nowrapRight?: boolean;
    nowrapTop?: boolean;
    nowrapBottom?: boolean;
    onFocus?: (rowId: string, column: ColumnType) => void;
  } = {},
) {
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  const gridId = Math.random().toString(36).substring(2, 15); // just for debugging
  const columnRefs: Ref<Record<string, RefType>> = ref({});
  const rowsLength = computed(() => rows.value?.length ?? 0);
  const onRowAvailable: Record<string, ((ref: RefType) => void)[]> = {};

  let columnRefsBatchChange: Record<string, RefType | "delete"> | null = null;

  function registerColumnRef(rowId: string, column: ColumnType, ref: RefType | undefined) {
    const columnId = rowId + "." + column;
    if (ref != undefined) {
      if (columnRefs.value[columnId] !== ref) {
        if (onRowAvailable[columnId] != null) {
          onRowAvailable[columnId]?.forEach((cb) => cb(ref));
          delete onRowAvailable[columnId];
        }
        if (columnRefsBatchChange != null) {
          columnRefsBatchChange[columnId] = ref;
        } else {
          columnRefs.value[columnId] = ref;
        }
      }
    } else {
      if (columnRefsBatchChange != null) {
        columnRefsBatchChange[columnId] = "delete";
      } else {
        delete columnRefs.value[columnId];
      }
    }
  }

  function onColumnAvailable(rowId: string, column: ColumnType, cb: (ref: RefType) => void) {
    const columnId = rowId + "." + column;
    if (columnRefs.value[columnId] != null) {
      cb(columnRefs.value[columnId]);
    } else {
      if (onRowAvailable[columnId] == null) {
        onRowAvailable[columnId] = [];
      }
      onRowAvailable[columnId]?.push(cb);
    }
  }

  function beginBatchChange() {
    // used when we need to add/remove columns since directly writing columnRefs via :ref
    // seems to trigger a re-render frame for every row + modified column
    // TODO @Performance @Robustness: improve grid update batching
    //  For instance this doesn't work for other connected clients in multiplayer since they
    //  will be receiving the changes via the direct mutation.
    columnRefsBatchChange = {};
  }

  function flush() {
    if (columnRefsBatchChange == null) {
      return;
    }
    for (const [key, value] of Object.entries(columnRefsBatchChange)) {
      if (value === "delete") {
        delete columnRefs.value[key];
      } else {
        columnRefs.value[key] = value;
      }
    }
    columnRefsBatchChange = null;
  }

  function findRef(
    predicate: (ref: RefType) => boolean,
  ): { rowId: string; column: ColumnType; ref: RefType } | undefined {
    for (const row of rows.value) {
      for (const column of columnsInOrder.value) {
        const ref = getRef(row.id, column);
        if (ref != undefined && predicate(ref)) {
          return { rowId: row.id, column, ref };
        }
      }
    }
    return undefined;
  }

  function getRef(rowId: string, column: ColumnType): RefType {
    const columnId = rowId + "." + column;
    return columnRefs.value[columnId];
  }

  function getColumn(rowId: string): RefType[] {
    return Object.entries(columnRefs.value)
      .filter(([key]) => key.startsWith(rowId))
      .map(([, value]) => value);
  }

  function blur() {
    Object.values(columnRefs.value).forEach((ref) => (ref as unknown as { blur?: () => void })?.blur?.());
  }

  function focus(index: number | string, column: ColumnType) {
    let row;
    if (typeof index == "string") {
      row = rows.value?.find((m) => m.id == index);
    } else {
      if (index < 0) {
        // if index is negative, start from the last row
        index = rows.value?.length + index;
      }
      row = rows.value?.[index];
    }

    if (!row) {
      log.warn("no row found for index", index);
      return;
    }
    const columnId = row.id + "." + column;
    (columnRefs.value?.[columnId] as { focus?: () => void })?.focus?.();
    options?.onFocus?.(row.id, column);
  }

  function navigateUp(rowId: string, column: ColumnType) {
    const rowIdx = rows.value?.findIndex((m) => m.id === rowId);
    if (!rowIdx) {
      options?.gridNavigateUp?.(rowIdx, column, columnsInOrder.value.indexOf(column));
    } else {
      focus(rowIdx - 1, column);
    }
  }

  function navigateDown(rowId: string, column: ColumnType) {
    const rowIdx = rows.value?.findIndex((m) => m.id === rowId) ?? 0;
    if (rowIdx == rowsLength.value - 1) {
      options?.gridNavigateDown?.(rowIdx, column, columnsInOrder.value.indexOf(column));
    } else {
      focus(rowIdx + 1, column);
    }
  }

  function navigateRight(rowId: string, column: ColumnType) {
    const rowIdx = rows.value?.findIndex((m) => m.id === rowId) ?? -1;
    const columnIdx = columnsInOrder.value.findIndex((f) => f === column);
    if (columnIdx == columnsInOrder.value.length - 1) {
      if (rowIdx != rowsLength.value - 1 && !options?.nowrapRight) {
        focus(rowIdx + 1, columnsInOrder.value[0]);
      } else {
        options?.gridNavigateRight?.(rowIdx, column, columnsInOrder.value.indexOf(column));
      }
    } else {
      focus(rowIdx, columnsInOrder.value[columnIdx + 1]);
    }
  }

  function navigateLeft(rowId: string, column: ColumnType) {
    const rowIdx = rows.value?.findIndex((m) => m.id === rowId) ?? -1;
    const columnIdx = columnsInOrder.value.findIndex((f) => f === column);
    if (columnIdx == 0) {
      if (rowIdx != 0 && !options?.nowrapLeft) {
        focus(rowIdx - 1, columnsInOrder.value[columnsInOrder.value.length - 1]);
      } else {
        options?.gridNavigateLeft?.(rowIdx, column, columnsInOrder.value.indexOf(column));
      }
    } else {
      focus(rowIdx, columnsInOrder.value[columnIdx - 1]);
    }
  }

  return {
    registerColumnRef,
    onColumnAvailable,
    beginBatchChange,
    flush,
    refsByColumn: columnRefs,
    refs: computed(() => Object.values(columnRefs.value)),
    findRef,
    getRef,
    getColumn,
    focus,
    blur,
    navigateUp,
    navigateDown,
    navigateRight,
    navigateLeft,
  };
}
