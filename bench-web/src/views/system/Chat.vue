<script lang="ts" setup>
import { ViewData, NodeType, BoxData, Variant, ObjectType, Orientation, TextData, MessageData } from "@/proto/wire";
import { describeNode, toNodeReference, type TypedNodeReferenceData } from "@/proto/wiring";
import { makeViewId, viewEmits, type ViewExposed } from "@/views/common";
import { canvas, inspectionPtr, pkg } from "@/system/space";
import { computed, ref, toRef, type Ref } from "vue";
import { findExistingConnectionOrError, useExistingConnection } from "@/system/connection";
import Scroll from "@/views/containers/Scroll.vue";
import { ScrollbarWidth } from "@/utils/layout";
import { useElementSize } from "@vueuse/core";
import Text from "@/views/content/Text.vue";
import { IconInline, getNodeIcon, makeIcon } from "@/system/icon";
import { emptyText, isTextEmpty } from "@/system/text";
import { dtToTs, formatDurationFromNow, tsToDt } from "@/utils/time";
import { user } from "@/system/user";
import { DateTime } from "luxon";

const HEADER_HEIGHT = 40;
const MAX_WIDTH = 800;
const MIN_INPUT_HEIGHT = 40;
const MIN_GUTTER_WIDTH = 12;
const ASIDE_WIDTH = 36;

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; size: Required<Pick<BoxData, "width" | "height">> } & Partial<
    Pick<ViewData, "title" | "nodePtr" | "variant">
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = makeViewId(props);

const maxInputHeight = computed(() => (props.size.height - HEADER_HEIGHT) / 2);
const inputRef: Ref<HTMLDivElement | null> = ref(null);
const inputSize = useElementSize(inputRef);
const textRef: Ref<InstanceType<typeof Text> | null> = ref(null);

const threadPtr = toRef(props, "nodePtr") as Ref<TypedNodeReferenceData<NodeType.MESSAGE> | undefined>;
const { graph: spaceGraph, connection: spaceConnection } = useExistingConnection(self);
const preparedPkgConnection = useExistingConnection(threadPtr);
const { graph: pkgGraph, connection: pkgConnection } = preparedPkgConnection;
const thread = pkgGraph.getRef(threadPtr);
const messages = pkgGraph.getChildrenRef(thread, NodeType.MESSAGE);
const ancestors = pkgGraph.getAncestorsRef(threadPtr, { includeSelf: false });
const context = computed(() => ancestors.value.find((n) => n.metatype != ObjectType.MESSAGE));

type RenderedMessage = {
  message: MessageData;
  isContinued: boolean;
  isContinuationBreak: boolean;
};
const renderedMessages = computed(() => {
  // collapse continued messages if they are from same author within 5 minutes
  const result: RenderedMessage[] = [];
  for (let i = 0; i < messages.value.length; i++) {
    const message = messages.value[i];
    const lastMessage = result[result.length - 1]?.message;
    const isContinued =
      lastMessage != null &&
      lastMessage.createdByPtr?.id == message.createdByPtr?.id &&
      message.createdAt!.seconds - lastMessage.createdAt!.seconds < 300;
    const isContinuationBreak = !isContinued && lastMessage != null;
    result.push({ message, isContinued, isContinuationBreak });
  }
  return result;
});

const text: Ref<TextData> = ref(emptyText());
const canSubmit = computed(() => !isTextEmpty(text.value));

/** Submits a message to the current thread. If it doesn't exist, create a root thread. */
function submit() {
  if (isTextEmpty(text.value)) return;
  if (pkg.value == null) throw new Error("no package");

  if (threadPtr.value == null) {
    // create new thread with message inside (in package)
    const rootPtr = toNodeReference(pkg.value);
    const tx = findExistingConnectionOrError("get", { roots: [rootPtr] }).tx;
    const selfView = spaceGraph.getOrError(self.value!);
    const thread = tx.create({
      metatype: NodeType.MESSAGE,
      parentPtr: rootPtr,
      packagePtr: rootPtr,
    });
    const message = tx.create({
      metatype: NodeType.MESSAGE,
      parentPtr: toNodeReference(thread),
      packagePtr: rootPtr,
      text: text.value,
    });
    spaceConnection.tx.update(selfView, { nodePtr: message.parentPtr });
  } else {
    // append to existing thread
    if (thread.value == null) throw new Error(`thread not found: ${describeNode(threadPtr.value)}`);
    pkgConnection.tx.create({
      metatype: NodeType.MESSAGE,
      parentPtr: threadPtr.value,
      packagePtr: thread.value.packagePtr!,
      text: text.value,
    });
  }

  // reset
  text.value = emptyText();
}

