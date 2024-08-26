<script lang="ts" setup>
import {
  BenchType,
  BoxData,
  NodeReferenceData,
  NodeType,
  ObjectType,
  Orientation,
  TypeKind,
  Variant,
  ViewData,
  ViewType,
  type AnyNodeData,
} from "@/proto/wire";
import { type TypedNodeReferenceData } from "@/proto/wiring";
import { ICON_BY_BENCH_TYPE, ICON_BY_BLOCK_TYPE, IconInline, makeIcon } from "@/ui/icon";
import { isEnumType, isNodeType } from "@/language/const";
import type { NodeItem, TypeItem } from "@/ui/search";
import { enumIndex, graphIndex, typeIndex, useSearch, type EnumOptionItem, type SearchIndex } from "@/ui/search";
import { canvas, pkgGraph } from "@/system/space";
import { ScrollbarWidth } from "@/ui/layout";
import type { PopoverInfoIn } from "@/ui/popover";
import {
  makeViewId,
  ViewContentWrapper,
  viewEmits,
  type FocusAnchor,
  type ViewExposed,
  type ViewProps,
} from "@/views/common";
import Scroll from "@/views/containers/Scroll.vue";
import { computed, ref, toRef, watch, type Ref } from "vue";
import { getConstrainedTypeName, nodeMatchesConstraint } from "@/language/field";

const MIN_WIDTH = 200;
const DEFAULT_WIDTH = 280;
const MAX_HEIGHT = 360;

const props = defineProps<
  {
    self?: TypedNodeReferenceData<NodeType.VIEW>;
    modelValue?: any;
    size?: Partial<Pick<BoxData, "width" | "height">>;
    placeholder?: string;
    customIndex?: SearchIndex<any>;
  } & Partial<
    Pick<
      ViewData,
      | "name"
      | "title"
      | "text"
      | "icon"
      | "valuePacked"
      | "valueType"
      | "variant"
      | "isInput"
      | "isInline"
      | "isDisabled"
    >
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = makeViewId(props);

const buttonRef: Ref<HTMLButtonElement | null> = ref(null);
const query: Ref<string> = ref("");
const queryRef: Ref<HTMLInputElement | null> = ref(null);
const activeResultId: Ref<string | null> = ref(null);
const width = computed(() => Math.max(MIN_WIDTH, props.size?.width ?? DEFAULT_WIDTH));

const baseType = pkgGraph.getRef(
  computed(() => props.valueType?.baseTypePtr as TypedNodeReferenceData<NodeType.BLOCK> | undefined),
);
const facetIcon = computed(() => {
  if (props.valueType?.kind == TypeKind.BASED_NODE && baseType.value != null) {
    return ICON_BY_BLOCK_TYPE[baseType.value.type];
  } else if (props.valueType?.benchType != null) {
    return ICON_BY_BENCH_TYPE[props.valueType.benchType];
  } else {
    return null;
  }
});
const facetName = computed(() => {
  if (props.valueType?.kind == TypeKind.BASED_NODE && baseType.value != null) {
    return baseType.value.name;
  } else if (props.valueType?.benchType != null) {
    return getConstrainedTypeName(props.valueType);
  } else {
    return null;
  }
});
// NOTE: technically modelValueTitle/Icon aren't fully reactive (requires modelValue to change)
const modelValueTitle = computed(() =>
  props.modelValue != null ? index.value.fromValue(props.modelValue)?.title : null,
);
const modelValueIcon = computed(() =>
  props.modelValue != null ? index.value.fromValue(props.modelValue)?.icon : null,
);

type PickerItem = EnumOptionItem | NodeItem | TypeItem;
const index: Ref<SearchIndex<any>> = computed(() => {
  if (props.customIndex != null) {
    return props.customIndex;
  } else if (isEnumType(props.valueType?.benchType)) {
    return enumIndex({ id: "enum", enumTypes: [props.valueType.benchType] });
  } else if (isNodeType(props.valueType?.benchType)) {
    let roots: AnyNodeData[] | undefined = undefined;
    if (props.valueType.baseTypePtr != null) {
      // based node
      const base = pkgGraph.get(props.valueType.baseTypePtr);
      if (base != null) roots = [base];
    }
    return graphIndex({
      id: "graph",
      graph: pkgGraph,
      metatypes: [props.valueType.benchType],
      roots,
      skipDepth: roots != null ? 0 : 2,
      filter:
        props.valueType?.constraint != null
          ? (node) => nodeMatchesConstraint(node, props.valueType!.constraint!)
          : undefined,
    });
  } else if (props.valueType?.benchType == BenchType.TYPE_INFO) {
    return typeIndex({ id: "type", graph: pkgGraph, skipDepth: 2 });
  } else {
    throw new Error(`unsupported value type: ${props.valueType?.benchType}`);
  }
});
const resultsRefs: Ref<Record<string, HTMLElement | null>> = ref({});
const { results, resultsTotal } = useSearch<PickerItem>({
  query,
  indices: computed(() => ({ main: index.value })),
  isEnabled: computed(() => props.isInline),
});

