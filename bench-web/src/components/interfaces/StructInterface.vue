<script lang="ts" setup>
import type { Field, ResolvedField, Statement } from "@/state/module";
import FieldInterface from "@/components/interfaces/FieldInterface.vue";
import ValueInterface from "@/components/interfaces/ValueInterface.vue";
import { useNavigationGrid } from "@/composables/useGrid";
import { onStartTyping } from "@vueuse/core";
import { computed, nextTick, ref, toRef, type Ref } from "vue";
import { TypeFlag, useCurrentModule } from "@/state/module";
import { TypeTag } from "@/gql/graphql";
import { ChevronRightIcon, QueueListIcon, TableCellsIcon } from "@heroicons/vue/24/outline";

type StructAppearance = {
  verticalBorders?: boolean;
  minRowHeight?: number;
  maxRowHeight?: number;
  rowPadding?: number;
  hideFieldType?: boolean;
  minimalFields?: boolean;
  view?: "tree" | "grid";
};

const DEFAULT_APPEARANCE = {
  verticalBorders: true,
  minRowHeight: 32, // incl. padding
  maxRowHeight: 220,
  rowPadding: 4,
  hideFieldType: true,
  minimalFields: false,
  view: "grid",
};

const props = defineProps<{
  type: Field | Statement | Field[];
  isOutput?: boolean;
  modelValue: Record<string, any>;
  readonly?: boolean;
  readonlyType?: boolean;
  active?: boolean;
  debounced?: boolean;
  fullInputs?: boolean;
  showControls?: boolean;
  appearance?: StructAppearance;
}>();

const emit = defineEmits<{
  (e: "update:modelValue", value: Record<string, any>): void;
  (e: "update:field", value: Field): void;
  (e: "duplicate:field", value: Field): void;
  (e: "delete:field", value: Field): void;
  (e: "navigateUp"): void;
  (e: "navigateDown"): void;
  (e: "deleteSelf"): void;
}>();

const appearance = computed(() => ({ ...DEFAULT_APPEARANCE, ...props.appearance }));
const module = useCurrentModule();

function getFields(node: Field | Statement, isOutput?: boolean): Field[] {
  const statement = node.__typename == "Statement" ? node : module.statementOf((node as Field).referenceCk);
  return (
    statement?.resolvedFields
      ?.map((f) => f as ResolvedField)
      .map((f) => (f?.fieldCk == null ? null : module.fieldOf(f.fieldCk)))
      .filter(
        (f) =>
          f != null &&
          f.deletedAt == null &&
          !(f.flags & TypeFlag.IS_CONFIG) &&
          (isOutput === undefined || Boolean(f.flags & TypeFlag.IS_OUTPUT) === isOutput)
      )
      .map((f) => f as Field) ?? []
  );
}

const fields = computed(() => {
  if (Array.isArray(props.type)) {
    return props.type;
  }
  return getFields(props.type, props.isOutput);
});
const display: Ref<"tree" | "grid"> = ref((appearance.value.view as "tree" | "grid") ?? "grid");

function readField(field: Field) {
  return props.modelValue[module.getTypedKey(field) as string];
}

function writeField(field: Field, value: any) {
  const key = module.getTypedKey(field);
  if (key == null) return;
  emit("update:modelValue", { ...props.modelValue, [key]: value });
}

function deleteField(key: string) {
  const copy = { ...props.modelValue };
  delete copy[key];
  emit("update:modelValue", copy);
}

// grid

const grid = useNavigationGrid<"type" | "value", InstanceType<typeof ValueInterface | typeof FieldInterface>>(
  ref(["type", "value"]),
  fields,
  {
    gridNavigateUp: () => emit("navigateUp"),
    gridNavigateDown: () => emit("navigateDown"),
  }
);

// auto clear and start editing when typing while not editing
onStartTyping((e) => {
  if (props.readonly) return;
  const cell = grid.findRef((r) => r.$el.parentNode.contains(e.target));
  if (cell != null && cell.rowId != "type") {
    const field = fields.value.find((f) => f.key == cell.column);
    if (field == null) return;
    deleteField(module.getTypedKey(field) as string);
    nextTick(() => (cell.ref as InstanceType<typeof ValueInterface>).edit?.());
  }
});

