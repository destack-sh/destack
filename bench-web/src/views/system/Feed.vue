<script lang="ts" setup>
import {
  BlockData,
  BoxData,
  ChangeCategory,
  EditType,
  ExpressionData,
  ExpressionOp,
  LogData,
  LogProperty,
  NodeReferenceData,
  NodeType,
  ObjectType,
  Orientation,
  RunData,
  RunProperty,
  RunStatus,
  StepData,
  Timestamp,
  ViewData,
  type AnyNodeData,
} from "@/proto/wire";
import { describeNode, isNode, propertyReference, toNodeReference, type TypedNodeReferenceData } from "@/proto/wiring";
import { type Action } from "@/system/action";
import { PACKAGE_SCOPE } from "@/system/client";
import { supergraph, useExistingConnection, useSearchConnection } from "@/system/connection";
import { makeExpression, resolveSubject, type EditSubject } from "@/system/expression";
import { ICON_BY_NODE_TYPE, ICON_BY_RUN_STATUS, IconInline, getNodeIcon, makeIcon } from "@/system/icon";
import { ACTIVE_RUN_STATUSES, EDIT_TYPE_PAST_VERB, TERMINAL_RUN_STATUSES, toCamelName } from "@/system/lang";
import { canvas } from "@/system/space";
import { user } from "@/system/user";
import { getElement } from "@/utils/element";
import { humanizeNumber } from "@/utils/human";
import { ScrollbarWidth } from "@/utils/layout";
import { ACCENT_COLOR_BY_RUN_STATUS } from "@/utils/style";
import { formatRelativeDate } from "@/utils/time";
import { DEFAULT_HEADER_HEIGHT, useExpansion, useViewState } from "@/views/canvas";
import { makeViewId, viewEmits, type ViewComponent, type ViewExposed } from "@/views/common";
import Scroll from "@/views/containers/Scroll.vue";
import { computed, ref, toRef, type Ref } from "vue";

const HEADER_HEIGHT = DEFAULT_HEADER_HEIGHT;
const MIN_WIDTH = 320;
const MAX_WIDTH = 800;

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; size?: Required<Pick<BoxData, "width" | "height">> } & Partial<
    Pick<ViewData, "variant" | "focus" | "isInput" | "isInline" | "valueType" | "valuePacked">
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = makeViewId(props);
const { graph: spaceGraph } = useExistingConnection(self);

const { state, useStateProp } = useViewState({
  selfPtr: self,
  graph: spaceGraph,
  stateType: ObjectType.FEED_VIEW_STATE,
  props,
  emit,
});
const nodeType = useStateProp("nodeType", NodeType.LOG);
const activeFilterKeys = useStateProp("filterPills", []);

//
// Filter
//

type FeedItemBase = {
  kind: string;
  id: string;
  createdAt: Timestamp;
  actions: Action[];
  createdBy: EditSubject | null;
};
type LogEditItem = FeedItemBase & {
  kind: "log-edit";
  it: LogData;
  node: AnyNodeData | null;
  nodeType: NodeType;
};
type RunItem = FeedItemBase & {
  kind: "run";
  it: RunData;
  node: BlockData | StepData | null;
};
type FeedItem = LogEditItem | RunItem;