// auto-select best match when searching
watch(results, () => {
  if (results.value.length > 0) {
    activeResultId.value = results.value[0].id;
  }
});

function isSelected(value: PickerItem) {
  return props.modelValue != null && index.value.valueEquals(value, props.modelValue);
}
function isActive(item: PickerItem) {
  return item.id === activeResultId.value;
}
function fire(option: string | PickerItem | undefined) {
  if (typeof option == "string") option = results.value.find((r) => r.id === option);
  if (option == null) return;
  const value = index.value.toValue(option);
  apply(value);
}
function apply(value: any) {
  emit("update:modelValue", value ?? undefined);
  emit("apply", value ?? undefined);
}

function focus(anchor?: "previous" | "next" | FocusAnchor | NodeReferenceData) {
  if (!props.isInline) {
    return buttonRef.value;
  } else {
    const idx = results.value.findIndex((r) => r.id === activeResultId.value);
    if (anchor == "previous") {
      activeResultId.value = results.value[idx > 0 ? idx - 1 : results.value.length - 1].id;
    } else if (anchor == "next") {
      activeResultId.value = results.value[idx < results.value.length - 1 ? idx + 1 : 0].id;
    }
    if (activeResultId.value != null) {
      resultsRefs.value[activeResultId.value]?.scrollIntoView({ block: "center", behavior: "instant" });
    }
    return queryRef.value;
  }
}

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id, variants: [Variant.PRIMARY, Variant.COMPACT, Variant.STEALTH], focus });
</script>
<template>
  <ViewContentWrapper v-bind="props">
    <!-- TODO :Incomplete: Picker.isDisabled/... -->
    <!-- Dropdown -->
    <!-- NOTE: dropdown button style should match inline combobox header style since we overlay them -->
    <button
      v-if="!isInline && variant != Variant.COMPACT"
      ref="buttonRef"
      v-menu="
        (): PopoverInfoIn => ({
          component: ViewType.PICKER,
          placement: 'inside-top-left',
          referenceMargin: 0,
          props: {
            ...(props as ViewProps),
            title: undefined, // clear title
            size: { metatype: ObjectType.BOX, width: buttonRef?.getBoundingClientRect().width },
            isInline: true,
          },
          onApply: (value: any) => apply(value),
        })
      "
      :disabled="props.isDisabled || !props.isInput"
      class="group flex w-full flex-row items-center rounded border border-gray-200 bg-white px-2.5 py-1 hover:border-gray-300 data-[popover=true]:border-gray-300"
    >
      <!-- Current value -->
      <template v-if="modelValue != null">
        <IconInline v-if="modelValueIcon" v-bind="modelValueIcon" class="mr-1.5 w-5 text-gray-700" />
        <span class="truncate">{{ modelValueTitle ?? "???" }}</span>
      </template>
      <template v-else>
        <IconInline v-if="facetIcon" v-bind="facetIcon" class="mr-1.5 w-5 text-gray-400" />
        <span class="truncate text-gray-400">{{ facetName ?? "Select" }}</span>
      </template>
      <!-- Controls -->
      <div v-if="!props.isDisabled && props.isInput" class="ml-auto flex-shrink-0 pl-1.5">
        <!-- Clear -->
        <button
          v-if="modelValue != null && !valueType?.isRequired"
          class="mr-2 text-gray-400 opacity-0 hover:text-primary-900 group-hover:opacity-100"
          @click.stop="emit('update:modelValue', undefined)"
        >
          <i class="fas fa-xmark" />
        </button>
        <i class="fas fa-caret-down ml-auto text-gray-400 hover:text-primary-900" />
      </div>
    </button>

    <!-- Inline Multi-Toggle -->
    <div
      v-else-if="variant == Variant.COMPACT || variant == Variant.STEALTH"
      class="flex h-7 w-full flex-row items-center justify-between gap-x-2 truncate rounded bg-gray-100 px-2"
    >
      <!-- Inline choice -->
      <button
        v-for="item in results"
        :key="item.id"
        v-tooltip="{ icon: item.icon, title: item.title, small: true, group: 'picker' }"
        :data-selected="isSelected(item)"
        :disabled="props.isDisabled"
        class="group flex-1 flex-shrink-0 truncate rounded px-0.5 text-center font-medium shadow-gray-200 hover:text-primary-900 enabled:text-gray-600 disabled:text-gray-400 data-[selected=true]:bg-white data-[selected=true]:text-gray-700 data-[selected=true]:shadow-sm"
        @click.prevent="!isSelected(item) || valueType?.isRequired ? fire(item) : apply(undefined)"
      >
        <IconInline
          v-if="variant == Variant.STEALTH && item.icon"
          v-bind="item.icon"
          class="w-5 group-hover:text-primary-900"
        />
        <template v-else>{{ item.title }}</template>
      </button>
      <div v-if="results.length == 0" class="mx-auto">
        <i class="fas fa-empty-set mr-1.5 text-gray-600" />
        <span class="text-gray-500">No options</span>
      </div>
    </div>

    <!-- Inline Combobox -->
    <div v-else-if="isInline" :style="{ width: width + 'px' }">
      <!-- Header -->
      <div class="flex h-[30px] w-full flex-row items-center border-b border-gray-200 px-3 py-1">
        <IconInline
          v-bind="modelValueIcon ?? icon ?? makeIcon({ faName: 'fas fa-caret-circle-down' })"
          class="mr-2 w-5 text-gray-700"
        />
        <!-- Query -->
        <input
          ref="queryRef"
          v-model="query"
          type="text"
          class="w-full border-0 bg-transparent p-0 placeholder-gray-500 outline-none ring-0 focus:ring-0"
          :placeholder="placeholder ?? modelValueTitle ?? `Select ${facetName ?? '???'}`"
          @keydown.enter.stop.prevent="activeResultId && fire(activeResultId)"
          @keydown.up.stop.prevent="focus('previous')"
          @keydown.down.stop.prevent="focus('next')"
        />
      </div>
      <!-- Body -->
      <Scroll
        size-is-dynamic
        :size="{ width, height: MAX_HEIGHT }"
        :orientation="Orientation.VERTICAL"
        :track-width="ScrollbarWidth.sm"
        track-is-overlay
      >
        <!-- Results -->
        <ul v-if="results.length > 0" class="flex flex-col py-0.5">
          <template v-for="item in results" :key="item.id">
            <!-- Results -->
            <li
              :ref="(ref?: any) => (ref != null ? (resultsRefs[item.id] = ref) : delete resultsRefs[item.id])"
              role="menuitem"
              class="mx-0.5 mb-[1px] mr-1.5 mt-[1px] flex h-[28px] max-w-full cursor-pointer flex-row items-center rounded border border-transparent px-2 hover:bg-gray-100 data-[active=true]:border-primary-900"
              :data-selected="isSelected(item)"
              :data-active="isActive(item)"
              @click.prevent="fire(item)"
            >
              <!-- Content -->
              <IconInline
                v-if="item.icon"
                v-bind="item.icon"
                class="mr-1.5 w-5 flex-shrink-0"
                :class="isActive(item) ? 'text-primary-900' : 'text-gray-700'"
              />
              <span v-else class="mr-1.5 w-5 flex-shrink-0 text-gray-700" />
              <span
                class="select-none truncate"
                :class="isActive(item) ? 'text-primary-900' : ''"
                v-html="item.titleMarked ?? item.title"
              />
              <!-- Metadata -->
              <span class="ml-auto truncate pl-2">
                <!-- Checked -->
                <i v-if="isSelected(item)" class="fas fa-check flex-shrink-0 pl-2 pr-1 text-gray-700" />
                <!-- Path -->
                <span
                  v-if="valueType?.kind != TypeKind.BASED_NODE && 'path' in item"
                  class="truncate pl-2 text-gray-500"
                >
                  <span v-html="item.pathMarked ?? item.path" />
                </span>
              </span>
            </li>
          </template>
        </ul>
        <!-- NOTE: Picker no results/overflow is very similar to Omnibar/Icon/etc. :ResultInfo -->
        <!-- Too many results (truncated) -->
        <div v-if="results.length < resultsTotal" class="my-1 max-w-full px-[11px] pb-2 text-gray-500">
          <i class="fas fas fa-ellipsis w-5 text-center" />
          <span class="ml-1.5">
            <span class="font-semibold">{{ resultsTotal - results.length }}</span> more results
            <template v-if="query.length > 0">for </template>
            <span class="truncate font-semibold">{{ query }}</span>
          </span>
        </div>
        <!-- Help -->
        <div v-if="results.length == 0" class="max-w-full py-1">
          <!-- Nothing found -->
          <div v-if="results.length === 0" class="px-[11px] py-1 text-gray-500">
            <i class="fas fa-empty-set w-5 text-center text-gray-600" />
            <span class="ml-1"> No results </span>
          </div>
        </div>
      </Scroll>
    </div>
  </ViewContentWrapper>
</template>
