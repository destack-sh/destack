<script lang="ts" setup>
import { EditSubject, makeExpression } from "@/language/expression";
import { createMessage, getMessageAuthorPtr } from "@/language/message";
import { useSubnodeProperty } from "@/language/node";
import { getTextLine, trimText } from "@/language/text";
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
  TextData,
  ViewData,
  ViewType,
} from "@/proto/wire";
import { isNode, propertyReference, toNodeRef, TypedNodeReferenceData } from "@/proto/wiring";
import { BENCH_SCOPE, benchPtr } from "@/system/client";
import { SearchConnectionParams, useSearchConnection } from "@/system/connection";
import { supergraph } from "@/system/globals";
import { canvas, pkg, pkgGraph } from "@/system/space";
import { user } from "@/system/user";
import {
  ActionMapImplementation,
  fireAction,
  getAction,
  getNodesForAction,
  isActionEnabled,
  MESSAGE_CONTEXT_ACTIONS,
} from "@/ui/action";
import { AvatarInline, getNodeIcon, getNodeName, IconInline } from "@/ui/icon";
import { computedValue } from "@/utils/ref";
import { formatAbsoluteDate, tsToDt } from "@/utils/time";
import { viewEmits, type ViewExposed } from "@/views/common";
import Scroll from "@/views/containers/Scroll.vue";
import File from "@/views/content/File.vue";
import Text from "@/views/content/Text.vue";
import { useElementSize } from "@vueuse/core";
import { DateTime } from "luxon";
import { computed, nextTick, Ref, ref, toRef } from "vue";

const MESSAGE_HEIGHT_MIN = 28;
const MESSAGE_MAX_TIME_DELTA_SECONDS = 5 * 60; // 5 minutes
const MESSAGE_SIDE_WIDTH = 52;
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
const replyToPtr = useSubnodeProperty(NodeType.VIEW, ViewType.CHAT, subnodePacked, "replyToPtr");
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
const inputRef = ref<InstanceType<typeof Text> | null>(null);
const inputSize = useElementSize(inputContainerRef);
const bodyScrollRef = ref<InstanceType<typeof Scroll> | null>(null);
const editingTextRefs = ref<InstanceType<typeof Text>[] | null>(null);

//
// Messages :MessageRouting
//

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
  authorName: string | null;
  replyTo: MessageView | null;
  isNewGroup: boolean;
  isNewDate: boolean;
  isEdited: boolean;
  isEditing: boolean;
  isReplyingTo: boolean;
  isSelected: boolean;
};
const messageViews = computed(() => {
  const views: MessageView[] = [];
  const viewsById: Record<string, MessageView> = {};
  for (let i = 0; i < messages.value.length; i++) {
    const message = messages.value[i];
    const authorPtr = getMessageAuthorPtr(message);
    const author = authorPtr != null ? (remoteAuthorsById.value[authorPtr.id!] ?? supergraph.get(authorPtr)) : null;
    const authorIcon = author != null ? (getNodeIcon(author) ?? null) : null;
    const authorName = author != null ? (getNodeName(author) ?? null) : null;
    let isNewGroup;
    let isNewDate;
    if (i == 0) {
      isNewGroup = true;
      isNewDate = false;
    } else {
      const previousDt = tsToDt(messages.value[i - 1].createdAt!);
      const currentDt = tsToDt(message.createdAt!);
      isNewGroup =
        message.createdByPtr?.id != messages.value[i - 1]?.createdByPtr?.id ||
        Math.abs(Number(messages.value[i - 1].createdAt!.seconds) - Number(message.createdAt!.seconds)) >
          MESSAGE_MAX_TIME_DELTA_SECONDS;
      isNewDate = previousDt.day != currentDt.day;
    }
    const isEdited = message.updatedAt?.seconds != message.createdAt?.seconds;
    const isEditing = editingPtr.value?.id == message.id;
    const isReplyingTo = replyToPtr.value?.id == message.id;
    const isSelected = canvas.isSelected(message);
    const richMessage: MessageView = {
      idx: i,
      message,
      author,
      authorIcon,
      authorName,
      isNewGroup,
      isNewDate,
      isEdited,
      isEditing,
      isReplyingTo,
      isSelected,
      replyTo: null, // fill later
    };
    views.push(richMessage);
    viewsById[message.id] = richMessage;
  }
  // fill in replyTo
  for (const view of views) {
    if (view.message.replyToPtr != null) {
      const replyTo = viewsById[view.message.replyToPtr.id!];
      if (replyTo != null) {
        view.replyTo = replyTo;
        view.isNewGroup = true; // always begin new group for reply
      }
    }
  }
  return views;
});
const replyTo = computed(() => {
  if (replyToPtr.value == null) return null;
  return messageViews.value.find((m) => m.message.id == replyToPtr.value?.id);
});

//
// Interaction
//