// NOTE :Incomplete: support generic Feed query instead of just pills (once we have proper expression builders)
type FilterPill = {
  key: string;
  name: string;
  isEnabled: boolean;
  group: string;
  filterIfActive?: ExpressionData;
  filterIfInactive?: ExpressionData;
};
const pills: Ref<FilterPill[]> = computed(() => {
  const pills: FilterPill[] = [];
  if (nodeType.value == NodeType.LOG) {
    const logCategory = propertyReference(ObjectType.LOG, LogProperty.category);
    pills.push({
      key: "log-category-space",
      name: "Space",
      isEnabled: true,
      group: "log-category",
      filterIfActive: makeExpression({
        op: ExpressionOp.EQUALS,
        propertyPtr: logCategory,
        value: ChangeCategory.SPACE,
      }),
      filterIfInactive: makeExpression({
        op: ExpressionOp.NOT_EQUALS,
        propertyPtr: logCategory,
        value: ChangeCategory.SPACE,
      }),
    });
    pills.push({
      key: "log-just-me",
      name: "Just me",
      isEnabled: user.value != null,
      group: "log-subject",
      filterIfActive: makeExpression({
        op: ExpressionOp.EQUALS,
        propertyPtr: propertyReference(ObjectType.LOG, LogProperty.createdByPtr),
        value: toNodeReference(user.value!),
      }),
    });
    pills.push({
      key: "log-not-me",
      name: "Not me",
      isEnabled: user.value != null,
      group: "log-subject",
      filterIfActive: makeExpression({
        op: ExpressionOp.NOT_EQUALS,
        propertyPtr: propertyReference(ObjectType.LOG, LogProperty.createdByPtr),
        value: toNodeReference(user.value!),
      }),
    });
  } else if (nodeType.value == NodeType.RUN) {
    const runStatus = propertyReference(ObjectType.RUN, RunProperty.status);
    pills.push({
      key: "run-status-active",
      name: "Active",
      isEnabled: true,
      group: "run-status",
      filterIfActive: makeExpression({ op: ExpressionOp.IN, propertyPtr: runStatus, value: ACTIVE_RUN_STATUSES }),
    });
    pills.push({
      key: "run-status-terminated",
      name: "Terminated",
      isEnabled: true,
      group: "run-status",
      filterIfActive: makeExpression({ op: ExpressionOp.IN, propertyPtr: runStatus, value: TERMINAL_RUN_STATUSES }),
    });
    pills.push({
      key: "run-status-failed",
      name: "Failed",
      isEnabled: true,
      group: "run-status",
      filterIfActive: makeExpression({ op: ExpressionOp.EQUALS, propertyPtr: runStatus, value: RunStatus.FAILED }),
    });
  }
  return pills;
});

function isPillActive(pill: FilterPill): boolean {
  return activeFilterKeys.value.includes(pill.key);
}
function togglePill(pill: FilterPill) {
  const isActive = isPillActive(pill);
  if (isActive) {
    activeFilterKeys.value = activeFilterKeys.value.filter((key) => key != pill.key);
  } else {
    activeFilterKeys.value = activeFilterKeys.value
      .filter((key) => pills.value.find((p) => p.key === key)?.group !== pill.group)
      .concat(pill.key);
  }
}

const effectiveFilter: Ref<ExpressionData> = computed(() => {
  const clauses: ExpressionData[] = [];
  // add given filter from state
  if (state.value.filter != null) {
    clauses.push(state.value.filter);
  }
  // and any pills
  for (const pill of pills.value) {
    if (!pill.isEnabled) continue;
    if (activeFilterKeys.value.includes(pill.key)) {
      if (pill.filterIfActive != null) {
        clauses.push(pill.filterIfActive);
      }
    } else if (pill.filterIfInactive != null) {
      clauses.push(pill.filterIfInactive);
    }
  }
  const filter = makeExpression({ op: ExpressionOp.AND, clauses });
  return filter;
});

//
// Feed
//

const { roots, graph, connection, isStale, isConnected, isConnecting, page } = useSearchConnection(
  { name: `feed.${toCamelName(NodeType, nodeType.value).toLowerCase()}`, live: true },
  computed(() => ({
    scope: PACKAGE_SCOPE.value,
    nodeType: nodeType.value,
    first: 32,
    count: true,
    sort: [
      makeExpression({
        op: ExpressionOp.DESCENDING,
        propertyPtr: propertyReference(nodeType.value as unknown as ObjectType, LogProperty.createdAt),
      }),
    ],
    filter: effectiveFilter.value,
  })),
);
const items = computed<FeedItem[]>(() => {
  const items: FeedItem[] = [];
  for (const it of roots.value) {
    if (isNode(it, NodeType.LOG)) {
      const item: LogEditItem = {
        kind: "log-edit",
        id: it.id,
        it,
        node: it.nodePtr != null ? supergraph.get(it.nodePtr) : null,
        nodeType: it.nodePtr!.type,
        createdAt: it.createdAt!,
        createdBy: resolveSubject(it.createdByPtr),
        actions: [],
      };
      items.push(item);
    } else if (isNode(it, NodeType.RUN)) {
      let node: BlockData | StepData | null = null;
      if (it.stepPtr != null) node = supergraph.get(it.stepPtr) as StepData;
      else if (it.blockPtr != null) node = supergraph.get(it.blockPtr) as BlockData;
      const item: RunItem = {
        kind: "run",
        id: it.id,
        it,
        node,
        createdAt: it.createdAt!,
        createdBy: resolveSubject(it.createdByPtr),
        actions: [],
      };
      items.push(item);
    } else {
      throw new Error(`unexpected node type: ${describeNode(it)}`);
    }
  }
  return items;
});
const itemRefs: Ref<Record<string, HTMLElement>> = ref({});

