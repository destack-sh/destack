<script lang="ts" setup>
import { EditSubject, makeExpression } from "@/language/expression";
import { createMessage, getMessageAuthorPtr } from "@/language/message";
import { useSubnodeProperty } from "@/language/node";
import {
  BlockData,
  ExpressionType,
  IconData,
  MessageData,
  MessageProperty,
  MessageType,
  NodeReferenceData,
  NodeType,
  Orientation,
  PackageData,
  RectangleData,
  ViewData,
  ViewType,
} from "@/proto/wire";
import { isNode, propertyReference, toNodeRef, TypedNodeReferenceData } from "@/proto/wiring";
import { BENCH_SCOPE, benchPtr } from "@/system/client";
import { SearchConnectionParams, useSearchConnection } from "@/system/connection";
import { supergraph } from "@/system/globals";
import { canvas, pkg, pkgGraph } from "@/system/space";
import { AvatarInline, getNodeIcon, getNodeName, IconInline } from "@/ui/icon";
import { computedValue } from "@/utils/ref";
import { formatAbsoluteDate, tsToDt } from "@/utils/time";
import { viewEmits, type ViewExposed } from "@/views/common";
import Scroll from "@/views/containers/Scroll.vue";
import File from "@/views/content/File.vue";
import Text from "@/views/content/Text.vue";
import { useElementSize } from "@vueuse/core";
import { computed, Ref, ref, toRef } from "vue";

const MESSAGE_HEIGHT_MIN = 28;
const MESSAGE_MAX_TIME_DELTA_SECONDS = 5 * 60; // 5 minutes
const MESSAGE_SIDE_WIDTH = 44;
const props = defineProps<
  {
    self?: TypedNodeReferenceData<NodeType.VIEW>;
    id: string;
    size?: Partial<Pick<RectangleData, "width" | "height">>;
  } & Partial<Pick<ViewData, "name" | "title" | "icon" | "nodePtr" | "subnodePacked">>
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = toRef(props, "id");
const state = canvas.registerView(self, id);

// subnode
const subnodePacked = toRef(props, "subnodePacked");
const text = useSubnodeProperty(NodeType.VIEW, ViewType.CHAT, subnodePacked, "text");
const nodesPtr = useSubnodeProperty(NodeType.VIEW, ViewType.CHAT, subnodePacked, "nodesPtr");
const nodes = supergraph.getManyRef(nodesPtr);
const files = computed(() => nodes.value.filter((n) => isNode(n, NodeType.FILE)));

// node
const nodePtr = computedValue(() => props.nodePtr);
const node = pkgGraph.getRef(nodePtr);
const nodeAncestors = pkgGraph.getAncestorsRef(nodePtr);
const origin: Ref<BlockData | PackageData | null> = computed(() => {
  if (isNode(node.value, NodeType.BLOCK)) {
    return node.value;
  } else {
    const block = nodeAncestors.value.find((a) => isNode(a, NodeType.BLOCK));
    if (block != null) {
      return block;
    }
  }
  return pkg.value;
});

const inputContainerRef = ref<HTMLInputElement | null>(null);
const inputRef = ref<HTMLInputElement | null>(null);
const inputSize = useElementSize(inputContainerRef);
const bodyScrollRef = ref<InstanceType<typeof Scroll> | null>(null);