const currentAuthor = user;
const editingText = ref<TextData | null>(null);
const editingPtr = ref<NodeReferenceData | null>(null);

function startEdit(message: MessageData) {
  editingPtr.value = toNodeRef(message);
  editingText.value = message.text ?? null;
  nextTick(() => {
    editingTextRefs.value?.[0]?.focus?.();
  });
}

function stopEdit() {
  editingPtr.value = null;
  editingText.value = null;
}

function submitEdit() {
  const message = messages.value.find((m) => m.id == editingPtr.value?.id);
  if (message == null) throw new Error("message not found");
  connection.tx.update(message, { text: editingText.value ?? undefined });
}

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
      replyToPtr: replyTo.value != null ? replyToPtr.value : undefined,
    },
  });
}

function clear() {
  state.update(
    { metatype: NodeType.VIEW, type: ViewType.CHAT, subnode: { text: undefined, nodesPtr: [], replyToPtr: undefined } },
    { debounce: "tick" },
  );
}

function stopReplying() {
  state.update(
    { metatype: NodeType.VIEW, type: ViewType.CHAT, subnode: { replyToPtr: undefined } },
    { debounce: "tick" },
  );
}

const actions: Partial<ActionMapImplementation<"chat">> = {
  "chat.message.reply": {
    isEnabled: (action, ctx) => {
      const { nodes: messages } = getNodesForAction(action, ctx, [NodeType.MESSAGE]);
      return messages.length > 0;
    },
    action: (action, ctx) => {
      const { nodes: messages } = getNodesForAction(action, ctx, [NodeType.MESSAGE]);
      state.update(
        { metatype: NodeType.VIEW, type: ViewType.CHAT, subnode: { replyToPtr: toNodeRef(messages[0]) } },
        { debounce: "tick" },
      );
      nextTick(() => {
        inputRef.value?.focus?.();
      });
    },
  },
  "chat.message.edit": {
    isEnabled: (action, ctx) => {
      const { nodes: messages } = getNodesForAction(action, ctx, [NodeType.MESSAGE]);
      return messages.length > 0 && messages.every((m) => m.createdByPtr?.id == currentAuthor.value?.id);
    },
    action: (action, ctx) => {
      const { nodes: messages } = getNodesForAction(action, ctx, [NodeType.MESSAGE]);
      startEdit(messages[0]);
    },
  },
};