function toSubjectIcon(item: FeedItem) {
  return item.createdBy != null
    ? getNodeIcon(item.createdBy)
    : ICON_BY_NODE_TYPE[item.it.metatype as unknown as NodeType];
}

//
// Interaction
//

const { toggleExpanded, isExpanded } = useExpansion({ graph: spaceGraph, tx: canvas.tx, self });
const focusedItem = computed(() => {
  if (props.focus?.nodesPtr.length ?? 0 > 0) {
    const focusedId = props.focus!.nodesPtr[0].id;
    return items.value.find((item) => item.id == focusedId);
  } else {
    return null;
  }
});
const focusedNode = computed(() => focusedItem.value?.it);

function mapToNode(element: HTMLElement | SVGElement | ViewComponent): NodeReferenceData | null {
  // find 'data-message-id' attribute
  let el = getElement(element);
  while (el != null) {
    const id = el.getAttribute("data-message-id");
    if (id != null) {
      const node = supergraph.get({ id });
      if (node != null) return toNodeReference(node);
    }
    el = el.parentElement;
  }
  return null;
}

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id, mapToNode });
</script>
<template>
  <div>
    <!-- Header -->
    <div
      class="group flex w-full flex-row items-center gap-x-1.5"
      :class="[!isInline ? 'mx-auto  px-5' : '']"
      :style="{ height: HEADER_HEIGHT + 'px', minWidth: MIN_WIDTH + 'px', maxWidth: MAX_WIDTH + 'px' }"
    >
      <!-- Filters -->
      <button
        v-for="pill in pills"
        :key="pill.name"
        :disabled="!pill.isEnabled"
        :data-active="isPillActive(pill)"
        class="data-[active=true] rounded-2xl border border-gray-200 px-2 enabled:text-gray-700 disabled:text-gray-400 data-[active=true]:border-primary-900 data-[active=true]:text-primary-900 data-[active=false]:hover:text-primary-900"
        @click="togglePill(pill)"
      >
        <span>{{ pill.name }}</span>
      </button>
      <!-- Staleness -->
      <Transition
        enter-active-class="transition-opacity ease-in duration-150"
        enter-from-class="opacity-0"
        enter-to-class="opacity-100"
        leave-active-class="transition-all ease-out duration-150"
        leave-from-class="opacity-100"
        leave-to-class="opacity-0"
      >
        <span v-if="isStale" class="ml-1">
          <i class="fas fa-circle-small animate-pulse text-gray-400" />
        </span>
      </Transition>
      <!-- Date picker -->
      <div class="ml-auto">
        <!-- NOTE :Incomplete: paginate & pick date range in Feed -->
        <button disabled class="enabled:text-gray-700 disabled:text-gray-400">
          <i class="fas fa-calendar-alt mr-1.5 w-5 text-center text-gray-400" />
          <span>All time</span>
        </button>
      </div>
    </div>
    <!-- Body -->
    <component
      :is="isInline ? 'div' : Scroll"
      :size="{ width: size?.width, height: size?.height! - HEADER_HEIGHT }"
      :orientation="Orientation.VERTICAL"
      :track-width="ScrollbarWidth.md"
      track-is-overlay
    >
      <!-- NOTE :UX :Incomplete: make feed not so ugly, support more feed variants (like table) -->
      <ul v-if="isConnected" class="mt-1 flex flex-col gap-y-1 py-1">
        <!-- Feed item -->
        <li
          v-for="item in items"
          :key="item.id"
          :ref="(ref: any) => (ref != null ? (itemRefs[item.id] = ref) : delete itemRefs[item.id])"
          :data-item-id="item.id"
          class="group/item mx-auto w-full rounded-md border py-0.5 text-gray-900 hover:cursor-pointer hover:bg-gray-100"
          :class="[
            !isInline ? 'mx-auto  px-5' : '',
            focusedNode?.id == item.id ? 'border-primary-900' : 'border-transparent',
          ]"
          :style="{ minWidth: MIN_WIDTH + 'px', maxWidth: MAX_WIDTH + 'px' }"
        >
          <!-- Item header -->
          <div class="flex flex-row flex-nowrap items-center gap-x-1">
            <!-- Log -->
            <template v-if="item.kind == 'log-edit'">
              <!-- Subject -->
              <button class="flex-shrink-0">
                <IconInline v-bind="toSubjectIcon(item)" class="mr-1 w-5 text-gray-700" />
                <span v-if="item.it.createdByPtr != null">
                  {{ (item.createdBy as any)?.name ?? toCamelName(NodeType, item.it.createdByPtr.type) }}
                </span>
                <span v-else class="italic">System</span>
              </button>
              <!-- Verb -->
              <span>
                <!-- <IconInline v-bind="item.icon" class="text-gray-700 mr-1" /> -->
                <span>{{ EDIT_TYPE_PAST_VERB[item.it.type as unknown as EditType] }}</span>
              </span>
              <!-- Object -->
              <button
                class="group/node flex-shrink-0 px-1 hover:bg-primary-100 hover:text-primary-900"
                @click="item.node && canvas.goToNode(item.node)"
              >
                <IconInline
                  v-bind="item.node != null ? getNodeIcon(item.node) : ICON_BY_NODE_TYPE[item.nodeType]"
                  class="mr-1 w-5 text-gray-700 group-hover/node:text-primary-900"
                />
                <span>{{ (item.node as any)?.name ?? toCamelName(NodeType, item.nodeType) }}</span>
                <!-- Old name if new name is different -->
                <span
                  v-if="
                    item.it.vignette?.name != null &&
                    item.node != null &&
                    item.it.vignette?.name != (item.node as any)?.name
                  "
                  class="text-gray-400"
                >
                  ({{ item.it.vignette?.name }})
                </span>
              </button>
            </template>

            <!-- Run -->
            <template v-else-if="item.kind == 'run'">
              <!-- Status -->
              <IconInline
                class="w-5"
                :class="ACCENT_COLOR_BY_RUN_STATUS[item.it.status]"
                v-bind="ICON_BY_RUN_STATUS[item.it.status]"
              />
              <!-- Node -->
              <button>
                <IconInline
                  v-bind="item.node != null ? getNodeIcon(item.node) : makeIcon('fas fa-lambda')"
                  class="mr-1 w-5 text-gray-700 group-hover/node:text-primary-900"
                />
                <span>{{ (item.node as any)?.name ?? "Lambda" }}</span>
              </button>
              <!-- Subject -->
              by
              <button class="flex-shrink-0">
                <IconInline v-bind="toSubjectIcon(item)" class="mr-1 w-5 text-gray-700" />
                <span>{{ (item.createdBy as any)?.name ?? toCamelName(NodeType, item.it.createdByPtr?.type) }}</span>
              </button>
            </template>
            <span v-else class="text-danger-500">???</span>

            <!-- Extra stuff -->
            <div class="ml-auto inline-flex flex-row items-center gap-x-2">
              <!-- Actions -->
              <button
                v-for="action in item.actions"
                :key="action.id"
                v-tooltip="{ title: action.title, small: true }"
                class="text-gray-400 hover:text-gray-900 hover:opacity-100 group-hover/item:opacity-100"
                :class="focusedNode?.id == item.id ? '' : 'opacity-0'"
              >
                <IconInline v-bind="action.icon" />
              </button>
              <!-- ... -->
              <!-- Time -->
              <span class="text-gray-400">
                {{ formatRelativeDate(item.createdAt, { minUnit: "m", minValue: 1 }) }}
              </span>
            </div>
          </div>

          <!-- Item body (if expanded) -->
          <!-- ...? -->
        </li>
        <!-- Nothing found -->
        <div v-if="items.length == 0" class="mx-auto my-1 w-full text-center">
          <i class="fas fa-empty-set w-5 text-center text-gray-400" />
          <span class="ml-1 text-gray-500">No results</span>
        </div>
        <!-- End of list -->
        <div v-if="page.total && page.size < page.total" class="mx-auto my-1 w-full text-center">
          <i class="fas fa-ellipsis-h w-5 text-center text-gray-400" />
          <span v-if="page.total" class="ml-1 text-gray-500">{{ humanizeNumber(page.total - page.size) }} more</span>
        </div>
      </ul>
      <!-- Loading -->
      <div v-else class="flex h-full min-h-20 w-full flex-col justify-center text-center">
        <Transition
          enter-from-class="opacity-0"
          enter-active-class="transition-opacity duration-200"
          enter-to-class="opacity-100"
          appear
        >
          <i class="fas fa-spinner-third animate-spin text-gray-400" />
        </Transition>
      </div>
    </component>
  </div>
</template>
