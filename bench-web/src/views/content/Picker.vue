<script lang="ts" setup>
import { isEnumType, isNodeType, toCamelName } from "@/language/const";
import { getConstrainedTypeName, nodeMatchesConstraint } from "@/language/field";
import {
  BenchType,
  IconData,
  NodeReferenceData,
  NodeType,
  ObjectType,
  Orientation,
  RectangleData,
  TypeKind,
  Variant,
  ViewData,
  ViewType,
  type AnyNodeData,
} from "@/proto/wire";
import { type TypedNodeReferenceData } from "@/proto/wiring";
import { supergraph } from "@/system/connection";
import { canvas, pkgGraph } from "@/system/space";
import { ICON_BY_BENCH_TYPE, ICON_BY_BLOCK_TYPE, ICON_BY_TYPE_KIND, IconInline, makeIcon } from "@/ui/icon";
import { ScrollbarWidth } from "@/ui/layout";
import type { PopoverInfoIn } from "@/ui/popover";
import type { NodeItem, SearchItem, TypeItem } from "@/ui/search";
import { enumIndex, graphIndex, typeIndex, useSearch, type EnumOptionItem, type SearchIndex } from "@/ui/search";
import { ViewContentWrapper, viewEmits, type FocusAnchor, type ViewExposed, type ViewProps } from "@/views/common";
import Scroll from "@/views/containers/Scroll.vue";
import { computed, ref, toRef, watch, type Ref } from "vue";

const MIN_WIDTH = 320;
const DEFAULT_WIDTH = 400;
const MAX_HEIGHT = 360;

const props = defineProps<
  {
    self?: TypedNodeReferenceData<NodeType.VIEW>;
    id: string;
    modelValue?: any;
    size?: Partial<Pick<RectangleData, "width" | "height">>;
    placeholder?: string;
    customIndex?: SearchIndex<any>;
    isPopover?: boolean;
  } & Partial<
    Pick<ViewData, "name" | "title" | "icon" | "valueType" | "variant" | "isInput" | "isInline" | "isDisabled">
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = toRef(props, "id");
const state = canvas.registerView(self, id);

const buttonRef: Ref<HTMLButtonElement | null> = ref(null);
const query: Ref<string> = ref("");
const queryRef: Ref<HTMLInputElement | null> = ref(null);
const activeResultId: Ref<string | null> = ref(null);
const width = computed(() => Math.max(MIN_WIDTH, props.size?.width ?? DEFAULT_WIDTH));

const baseType = supergraph.getRef(
  computed(() => props.valueType?.baseTypePtr as TypedNodeReferenceData<NodeType.BLOCK> | undefined),
);
const facetIcon = computed(() => {
  if (props.valueType?.kind == TypeKind.BASED_NODE && baseType.value != null) {
    return ICON_BY_BLOCK_TYPE[baseType.value.type];
  } else if (props.valueType?.benchType != null) {
    return ICON_BY_BENCH_TYPE[props.valueType.benchType];
  } else if (props.valueType?.kind != null) {
    return ICON_BY_TYPE_KIND[props.valueType.kind];
  } else {
    return null;
  }
});
const facetName = computed(() => {
  if (props.valueType?.kind == TypeKind.BASED_NODE && baseType.value != null) {
    return baseType.value.name;
  } else if (props.valueType?.benchType != null) {
    return getConstrainedTypeName(props.valueType);
  } else if (props.valueType?.kind == TypeKind.NODE) {
    return toCamelName(TypeKind, props.valueType.kind);
  } else {
    return null;
  }
});
// NOTE :Broken: technically modelValueVignettes/Icon aren't fully reactive (requires modelValue to change)
const hasValue = computed(() => {
  if (props.modelValue == null) return false;
  if (props.valueType?.isList) return (props.modelValue as any[]).length > 0;
  else return true;
});
const valueVignettes: Ref<{ title: string | undefined; icon: IconData | undefined }[]> = computed(() => {
  if (!hasValue.value) return [];
  if (!props.valueType?.isList) {
    const value = index.value.fromValue(props.modelValue) as SearchItem | null;
    return [{ title: value?.title, icon: (value as any)?.icon }];
  } else {
    const values = (props.modelValue as any[]).map((v) => index.value.fromValue(v) as SearchItem | null);
    return values.map((v) => ({ title: v?.title, icon: (v as any)?.icon }));
  }
});