defineExpose<ViewExposed>({ self, id, actions });
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
          v-for="{
            idx,
            message,
            author,
            replyTo,
            authorIcon,
            authorName,
            isEdited,
            isNewGroup,
            isNewDate,
            isEditing,
            isSelected,
            isReplyingTo,
          } in messageViews"
          :key="message.id"
          class="group/message mx-5 max-w-full"
          :class="[isNewGroup && idx != 0 ? 'mt-2' : '']"
          :data-node-id="message.id"
          :data-node-type="message.metatype"
          data-contextmenu-items="chat.message*"
        >
          <!-- New date (line with date in middle) -->
          <div v-if="isNewDate" class="relative mb-1 flex items-center">
            <div class="flex-grow border-t border-gray-200" />
            <div class="mx-4 flex-shrink text-sm text-gray-400">
              {{ formatAbsoluteDate(message.createdAt!, { prefer: "date" }) }}
            </div>
            <div class="flex-grow border-t border-gray-200"></div>
          </div>
          <!-- Replying to -->
          <div v-if="replyTo != null" class="relative flex max-w-full items-center">
            <!-- 'Line' (supposed to go from avatar to the author with a bend) -->
            <div
              class="absolute top-2 h-4 w-7 rounded rounded-b-none rounded-r-none border-l-2 border-t-2"
              :style="{ left: MESSAGE_SIDE_WIDTH / 2 - 3 + 'px' }"
            />
            <!-- Spacing for side -->
            <div class="" :style="{ width: MESSAGE_SIDE_WIDTH + 'px' }" />
            <!-- Author -->
            <span class="flex-shrink-0 text-gray-700">@{{ replyTo.authorName ?? "???" }}</span>
            <!-- Preview -->
            <span
              v-if="replyTo.message.text"
              class="ml-1 min-w-0 flex-1 truncate text-gray-400"
              :style="{ maxWidth: (size?.width != null ? size.width - 300 : 100) + 'px' }"
            >
              {{ getTextLine(replyTo.message.text) }}
            </span>
          </div>
          <!-- Body -->
          <div
            class="flex flex-row rounded transition-colors duration-150"
            :class="[
              isSelected ? 'bg-orange-100' : '',
              messageViews[idx - 1]?.isSelected && !isNewGroup ? 'rounded-t-none' : '',
              messageViews[idx + 1]?.isSelected && !messageViews[idx + 1]?.isNewGroup ? 'rounded-b-none' : '',
              isReplyingTo ? 'bg-gray-100' : '',
            ]"
          >
            <!-- Side -->
            <div
              class="flex-shrink-0 text-center"
              :class="[isNewGroup ? 'mt-1' : 'mt-0.5']"
              :style="{
                width: MESSAGE_SIDE_WIDTH + 'px',
              }"
            >
              <AvatarInline v-if="isNewGroup" class="mr-1 text-gray-700" size="medium" v-bind="authorIcon" />
              <span v-else class="text-xs text-gray-400 opacity-0 group-hover/message:opacity-100">
                {{ tsToDt(message.createdAt!).toLocaleString(DateTime.TIME_SIMPLE) }}
              </span>
            </div>
            <!-- Body -->
            <div class="relative flex-1">
              <!-- Meta (if new group) -->
              <div v-if="isNewGroup" class="">
                <!-- Author -->
                <span class="font-medium">
                  {{ authorName ?? "???" }}
                </span>
                <!-- Timestamp -->
                <span class="ml-1.5 text-xs text-gray-400">
                  {{ formatAbsoluteDate(message.createdAt!, { prefer: "time" }) }}
                </span>
                <!-- Edited? -->
                <span v-if="isEdited" class="text-xs ml-1 text-gray-400">(edited)</span>
              </div>
              <!-- Actions -->
              <div
                v-if="!isEditing"
                class="absolute right-0 top-0 z-10 flex flex-row rounded border border-gray-200 bg-white opacity-0 group-hover/message:opacity-100"
              >
                <button
                  v-for="action in MESSAGE_CONTEXT_ACTIONS.map(getAction)"
                  :key="action.id"
                  v-tooltip="{ small: true, title: action.title, group: 'message' }"
                  class="rounded px-1.5 py-0.5 transition-colors duration-150 enabled:cursor-pointer enabled:text-gray-700 enabled:hover:bg-gray-100 disabled:cursor-not-allowed disabled:text-gray-400"
                  :disabled="!isActionEnabled(action, { nodes: [message] })"
                  @click.stop.prevent="fireAction(action, { nodes: [message] })"
                >
                  <IconInline v-bind="action.icon" />
                </button>
              </div>
              <!-- Content -->
              <Text v-if="!isEditing" :id="'text-' + message.id" is-minimal :model-value="message.text" />
              <div v-else class="my-1">
                <Text
                  :id="'text-' + message.id"
                  ref="editingTextRefs"
                  :model-value="editingText ?? undefined"
                  is-input
                  suppress-enter
                  suppress-drop
                  @update:model-value="
                    (value) => {
                      editingText = value;
                    }
                  "
                  @keydown.enter="
                    (e) => {
                      if (!e.shiftKey) {
                        submitEdit();
                        stopEdit();
                      }
                    }
                  "
                  @keydown.esc.stop.prevent="stopEdit()"
                />
                <div v-if="isEditing" class="mt-0.5 flex-row text-xs text-gray-400">
                  <span>
                    escape to
                    <a href="#" class="text-primary-700 underline-offset-2 hover:underline" @click.stop="stopEdit()"
                      >cancel</a
                    >
                  </span>
                  •
                  <span>
                    enter to
                    <a href="#" class="text-primary-700 underline-offset-2 hover:underline" @click.stop="submitEdit()"
                      >save</a
                    >
                  </span>
                </div>
              </div>
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
      <!-- Replying to -->
      <div
        v-if="replyTo != null"
        class="flex w-full flex-row items-center rounded rounded-b-none border border-b-0 border-gray-200 bg-gray-100 px-2 py-1"
        role="button"
      >
        <span>Replying to</span>
        <span class="ml-1 font-medium text-gray-900">{{ replyTo.authorName ?? "???" }}</span>
        <!-- Preview -->
        <span
          v-if="replyTo.message.text"
          class="ml-1.5 truncate text-gray-400"
          :style="{
            maxWidth: (size?.width != null ? size.width - 300 : 100) + 'px',
          }"
        >
          {{ getTextLine(replyTo.message.text) }}
        </span>
        <!-- Clear -->
        <button class="ml-auto rounded-full px-1 text-gray-700 hover:text-gray-900" @click="stopReplying()">
          <i class="fas fa-xmark" />
        </button>
      </div>
      <!-- Box -->
      <div
        class="relative rounded border border-gray-200 px-2 py-1.5 focus-within:border-gray-400"
        :class="[replyTo != null ? 'rounded-t-none' : '']"
      >
        <div class="flex flex-row">
          <!-- Side -->
          <div class="sticky top-0 text-center" :style="{ width: MESSAGE_SIDE_WIDTH - 8 + 'px' }">
            <!-- Add extra -->
            <button
              v-tooltip="{ small: true, title: 'Add Context' }"
              class="transition-color mr-1 rounded-2xl bg-gray-100 px-1.5 py-0.5 text-gray-700 duration-150 hover:bg-gray-200"
            >
              <i class="fas fa-plus" />
            </button>
          </div>
          <!-- Input -->
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