const rowHeights = computed(() =>
  fields.value.map((field) =>
    Math.max(
      appearance.value.minRowHeight,
      Math.min(appearance.value.maxRowHeight, grid.getRef(field.id, "value")?.previewSize.height.value ?? 0)
    )
  )
);

// tree (inline, linearized)
type Node = {
  id: string;
  name?: string;
  depth: number;
  children?: Node[];
  expanded?: boolean;
  value: any;
  field?: Field;
  index?: number;
};
const expandedNodeIds: Ref<string[] | null> = ref(null);
const nodes: Ref<Node[]> = computed(() => {
  const nodes: Node[] = [];
  if (display.value != "tree") {
    return nodes;
  }

  function walk(
    value: any,
    field: Field,
    depth: number,
    options?: { parentId?: string; index?: number; ignore?: boolean }
  ): Node {
    if (field.tag == TypeTag.TypeReference) {
      field = module.effectiveTypeOf(field);
    }
    const isArray =
      options?.index == undefined &&
      (field.flags & TypeFlag.IS_ARRAY || (field.flags & TypeFlag.IS_ARRAYABLE && Array.isArray(value)));
    const hasChildren = Boolean(isArray) || field.tag == TypeTag.Struct;
    let id = options?.index != null ? `${field.key}:${options?.index}` : field.key;
    if (options?.parentId != null) {
      id = `${options?.parentId}.${id}`;
    }
    const node: Node = {
      id,
      name: options?.index != null ? `${options?.index}` : field.name ?? undefined,
      depth,
      value,
      field,
      index: options?.index,
      expanded: hasChildren && (expandedNodeIds.value == null || expandedNodeIds.value.includes(id)),
    };
    if (options?.ignore) {
      return node;
    }

    nodes.push(node);
    // walk children (array elements or struct fields)
    let children: Node[] | undefined = undefined;
    if (isArray && Array.isArray(value)) {
      // walk array
      children = value.map((v, i) =>
        walk(v, field, depth + 1, { parentId: node.id, index: i, ignore: !node.expanded })
      );
      node.name = node.name + " (" + value.length + ")";
    } else if (field.tag == TypeTag.Struct && value != null) {
      // walk struct field
      const childFields = getFields(field);
      children = childFields.map((f) =>
        walk(value[module.getTypedKey(f) ?? ""], f, depth + 1, { parentId: node.id, ignore: !node.expanded })
      );
    }

    node.children = children;
    return node;
  }
  // walk root fields
  for (const field of fields.value) {
    walk(props.modelValue[module.getTypedKey(field) ?? ""], field, 0);
  }
  // auto expand everything up to a certain depth on first tree render

  return nodes;
});
function toggleExpanded(node: Node) {
  if (node.children == null) return;
  if (expandedNodeIds.value == null) {
    // init to all expanded
    expandedNodeIds.value = nodes.value.filter((n) => n.children != null).map((n) => n.id);
  }
  if (node.expanded) {
    expandedNodeIds.value = expandedNodeIds.value.filter((id) => id != node.id);
  } else {
    expandedNodeIds.value = [...expandedNodeIds.value, node.id];
  }
}