type PickerItem = EnumOptionItem | NodeItem | TypeItem;
const index: Ref<SearchIndex<any>> = computed(() => {
  if (props.customIndex != null) {
    return props.customIndex;
  } else if (isEnumType(props.valueType?.benchType)) {
    return enumIndex({ id: "enum", enumTypes: [props.valueType.benchType] });
  } else if (props.valueType?.kind == TypeKind.NODE || isNodeType(props.valueType?.benchType)) {
    let roots: AnyNodeData[] | undefined = undefined;
    if (props.valueType.baseTypePtr != null) {
      // based node
      const base = pkgGraph.get(props.valueType.baseTypePtr);
      if (base != null) roots = [base];
    } else if ((props.valueType.constraint?.nodeScopePtr?.length ?? 0) > 0) {
      roots = props.valueType.constraint!.nodeScopePtr.map((r) => pkgGraph.get(r)).filter((r) => r != null);
    }
    let metatypes: NodeType[];
    if (props.valueType.benchType != null) {
      metatypes = [props.valueType.benchType as unknown as NodeType];
    } else if ((props.valueType.constraint?.nodeTypes?.length ?? 0) > 0) {
      metatypes = props.valueType.constraint!.nodeTypes;
    } else {
      metatypes = [NodeType.BLOCK, NodeType.STEP, NodeType.FIELD, NodeType.VIEW];
    }
    return graphIndex({
      id: "graph",
      graph: pkgGraph,
      metatypes,
      roots,
      skipDepth: roots != null ? 0 : 2,
      maxDepth: props.valueType?.constraint?.nodeMaxDepth,
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
  return hasValue.value && index.value.valueEquals(value, props.modelValue);
}
function isActive(item: PickerItem) {
  return item.id === activeResultId.value;
}
function select(option: string | PickerItem | undefined) {
  if (typeof option == "string") option = results.value.find((r) => r.id === option);
  if (option == null) return;
  const value = index.value.toValue(option);
  if (value != null) {
    if (!props.valueType?.isList) {
      apply(value);
    } else if (!isSelected(option)) {
      apply([...((props.modelValue as any[]) ?? []), value]);
    }
    query.value = "";
  }
}
function deselect(option: PickerItem | number) {
  if (!props.valueType?.isList) {
    apply(undefined);
  } else if (hasValue.value) {
    if (typeof option == "number") {
      apply((props.modelValue as any[]).filter((v, i) => i != option));
    } else {
      apply((props.modelValue as any[]).filter((v) => !index.value.valueEquals(v, option)));
    }
  }
}
function apply(value: any) {
  emit("update:modelValue", value ?? undefined);
  emit("apply", value ?? undefined, props.valueType?.isList);
}
function clear() {
  apply(undefined);
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

defineExpose<ViewExposed>({
  self,
  id,
  focus,
  interact: () => {
    if (!props.isInline) {
      buttonRef.value?.click();
    } else {
      queryRef.value?.focus();
    }
  },
});
</script>
<template>
  <ViewContentWrapper :type="ViewType.PICKER" v-bind="props">
    <!-- TODO :Incomplete: Picker.isDisabled/... -->
    <!-- Dropdown -->
    <!-- NOTE: dropdown button style should match inline combobox header style since we overlay them -->
    <div
      v-if="!isInline && variant != Variant.COMPACT"
      ref="buttonRef"
      v-menu="
        (): PopoverInfoIn => ({
          kind: 'view',
          component: ViewType.PICKER,
          placement: 'inside-top-left',
          isEnabled: !isDisabled && isInput,
          // stealth picker has no padding, but popover picker does, so add offset to ensure it's aligned
          offset: variant == Variant.STEALTH ? { x: -10, y: -5 } : undefined,
          referenceMargin: 0,
          dontAnimate: variant == Variant.STEALTH,
          props: {
            ...(props as ViewProps),
            variant: Variant.PRIMARY, // full dropdown
            title: undefined, // clear title
            size: {
              metatype: ObjectType.RECTANGLE,
              width: Math.max(
                MIN_WIDTH,
                buttonRef?.getBoundingClientRect().width! + (variant == Variant.STEALTH ? 10 : 0), // see above
              ),
            },
            isPopover: false, // want clean inline style so it matches this variant
            isInline: true,
          },
          onApply: (value: any) => apply(value),
        })
      "
      role="button"
      :disabled="props.isDisabled || !props.isInput"
      class="group flex w-full flex-row flex-wrap items-center gap-y-1 rounded border-gray-200 hover:border-gray-200 data-[popover=true]:border-gray-200"
      :class="[variant != Variant.STEALTH ? 'border px-2.5 py-1' : '']"
    >
      <!-- Current value -->
      <template v-if="hasValue">
        <button
          v-for="(v, i) in valueVignettes"
          :key="i"
          class="mr-2 flex flex-row items-center rounded"
          :class="valueType?.isList ? 'bg-gray-100 px-1' : ''"
        >
          <IconInline v-if="v.icon" v-bind="v.icon" class="mr-1.5 w-5 text-gray-700" />
          <span class="truncate">{{ v.title ?? "???" }}</span>
        </button>
      </template>
      <div
        v-else
        class="mr-2 transition-colors duration-150"
        :class="variant == Variant.STEALTH ? 'opacity-0 group-hover:opacity-100' : ''"
      >
        <IconInline v-if="facetIcon" v-bind="facetIcon" class="mr-1.5 w-5 text-gray-400" />
        <span class="truncate text-gray-400">{{ facetName ?? "Select" }}</span>
      </div>
      <!-- Add -->
      <button
        v-if="!isDisabled && isInput && valueType?.isList"
        class="mr-2 text-gray-400 opacity-0 hover:text-gray-700 group-hover:opacity-100"
      >
        <i class="fas fa-plus" />
      </button>
      <!-- Controls -->
      <div
        v-if="!isDisabled && isInput"
        class="ml-auto flex-shrink-0 pl-1.5 transition-colors duration-150"
        :class="variant == Variant.STEALTH ? 'opacity-0 group-hover:opacity-100' : ''"
      >
        <!-- Clear -->
        <button
          v-if="hasValue && !valueType?.isRequired"
          class="mr-2 text-gray-400 opacity-0 hover:text-gray-700 group-hover:opacity-100"
          @click.stop="clear"
        >
          <i class="fas fa-xmark" />
        </button>
        <i class="fas fa-caret-down ml-auto text-gray-400 hover:text-gray-700" />
      </div>
    </div>

    <!-- Inline Multi-Toggle -->
    <div
      v-else-if="variant == Variant.COMPACT || variant == Variant.STEALTH"
      class="flex h-7 w-full flex-row items-center justify-between gap-x-1 truncate rounded bg-gray-100 px-0.5"
    >
      <!-- Inline choice -->
      <button
        v-for="item in results"
        :key="item.id"
        v-tooltip="{ icon: item.icon, title: item.title, small: true, group: 'picker' }"
        :data-selected="isSelected(item)"
        :disabled="props.isDisabled"
        class="group flex-1 flex-shrink-0 truncate rounded px-0.5 py-0.5 text-center font-medium shadow-gray-200 hover:bg-gray-200 hover:text-gray-800 enabled:text-gray-600 disabled:text-gray-400 data-[selected=true]:bg-white data-[selected=true]:text-gray-700 data-[selected=true]:shadow-sm"
        @click.prevent="!isSelected(item) || valueType?.isRequired ? select(item) : clear()"
      >
        <IconInline v-if="variant == Variant.STEALTH && item.icon" v-bind="item.icon" class="w-5" />
        <span v-else class="truncate">{{ item.title }}</span>
      </button>
      <div v-if="results.length == 0" class="mx-auto">
        <i class="fas fa-empty-set mr-1.5 text-gray-600" />
        <span class="text-gray-500">No options</span>
      </div>
    </div>

    <!-- Inline Combobox -->
    <div v-else-if="isInline" :style="{ width: width + 'px' }">
      <!-- Header -->
      <div
        class="flex flex-row flex-wrap items-center gap-y-1.5 px-2.5 py-1"
        :class="[
          isPopover ? 'mx-2 mb-0.5 mt-1.5 rounded border border-gray-200 bg-gray-100' : 'border-b border-gray-200',
        ]"
      >
        <!-- Current value -->
        <template v-if="valueType?.isList">
          <button
            v-for="(v, i) in valueVignettes"
            :key="i"
            class="mr-2 flex flex-row items-center rounded bg-gray-100 px-1"
          >
            <IconInline v-if="v.icon" v-bind="v.icon" class="mr-1.5 w-5 text-gray-700" />
            <span class="truncate">{{ v.title ?? "???" }}</span>
            <!-- Deselect -->
            <button
              v-if="!isDisabled && isInput"
              class="ml-1.5 text-gray-400 hover:text-gray-700"
              @click.stop="deselect(i)"
            >
              <i class="fas fa-xmark" />
            </button>
          </button>
        </template>
        <!-- Query -->
        <span class="flex flex-row items-center">
          <IconInline
            v-bind="icon ?? makeIcon({ faName: 'fas fa-caret-circle-down' })"
            class="mr-2 w-5 text-gray-700"
          />
          <input
            ref="queryRef"
            v-model="query"
            type="text"
            class="w-full border-0 bg-transparent p-0 placeholder-gray-500 outline-none ring-0 focus:ring-0"
            :placeholder="placeholder ?? `Select ${facetName ?? '???'}`"
            @keydown.enter.stop.prevent="activeResultId && select(activeResultId)"
            @keydown.up.stop.prevent="focus('previous')"
            @keydown.down.stop.prevent="focus('next')"
          />
        </span>
      </div>
      <!-- Body -->
      <Scroll
        id="body"
        size-is-dynamic
        :size="{ width, height: MAX_HEIGHT }"
        :orientation="Orientation.VERTICAL"
        :track-width="ScrollbarWidth.sm"
        track-is-overlay
      >
        <!-- Results -->
        <ul v-if="results.length > 0" class="flex flex-col py-0.5" :class="[isPopover ? 'mx-2 mb-1 mt-0.5' : '']">
          <template v-for="item in results" :key="item.id">
            <!-- Results -->
            <li
              :ref="(ref?: any) => (ref != null ? (resultsRefs[item.id] = ref) : delete resultsRefs[item.id])"
              role="menuitem"
              class="mx-0.5 mb-[1px] mr-1.5 mt-[1px] flex h-[28px] max-w-full cursor-pointer flex-row items-center rounded border border-transparent px-1.5 hover:bg-gray-100"
              :class="[isActive(item) ? 'bg-gray-100' : '']"
              :data-selected="isSelected(item)"
              :data-active="isActive(item)"
              @click.prevent="select(item)"
            >
              <!-- Content -->
              <IconInline v-if="item.icon" v-bind="item.icon" class="mr-1.5 w-5 flex-shrink-0 text-gray-700" />
              <span v-else class="mr-1.5 w-5 flex-shrink-0 text-gray-700" />
              <span class="flex-1 select-none">
                <span class="truncate" v-html="item.titleMarked ?? item.title" />
                <!-- Checked -->
                <i v-if="isSelected(item)" class="fas fa-check flex-shrink-0 pl-2 pr-1 text-gray-700" />
              </span>
              <!-- Metadata -->
              <span class="ml-auto flex-1 truncate pl-2 text-right">
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
