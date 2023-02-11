import { computed, ref, type Ref } from "vue";

export function useNavigationGrid<ColumnType = string, RefType = HTMLInputElement>(
  columnsInOrder: Ref<ColumnType[]>,
  rows: Ref<{ id: string }[]>,
  options: {
    gridNavigateUp?: (column: ColumnType, columnIndex: number) => void;
    gridNavigateDown?: (column: ColumnType, columnIndex: number) => void;
    gridNavigateLeft?: () => void;
    gridNavigateRight?: () => void;
  }
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
  }

  function navigateUp(rowId: string, column: ColumnType) {
    const rowIdx = rows.value?.findIndex((m) => m.id === rowId);
    if (!rowIdx) {
      options?.gridNavigateUp?.(column, columnsInOrder.value.indexOf(column));
    } else {
      focus(rowIdx - 1, column);
    }
  }

  function navigateDown(rowId: string, column: ColumnType) {
    const rowIdx = rows.value?.findIndex((m) => m.id === rowId) ?? 0;
    if (rowIdx == rowsLength.value - 1) {
      options?.gridNavigateDown?.(column, columnsInOrder.value.indexOf(column));
    } else {
      focus(rowIdx + 1, column);
    }
  }

  function navigateRight(rowId: string, column: ColumnType) {
    const rowIdx = rows.value?.findIndex((m) => m.id === rowId) ?? -1;
    const columnIdx = columnsInOrder.value.findIndex((f) => f === column);
    if (columnIdx == columnsInOrder.value.length - 1) {
      if (rowIdx != rowsLength.value - 1) {
        focus(rowIdx + 1, columnsInOrder.value[0]);
      } else {
        options?.gridNavigateRight?.();
      }
    } else {
      focus(rowIdx, columnsInOrder.value[columnIdx + 1]);
    }
  }

  function navigateLeft(rowId: string, column: ColumnType) {
    const rowIdx = rows.value?.findIndex((m) => m.id === rowId) ?? -1;
    const columnIdx = columnsInOrder.value.findIndex((f) => f === column);
    if (columnIdx == 0) {
      if (rowIdx != 0) {
        focus(rowIdx - 1, columnsInOrder.value[columnsInOrder.value.length - 1]);
      } else {
        options?.gridNavigateLeft?.();
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