// messages :MessageRouting
const DEFAULT_SORT = makeExpression({
  type: ExpressionType.ASCENDING,
  propertyPtr: propertyReference(NodeType.MESSAGE, MessageProperty.createdAt),
});
const {
  roots: messages,
  connection,
  graph,
  isStale,
  isConnecting,
} = useSearchConnection(
  { name: "chat", live: true },
  computed(
    (): SearchConnectionParams<NodeType.MESSAGE> => ({
      scope: BENCH_SCOPE.value,
      nodeType: NodeType.MESSAGE,
      count: true,
      isEnabled: origin.value != null,
      sort: [DEFAULT_SORT],
      filter: makeExpression({
        type: ExpressionType.EQUALS,
        propertyPtr: propertyReference(NodeType.MESSAGE, MessageProperty.originPtr),
        value: origin.value != null ? toNodeRef(origin.value) : undefined,
      }),
    }),
  ),
);
const remoteAuthorsPtr: Ref<NodeReferenceData[]> = computed(() => {
  const authorsPtrById: Record<string, NodeReferenceData> = {};
  for (const message of messages.value) {
    if (message.createdByPtr != null && message.createdByPtr.nodeType == NodeType.USER) {
      authorsPtrById[message.createdByPtr.id!] = message.createdByPtr;
    }
  }
  return Object.values(authorsPtrById);
});
const remoteAuthors = supergraph.getManyRef(remoteAuthorsPtr);
const remoteAuthorsById: Ref<Record<string, EditSubject>> = computed(() =>
  remoteAuthors.value.reduce(
    (acc, author) => {
      acc[author.id!] = author as EditSubject;
      return acc;
    },
    {} as Record<string, EditSubject>,
  ),
);
type MessageView = {
  idx: number;
  message: MessageData;
  author: EditSubject | null;
  authorIcon: IconData | null;
  isNewGroup: boolean;
  isNewDay: boolean;
};
const messageViews = computed(() => {
  const views: MessageView[] = [];
  for (let i = 0; i < messages.value.length; i++) {
    const message = messages.value[i];
    const authorPtr = getMessageAuthorPtr(message);
    const author = authorPtr != null ? (remoteAuthorsById.value[authorPtr.id!] ?? supergraph.get(authorPtr)) : null;
    const authorIcon = author != null ? (getNodeIcon(author) ?? null) : null;
    let isNewGroup;
    let isNewDay;
    if (i == 0) {
      isNewGroup = true;
      isNewDay = true;
    } else {
      const previousDt = tsToDt(messages.value[i - 1].createdAt!);
      const currentDt = tsToDt(message.createdAt!);
      isNewGroup =
        message.createdByPtr?.id != messages.value[i - 1]?.createdByPtr?.id ||
        Math.abs(Number(messages.value[i - 1].createdAt!.seconds) - Number(message.createdAt!.seconds)) >
          MESSAGE_MAX_TIME_DELTA_SECONDS;
      isNewDay = previousDt.day != currentDt.day;
    }
    const richMessage: MessageView = { idx: i, message, author, authorIcon, isNewGroup, isNewDay };
    views.push(richMessage);
  }
  return views;
});

function submit() {
  // create message
  if (benchPtr.value == null) throw new Error("no bench");
  if (origin.value == null) throw new Error("no origin");
  createMessage(connection.tx, graph, {
    message: {
      type: MessageType.LOCAL,
      parentPtr: benchPtr.value,
      originPtr: toNodeRef(origin.value),
      text: text.value,
    },
  });
}

function clear() {
  state.update(
    { metatype: NodeType.VIEW, type: ViewType.CHAT, subnode: { text: undefined, nodesPtr: [] } },
    { debounce: "tick" },
  );
}