defineExpose({
  focus: (position: "first" | "last" | string = "first") => {
    if (position == "first") {
      grid.focus(0, "type");
    } else if (position == "last") {
      grid.focus(-1, "type");
    } else {
      grid.focus(position, "type");
    }
  },
  openField(fieldId: string) {
    (grid.getRef(fieldId, "type") as InstanceType<typeof FieldInterface>)?.open("all");
  },
  blur: () => {
    grid.refs.value.forEach((r) => r.blur?.());
  },
});
</script>
<template>
  <!-- Wrapper for controls -->
  <div class="relative w-full">
    <!-- Grid view -->
    <table v-if="display == 'grid'" ref="gridRef" class="w-full table-fixed">
      <tr v-for="(field, y) in fields" :key="field.id">
        <td
          v-if="!appearance.minimalFields"
          class="w-1/3 self-start p-0"
          :class="[appearance.verticalBorders && y > 0 ? 'border-t border-red-900/[12%]' : '']"
        >
          <FieldInterface
            :ref="(el: any) => grid.registerColumnRef(field?.id, 'type', el)"
            :type="field"
            :readonly="(readonly ?? false) || (readonlyType ?? false)"
            orientation="vertical"
            class="w-full self-start border-r border-amber-900/[12%] px-1.5 py-1 text-gray-700 focus-within:border-solid focus-within:bg-amber-200 hover:bg-amber-200"
            hide-text
            :ref-types="[TypeTag.Enum, TypeTag.Struct]"
            :model-value="field"
            @update:model-value="emit('update:field', { ...$event, id: field.id, key: field.key } as Field)"
            @delete-self="emit('delete:field', field)"
            @duplicate-self="emit('duplicate:field', field)"
            @navigate-up="grid.navigateUp(field?.id, 'type')"
            @navigate-down="grid.navigateDown(field?.id, 'type')"
            @navigate-right="grid.navigateRight(field?.id, 'type')"
            @navigate-left="grid.navigateLeft(field?.id, 'type')"
            @enter="grid.navigateDown(field?.id, 'type')"
            :style="{
              minHeight: appearance.minRowHeight + 'px',
              height: rowHeights[y] + 'px',
            }"
          />
        </td>
        <td class="w-full p-0" :class="[appearance.verticalBorders && y > 0 ? 'border-t border-orange-900/[12%]' : '']">
          <!-- should probably separate the 'minimal fields' out, but not sure what becomes of that yet -->
          <div class="w-full select-none px-1" v-if="appearance.minimalFields">
            <span class="font-semibold text-gray-500">{{ field.name }}</span>
          </div>
          <ValueInterface
            :ref="(el: any) => grid.registerColumnRef(field.id, 'value', el)"
            :model-value="readField(field)"
            @update:model-value="(val) => writeField(field, val)"
            :type="module.effectiveTypeOf(field)"
            :readonly="readonly ?? false"
            :active="active ?? false"
            :debounced="debounced"
            :supports-drop="false"
            :full="fullInputs ?? false"
            wrap
            @delete-self="deleteField(module.getTypedKey(field) as string)"
            @navigate-up="grid.navigateUp(field.id, 'value')"
            @navigate-down="grid.navigateDown(field.id, 'value')"
            @navigate-right="grid.navigateRight(field.id, 'value')"
            @navigate-left="grid.navigateLeft(field.id, 'value')"
            class="scroll-hidden h-full w-full max-w-full self-start overflow-auto border border-transparent p-1 focus-within:border-solid focus-within:border-orange-900 focus-within:border-opacity-[15%] focus-within:bg-orange-100 hover:bg-orange-100"
            :style="{ 'max-height': appearance.maxRowHeight + 'px' }"
          />
        </td>
      </tr>
    </table>
    <!-- Tree view -->
    <div v-else class="flex flex-col">
      <div
        v-for="node in nodes"
        :key="node.id"
        class="flex min-h-[24px] flex-row rounded-sm px-1 py-0.5 hover:bg-orange-100"
        :class="[node.children != null ? 'hover:cursor-pointer' : '']"
        :style="{ marginLeft: node.depth * 18 + 'px' }"
        @click="toggleExpanded(node)"
      >
        <ChevronRightIcon
          v-if="node.children"
          class="mr-0.5 mt-0.5 h-4 w-4 flex-shrink-0 text-gray-500 transition-transform duration-150"
          :class="[node.expanded ? 'rotate-90' : '']"
        />
        <span class="flex-shrink-0 select-none whitespace-nowrap font-semibold text-gray-500">{{ node.name }}:</span>
        <ValueInterface
          v-if="node.field != null && !node.expanded"
          :model-value="node.value"
          readonly
          :type="node.field"
          active
          :wrap="node.children == null || !node.expanded"
          class="scroll-hidden ml-1.5 max-w-full self-start overflow-auto"
          :style="{ 'max-height': appearance.maxRowHeight + 'px' }"
        />
      </div>
    </div>
    <!-- Controls -->
    <div v-if="showControls && fields.length > 0" class="absolute right-0.5 top-0.5 flex flex-row gap-1 bg-white p-0.5">
      <!-- Toggle view -->
      <button
        @click.stop="display = display == 'tree' ? 'grid' : 'tree'"
        class="rounded-sm p-0.5 text-gray-400 hover:bg-orange-100 hover:text-gray-700"
      >
        <component :is="display == 'grid' ? TableCellsIcon : QueueListIcon" class="h-4 w-4" />
      </button>
    </div>
  </div>
</template>