const isFocusedAbsolute = canvas.isFocusedAbsoluteRef(self);
canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id, variants: [Variant.PRIMARY, Variant.COMPACT] });
</script>
<template>
  <div class="flex h-full w-full flex-col bg-white">
    <!-- Header -->
    <div class="group w-full border-b border-gray-200">
      <div
        class="mx-auto flex w-full max-w-full flex-row items-center gap-x-3 pl-4 pr-5"
        :style="{ height: HEADER_HEIGHT + 'px', maxWidth: MAX_WIDTH + 'px' }"
      >
        <!-- Thread (local root message node) -->
        <div class="flex-shrink-0">
          <i
            class="fas fa-message mr-1.5 w-5 text-center"
            :class="[thread == null ? 'text-gray-500' : 'text-gray-700']"
          />
          <span
            class="truncate"
            :class="[thread == null ? 'text-gray-600' : 'text-gray-900', thread?.title != null ? 'font-semibold' : '']"
          >
            {{ thread?.title ?? "Untitled Thread" }}
          </span>
          <!-- Select thread -->
          <button class="ml-1.5 text-gray-400">
            <i class="fas fa-chevron-down" />
          </button>
        </div>
        <!-- Context (non-message parent node) -->
        <div class="ml-auto flex-shrink-0">
          <IconInline
            class="mr-1.5 w-5 text-center text-gray-400"
            v-bind="(context != null ? getNodeIcon(context) : null) ?? makeIcon('fas fa-infinity')"
          />
          <span class="text-gray-400">
            {{ (context as any)?.name ?? "Everything" }}
          </span>
          <!-- Select context node (move thread) -->
          <button class="ml-1.5 text-gray-400">
            <i class="fas fa-chevron-down" />
          </button>
        </div>
      </div>
    </div>

    <!-- Body -->
    <Scroll
      :size="{
        width: props.size.width,
        height: props.size.height - HEADER_HEIGHT - inputSize.height.value ?? MIN_INPUT_HEIGHT,
      }"
      :orientation="Orientation.VERTICAL"
      :track-width="ScrollbarWidth.md"
    >
      <!-- Messages -->
      <div v-if="thread" class="my-1">
        <template v-for="{ message, isContinued, isContinuationBreak } in renderedMessages" :key="message.id">
          <div
            class="group/message relative mx-auto flex max-w-full flex-row rounded border bg-white px-2 py-0.5 hover:bg-gray-50"
            :class="[
              nodePtr?.id == inspectionPtr?.id ? 'border-primary-900' : 'border-transparent',
              isContinuationBreak ? 'mt-2' : '',
            ]"
            :style="{ width: 'calc(100% - ' + MIN_GUTTER_WIDTH * 2 + 'px)', maxWidth: MAX_WIDTH + 'px' }"
          >
            <!-- Aside -->
            <!-- Author Icon -->
            <div
              v-if="!isContinued"
              class="mr-3.5 mt-0.5 h-fit flex-shrink-0 rounded border border-gray-200 bg-secondary-100 py-1 text-center text-gray-700"
              :style="{ width: ASIDE_WIDTH + 'px' }"
            >
              <!-- TODO :Broken: load correct icon for User/Block -->
              <IconInline
                v-bind="
                  message.createdByPtr?.id == user?.id
                    ? user!.icon ?? makeIcon('fas fa-user-tie')
                    : makeIcon('fas fa-robot')
                "
              />
            </div>
            <!-- Time (if continued) -->
            <div v-else class="mr-3.5 flex-shrink-0 px-0.5" :style="{ width: ASIDE_WIDTH + 'px' }">
              <span class="text-xs text-gray-400 opacity-0 group-hover/message:opacity-100">
                {{ tsToDt(message.createdAt!).toLocaleString(DateTime.TIME_24_SIMPLE) }}
              </span>
            </div>
            <!-- Body -->
            <div class="w-full">
              <!-- Header -->
              <div v-if="!isContinued" class="mb-0.5 max-w-full gap-x-0.5">
                <!-- Author Name / Time -->
                <span class="truncate font-medium text-gray-900">
                  {{ message.createdByPtr?.id == user?.id ? user!.name : "Bench" }}
                </span>
                <span class="ml-1.5 text-xs text-gray-400">{{ formatDurationFromNow(message.createdAt!) }}</span>
              </div>
              <!-- Content -->
              <Text :model-value="message.text" :variant="Variant.STEALTH" />
              <!-- Controls (floating) -->
              <div
                class="absolute -top-3 right-1.5 z-10 ml-auto flex flex-row gap-x-2 rounded border border-gray-200 bg-white px-2 py-1 opacity-0 group-hover/message:opacity-100"
              >
                <!-- Reply -->
                <button
                  v-tooltip="{ title: 'Reply', referenceMargin: 8, small: true, showDelay: 200, hideDelay: 100 }"
                  class="rounded text-gray-400 hover:bg-gray-100 hover:text-primary-900"
                >
                  <i class="fas fa-reply w-5 text-center" />
                </button>
                <!-- Thread -->
                <button
                  v-tooltip="{ title: 'Thread', referenceMargin: 8, small: true, showDelay: 200, hideDelay: 100 }"
                  class="rounded text-gray-400 hover:bg-gray-100 hover:text-primary-900"
                >
                  <i class="fas fa-reel w-5 text-center" />
                </button>
                <!-- Menu -->
                <button
                  class="rounded text-gray-400 hover:bg-gray-100 hover:text-primary-900 data-[popover=true]:border-primary-900"
                >
                  <i class="fas fa-ellipsis-v w-5 text-center" />
                </button>
              </div>
            </div>
          </div>
        </template>
      </div>
      <!-- Nothing here yet -->
      <div v-else-if="threadPtr">nocheckin: no messages (select thread?)</div>
    </Scroll>

    <!-- Create message -->
    <div ref="inputRef" class="group mt-auto border-t border-gray-200" @click="textRef?.focus">
      <div
        ref="inputRef"
        class="mx-auto flex flex-row items-end"
        :class="[variant != Variant.COMPACT ? 'gap-x-2.5 px-3 py-3' : 'gap-x-1.5 px-2.5 py-1.5']"
        :style="{ maxWidth: MAX_WIDTH + 'px', minHeight: MIN_INPUT_HEIGHT + 'px', maxHeight: maxInputHeight + 'px' }"
      >
        <!-- Upload -->
        <button
          class="h-fit self-end rounded border-gray-200 bg-white px-2 py-0.5 text-base hover:bg-gray-100"
          :class="
            isFocusedAbsolute ? 'text-gray-600 hover:text-primary-900' : 'text-gray-400 group-hover:text-gray-500'
          "
          @click.stop="() => {}"
        >
          <i class="fas fa-plus" />
        </button>
        <!-- Content -->
        <Text
          ref="textRef"
          v-model="text"
          class="my-0.5 w-full self-end hover:cursor-text"
          :variant="Variant.STEALTH"
          is-input
          :placeholder="`Message your Bench`"
          suppress-enter
          @click.stop
          @keydown.enter.stop="submit"
        />
        <!-- Submit -->
        <button
          class="h-fit self-end rounded border-gray-200 bg-white px-2 py-0.5 text-base hover:bg-gray-100"
          :class="
            isFocusedAbsolute ? 'text-gray-600 hover:text-primary-900' : 'text-gray-400 group-hover:text-gray-500'
          "
          :disabled="!canSubmit"
          @click.stop="() => {}"
        >
          <i class="fas fa-arrow-up" />
        </button>
      </div>
    </div>
  </div>
</template>
