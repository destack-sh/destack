<script lang="ts" setup>
import {
  BlockData,
  BoxData,
  ChangeCategory,
  EditType,
  ExpressionOp,
  IconData,
  LogData,
  LogProperty,
  NodeReferenceData,
  NodeType,
  ObjectType,
  Orientation,
  PROPERTY_INFOS_BY_TYPE,
  RunData,
  StepData,
  Timestamp,
  UserData,
  ViewData,
  type AnyNodeData,
} from "@/proto/wire";
import { isNode, propertyReference, toNodeReference, type TypedNodeReferenceData } from "@/proto/wiring";
import { getAction, type Action } from "@/system/action";
import { PACKAGE_SCOPE } from "@/system/client";
import { supergraph, useExistingConnection, useSearchConnection } from "@/system/connection";
import { makeExpression } from "@/system/expression";
import { ICON_BY_EDIT_TYPE, ICON_BY_NODE_TYPE, IconInline, getNodeIcon } from "@/system/icon";
import { EDIT_TYPE_PAST_VERB, toCamelName } from "@/system/lang";
import { canvas } from "@/system/space";
import { getTypeIdentityForProperty, packValueSimpleStruct } from "@/system/value";
import { getElement } from "@/utils/element";
import { humanizeNumber } from "@/utils/human";
import { ScrollbarWidth } from "@/utils/layout";
import { formatRelativeDate } from "@/utils/time";
import { DEFAULT_HEADER_HEIGHT, useExpansion } from "@/views/canvas";
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
const { graph: spaceGraph, connection: spaceConnection } = useExistingConnection(self);

type FeedItemBase = {
  kind: string;
  id: string;
  icon: IconData;
  createdAt: Timestamp;
  actions: Action[];
};
type LogEditItem = FeedItemBase & {
  kind: "log-edit";
  it: LogData;
  node: AnyNodeData | null;
  nodeType: NodeType;
  subject: UserData | RunData | BlockData | StepData | null;
};
type LogChangeItem = FeedItemBase & {
  kind: "log-change";
  it: LogData;
  nodes: AnyNodeData[];
  subject: UserData | RunData | BlockData | StepData | null;
};
type RunItem = FeedItemBase & {
  kind: "run";
  it: RunData;
  node: BlockData | StepData | null;
};
type FeedItem = LogEditItem | LogChangeItem | RunItem;

// NOTE :Incomplete: support generic Feed query instead of just pills (once we have proper expression builders)
const nodeType = NodeType.LOG;
type FilterPill = {
  key: string;
  name: string;
  isEnabled: boolean;
  isActive: boolean;
  group: string;
  toggle?: () => void;
};
const pills: FilterPill[] = [
  {
    key: "status-active",
    name: "Active",
    isEnabled: true,
    isActive: true,
    group: "status",
  },
  {
    key: "status-terminated",
    name: "Terminated",
    isEnabled: true,
    isActive: false,
    group: "status",
  },
  {
    key: "status-failed",
    name: "Failed",
    isEnabled: true,
    isActive: false,
    group: "status",
  },
];

const { roots, graph, connection, page } = useSearchConnection(
  { name: `feed.${toCamelName(NodeType, nodeType).toLowerCase()}`, live: true },
  {
    scope: PACKAGE_SCOPE.value,
    nodeType: nodeType,
    first: 32,
    count: true,
    sort: [
      makeExpression({
        op: ExpressionOp.DESCENDING,
        propertyPtr: propertyReference(nodeType as unknown as ObjectType, LogProperty.createdAt),
      }),
    ],
    filter: makeExpression({
      op: ExpressionOp.AND,
      clauses: [
        makeExpression({
          op: ExpressionOp.NOT_EQUALS,
          propertyPtr: propertyReference(ObjectType.LOG, LogProperty.category),
          valuePacked: packValueSimpleStruct(
            ChangeCategory.SPACE,
            getTypeIdentityForProperty(PROPERTY_INFOS_BY_TYPE[ObjectType.LOG][LogProperty.category]),
          ),
        }),
      ],
    }),
  },
);

const items = computed<FeedItem[]>(() => {
  const items: FeedItem[] = [];
  if (nodeType == NodeType.LOG) {
    for (const it of roots.value) {
      let subject: UserData | RunData | BlockData | StepData | null;
      if (it.createdByPtr != null) {
        if (it.createdByPtr.type == NodeType.RUN) {
          subject = supergraph.get({ ck: it.createdByPtr.baseCk }) as BlockData | StepData | null;
        } else {
          subject = supergraph.get(it.createdByPtr) as UserData | null;
        }
      } else {
        subject = null;
      }
      const item: LogEditItem = {
        kind: "log-edit",
        id: it.id,
        it,
        icon: ICON_BY_EDIT_TYPE[it.type as unknown as EditType] ?? ICON_BY_NODE_TYPE[NodeType.LOG]!,
        node: it.nodePtr != null ? supergraph.get(it.nodePtr) : null,
        nodeType: it.nodePtr!.type,
        subject,
        createdAt: it.createdAt!,
        actions: [getAction("common.history.undo"), getAction("common.history.redo")],
      };
      items.push(item);
    }
  } else {
    throw new Error(`unsupported feed node type: ${nodeType}`);
  }
  return items;
});
const itemRefs: Ref<Record<string, HTMLElement>> = ref({});

// interaction
const { toggleExpanded, isExpanded } = useExpansion({ graph: spaceGraph, connection: spaceConnection, self });
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
        :data-active="pill.isActive"
        class="data-[active=true] rounded-2xl border border-gray-200 px-2 py-0.5 enabled:bg-gray-50 enabled:text-gray-700 disabled:text-gray-400 data-[active=true]:border-primary-900 data-[active=true]:text-primary-900 data-[active=true]:hover:bg-gray-100 data-[active=false]:hover:text-primary-900"
        @click="pill.toggle"
      >
        <span>{{ pill.name }}</span>
      </button>
      <!-- Date picker -->
      <div class="ml-auto">
        <!-- NOTE :Incomplete: paginate & pick date range in Feed -->
        <button disabled class="enabled:text-gray-700 disabled:text-gray-400">
          <i class="fas fa-calendar-alt mr-1.5 text-gray-400" />
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
      <!-- TODO :UX: make feed not so ugly -->
      <!-- NOTE :Incomplete: support more feed variants (like table) -->
      <ul v-if="connection.isConnected.value" class="flex flex-col gap-y-[3px] py-1">
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
              <button v-if="item.it.createdByPtr" class="flex-shrink-0">
                <IconInline
                  v-bind="
                    item.subject != null
                      ? getNodeIcon(item.subject)
                      : ICON_BY_NODE_TYPE[item.it.metatype as unknown as NodeType]
                  "
                  class="mr-1.5 text-gray-700"
                />
                <span>{{ (item.subject as any)?.name ?? toCamelName(NodeType, item.it.createdByPtr.type) }}</span>
              </button>
              <span v-else class="italic text-gray-900">System</span>
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
                  class="mr-1.5 text-gray-700 group-hover/node:text-primary-900"
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
            <template v-else-if="item.kind == 'run'"> run! </template>
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
        <div class="mx-auto my-1 w-full text-center">
          <i class="fas fa-ellipsis-h w-5 text-center text-gray-400" />
          <span v-if="page.total" class="ml-1 text-gray-500">{{ humanizeNumber(page.total - page.size) }} more</span>
        </div>
      </ul>
      <!-- Loading -->
      <div v-else class="flex h-full min-h-20 w-full flex-col text-center align-middle">
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