defineExpose<ViewExposed>({ self, id });
</script>
<template>
  <div>
    <!-- Body -->
    <Scroll
      id="scroll"
      ref="bodyScrollRef"
      :orientation="Orientation.VERTICAL"
      stick-to-end
      :size="{ height: size?.height != null ? size.height - inputSize.height.value : undefined }"
    >
      <!-- Messages -->
      <ul class="relative mb-3 flex flex-col">
        <!-- Message -->
        <li
          v-for="{ idx, message, author, authorIcon, isNewGroup, isNewDay } in messageViews"
          :key="message.id"
          class="mx-5"
          :class="[isNewGroup && idx != 0 ? (isNewDay ? 'mt-1' : 'mt-2') : '']"
        >
          <!-- New day? -->
          <div v-if="isNewDay" class="relative my-1 flex items-center">
            <div class="flex-grow border-t border-gray-200"></div>
            <div class="mx-4 flex-shrink text-sm text-gray-400">
              {{ formatAbsoluteDate(message.createdAt!, { prefer: "date" }) }}
            </div>
            <div class="flex-grow border-t border-gray-200"></div>
          </div>
          <!-- ... -->
          <div class="flex flex-row">
            <!-- Side -->
            <div
              class="flex-shrink-0 text-center mt-1"
              :style="{
                width: MESSAGE_SIDE_WIDTH + 'px',
              }"
            >
              <AvatarInline v-if="isNewGroup" class="mr-1 text-gray-700" size="medium" v-bind="authorIcon" />
            </div>
            <!-- Body -->
            <div>
              <!-- Meta (if new group) -->
              <div v-if="isNewGroup">
                <!-- Author -->
                <span class="font-medium">
                  {{ author != null ? getNodeName(author) : "???" }}
                </span>
                <!-- Timestamp -->
                <span class="ml-1.5 text-xs text-gray-400">
                  {{ formatAbsoluteDate(message.createdAt!, { prefer: "time" }) }}
                </span>
              </div>
              <!-- Content -->
              <Text :id="'text-' + message.id" is-minimal :model-value="message.text" />
              <!-- Extras -->
              <File
                v-for="filePtr in message.nodesPtr.filter((n) => n.nodeType == NodeType.FILE)"
                :id="'file-' + filePtr.id"
                :key="filePtr.id"
                :model-value="toNodeRef(filePtr)"
              />
            </div>
          </div>
        </li>
      </ul>
      <!-- Empty -->
      <div
        v-if="messages.length == 0"
        class="mx-1.5 flex w-full flex-row items-center justify-center px-2.5"
        :style="{
          height: MESSAGE_HEIGHT_MIN + 'px',
        }"
      >
        <!-- Loading -->
        <span v-if="isConnecting" class="">
          <i class="fas fa-spinner-third animate-spin text-gray-400" />
        </span>
        <!-- Empty -->
        <span v-else class="text-gray-400">No Messages here yet</span>
      </div>
    </Scroll>
    <!-- Input -->
    <div ref="inputContainerRef" class="mx-5">
      <div class="relative rounded border border-gray-200 px-2 py-1.5">
        <div class="flex flex-row">
          <!-- Side -->
          <div class="sticky top-0 text-center" :style="{ width: MESSAGE_SIDE_WIDTH - 8 + 'px' }">
            <!-- Add extra -->
            <button
              v-tooltip="{ small: true, title: 'Add File' }"
              class="transition-color mr-1 rounded-2xl bg-gray-100 px-1.5 py-0.5 text-gray-700 duration-150 hover:bg-gray-200"
            >
              <i class="fas fa-plus" />
            </button>
          </div>
          <div class="flex-1">
            <Scroll
              id="input-scroll"
              ref="inputScrollRef"
              :orientation="Orientation.VERTICAL"
              size-is-dynamic
              :size="{ height: size?.height != null ? size.height / 2 : undefined }"
            >
              <!-- Text -->
              <Text
                id="input"
                ref="inputRef"
                is-input
                is-minimal
                suppress-enter
                suppress-drop
                :model-value="text"
                @update:model-value="
                  (value) =>
                    state.update(
                      { metatype: NodeType.VIEW, type: ViewType.CHAT, subnode: { text: value } },
                      { debounce: 'long' },
                    )
                "
                @keydown.enter="
                  (e) => {
                    if (!e.shiftKey) {
                      e.preventDefault();
                      submit();
                      clear();
                    }
                  }
                "
              />
              <!-- Extras -->
              <File v-for="file in files" :id="'file-' + file.id" :key="file.id" :model-value="toNodeRef(file)" />
            </Scroll>
          </div>
        </div>
      </div>
      <!-- Spacing -->
      <div class="h-3" />
    </div>
  </div>
</template>
