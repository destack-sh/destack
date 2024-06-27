<script lang="ts" setup>
import {
  BlockData,
  BoxData,
  EditCategory,
  EditType,
  ExpressionOp,
  IconData,
  LogData,
  LogProperty,
  NodeReferenceData,
  NodeType,
  ObjectType,
  PROPERTY_INFOS_BY_TYPE,
  RunData,
  StepData,
  Timestamp,
  UserData,
  ViewData,
  type AnyNodeData,
} from "@/proto/wire";
import { propertyReference, toNodeReference, type TypedNodeReferenceData } from "@/proto/wiring";
import { getAction, type Action } from "@/system/action";
import { PACKAGE_SCOPE } from "@/system/client";
import { supergraph, useExistingConnection, useSearchConnection } from "@/system/connection";
import { makeExpression } from "@/system/expression";
import { ICON_BY_EDIT_TYPE, ICON_BY_NODE_TYPE, IconInline, getNodeIcon } from "@/system/icon";
import { EDIT_TYPE_PAST_VERB, toCamelName } from "@/system/lang";
import { canvas } from "@/system/space";
import { packProtoStruct } from "@/system/transaction";
import { getTypeIdentityForProperty, packValue, packValueSimple, packValueSimpleStruct } from "@/system/value";
import { getElement } from "@/utils/element";
import { formatRelativeDate } from "@/utils/time";
import { tooltipFromAction } from "@/utils/tooltip";
import { DEFAULT_HEADER_HEIGHT, useExpansion } from "@/views/canvas";
import { makeViewId, viewEmits, type ViewComponent, type ViewExposed } from "@/views/common";
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
  subject: UserData | RunData | null;
};
type LogChangeItem = FeedItemBase & {
  kind: "log-change";
  it: LogData;
  nodes: AnyNodeData[];
  subject: UserData | RunData | null;
};
type RunItem = FeedItemBase & {
  kind: "run";
  it: RunData;
  node: BlockData | StepData | null;
};
type FeedItem = LogEditItem | LogChangeItem | RunItem;

// TODO :Incomplete!: store Feed query (and View-type-specific data) in view node
const nodeType = NodeType.LOG;
const { roots, graph, connection, page } = useSearchConnection(
  { name: `feed.${toCamelName(NodeType, nodeType).toLowerCase()}`, live: true },
  {
    scope: PACKAGE_SCOPE.value,
    // nocheckin: parameterize Feed search
    nodeType: nodeType,
    first: 16,
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
            EditCategory.SPACE,
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
      const subject = it.createdByPtr != null ? (supergraph.get(it.createdByPtr) as RunData | UserData | null) : null;
      const item: LogEditItem = {
        kind: "log-edit",
        id: it.id,
        it,
        icon: ICON_BY_EDIT_TYPE[it.type as unknown as EditType] ?? ICON_BY_NODE_TYPE[NodeType.LOG]!,
        node: it.nodePtr != null ? supergraph.get(it.nodePtr) : null,
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
const { toggleExpanded, isExpanded } = useExpansion({
  graph: spaceGraph,
  connection: spaceConnection,
  self,
});
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
      class="group mx-auto flex w-full flex-row items-center"
      :class="[isInline ? '' : 'mx-5']"
      :style="{ height: HEADER_HEIGHT + 'px' }"
    >
      nocheckin feed filter pills
    </div>
    <!-- Content -->
    <ul v-if="connection.isConnected.value" class="mt-0.5 flex flex-col gap-y-[3px] py-1">
      <!-- Feed item -->
      <li
        v-for="item in items"
        :key="item.id"
        :ref="(ref: any) => (ref != null ? (itemRefs[item.id] = ref) : delete itemRefs[item.id])"
        :data-item-id="item.id"
        class="group/item rounded-md border py-0.5 pl-1 pr-1.5 text-gray-900 hover:cursor-pointer hover:bg-gray-100"
        :class="[isInline ? '' : 'mx-5', focusedNode?.id == item.id ? 'border-primary-900' : 'border-transparent']"
      >
        <!-- Item header -->
        <div class="flex flex-row flex-nowrap items-center gap-x-1.5">
          <!-- nocheckin -->
          <!-- Expand -->
          <button
            class="mr-1 w-5 rounded enabled:hover:text-primary-900"
            :class="focusedNode?.id == item.id ? '' : 'text-gray-400'"
            @click.stop="toggleExpanded(item.it)"
          >
            <i
              class="fas fa-chevron-right dxuration-75 transition-transform"
              :class="[isExpanded(item.it) ? 'rotate-90' : 'rotate-0']"
            />
          </button>

          <!-- Log -->
          <template v-if="item.kind == 'log-edit'">
            <!-- Subject -->
            <button v-if="item.it.createdByPtr">
              <IconInline
                v-bind="item.subject != null ? getNodeIcon(item.subject) : null"
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
              v-if="item.node"
              class="group/node px-1 hover:bg-primary-100 hover:text-primary-900"
              @click="canvas.goToNode(item.node)"
            >
              <IconInline
                v-bind="getNodeIcon(item.node)"
                class="mr-1.5 text-gray-700 group-hover/node:text-primary-900"
              />
              <span>{{ (item.node as any)?.name ?? toCamelName(NodeType, item.node.metatype) }}</span>
            </button>
            <span v-else class="group/node">
              <IconInline
                v-bind="ICON_BY_NODE_TYPE[item.it.nodePtr!.type]"
                class="text-gray-700 group-hover/node:text-primary-900"
              />
              <span class="ml-1 italic">Unavailable</span>
            </span>
          </template>

          <!-- Run -->
          <template v-else-if="item.kind == 'run'"> run! </template>
          <span v-else class="text-danger-500">???</span>

          <!-- Extra stuff -->
          <div class="ml-auto inline-flex flex-row items-center gap-x-1.5">
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

        <!-- Item body -->
        <!-- ...? -->
      </li>
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
  </div>
</template>
