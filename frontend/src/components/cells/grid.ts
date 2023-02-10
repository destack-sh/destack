import { computed, ref, type Ref } from "vue";

export function useNavigationGrid<ColumnType = string>(
  columnsInOrder: Ref<ColumnType[]>,
  rows: Ref<{ id: string }[]>,
  gridNavigateUp: () => void = () => ({}),
  gridNavigateDown: () => void = () => ({})
) {
  const columnRefs: Ref<Record<string, HTMLInputElement>> = ref({});
  const rowsLength = computed(() => rows.value?.length ?? 0);

  function registerColumnRef(rowId: string, column: ColumnType, ref: HTMLInputElement | undefined) {
    const columnId = rowId + "." + column;
    if (ref != undefined) {
      columnRefs.value[columnId] = ref;
    } else {
      delete columnRefs.value[columnId];
    }
  }

  function blur() {
    Object.values(columnRefs.value).forEach((ref) => ref.blur());
  }

  function focus(index: number | string, column: ColumnType) {
    let row;
    if (typeof index == "string") {
      row = rows.value?.find((m) => m.id == index);
    } else {
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
      gridNavigateUp();
    } else {
      focus(rowIdx - 1, column);
    }
  }

  function navigateDown(rowId: string, column: ColumnType) {
    const rowIdx = rows.value?.findIndex((m) => m.id === rowId) ?? 0;
    if (rowIdx == rowsLength.value - 1) {
      gridNavigateDown();
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
      }
    } else {
      focus(rowIdx, columnsInOrder.value[columnIdx - 1]);
    }
  }

  return {
    registerColumnRef,
    focus,
    blur,
    navigateUp,
    navigateDown,
    navigateRight,
    navigateLeft,
  };
}
