import { computed, ref, type Ref } from "vue";

export function useElementRefs<RefType = HTMLInputElement>() {
  const refs: Ref<Record<string, RefType>> = ref({});

  function registerRef(id: string, ref: RefType | undefined) {
    if (ref != undefined) {
      refs.value[id] = ref;
    } else {
      delete refs.value[id];
    }
  }

  function getRef(id: string): RefType {
    return refs.value[id];
  }

  function focus(id: string) {
    refs.value[id]?.focus();
  }

  return {
    registerRef,
    refs: computed(() => Object.values(refs.value)),
    getRef,
    focus,
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
    onFocus?: (rowId: string, column: ColumnType) => void;
  } = {}
) {
  const columnRefs: Ref<Record<string, RefType>> = ref({});
  const rowsLength = computed(() => rows.value?.length ?? 0);

  function registerColumnRef(rowId: string, column: ColumnType, ref: RefType | undefined) {
    const columnId = rowId + "." + column;
    if (ref != undefined) {
      columnRefs.value[columnId] = ref;
    } else {
      delete columnRefs.value[columnId];
    }
  }

  function getRef(rowId: string, column: ColumnType): RefType {
    const columnId = rowId + "." + column;
    return columnRefs.value[columnId];
  }

  function blur() {
    Object.values(columnRefs.value).forEach((ref) => ref.blur());
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
      console.warn("no row found for index", index);
      return;
    }
    const columnId = row.id + "." + column;
    columnRefs.value?.[columnId]?.focus();
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
    refs: computed(() => Object.values(columnRefs.value)),
    getRef,
    focus,
    blur,
    navigateUp,
    navigateDown,
    navigateRight,
    navigateLeft,
  };
}
