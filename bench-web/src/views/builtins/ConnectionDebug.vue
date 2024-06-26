<script lang="ts" setup>
import { Orientation } from "@/proto/wire";
import { isDeveloperMode } from "@/system/client";
import { connections, hasPendingConnections } from "@/system/connection";
import { ScrollbarWidth } from "@/utils/layout";
import Scroll from "@/views/containers/Scroll.vue";
import Button from "@/views/controls/Button.vue";
import Popover from "@/views/builtins/Popover.vue";
</script>
<template>
  <!-- Connection -->
  <Popover placement="bottom" :reference-margin="8" :container-margin="4">
    <template #trigger="{ toggle }">
      <button
        v-if="isDeveloperMode || hasPendingConnections"
        class="select-none rounded border-2 px-1 py-1 transition-colors"
        :class="
          connections.some((c) => c.isPaused.value || c.txBuffer.isPaused.value)
            ? 'border-warning-600'
            : 'border-transparent'
        "
        @click.stop="toggle"
      >
        <i
          class="fas"
          :class="
            !hasPendingConnections
              ? 'fa-cloud text-gray-700 hover:text-primary-900'
              : 'fa-cloud-slash text-warning-600 hover:text-primary-700'
          "
        />
      </button>
    </template>
    <template #content="{ close }">
      <!-- Connection summary -->
      <!-- will probably move this to a Connections View (maybe keep summary on hover) -->
      <div v-outside.click.stop="close" class="p z-50 rounded border border-gray-300 bg-white text-gray-900">
        <div class="my-1 border-b border-gray-300 px-3 py-1">
          <span class="font-semibold">Connections ({{ connections.length }})</span>
        </div>
        <Scroll
          :size="{ width: 480, height: 400 }"
          size-is-dynamic
          :orientation="Orientation.VERTICAL"
          :track-width="ScrollbarWidth.sm"
        >
          <ul class="my-1.5 flex min-w-[480px] flex-col gap-y-1 px-3">
            <li v-for="connection in connections" :key="connection.id" class="flex flex-row py-1">
              <!-- Metadata -->
              <span class="h-fit rounded bg-secondary-100 px-2 font-semibold text-secondary-900">
                {{ connection.kind }}
              </span>
              <span class="ml-2 font-semibold truncate">{{ connection.name }}</span>
              <span class="ml-2 text-gray-500">#{{ connection.id }}</span>
              <!-- Status -->
              <span class="ml-auto flex flex-row pl-4 align-top">
                <span class="mr-2" :class="connection.referenceCount > 0 ? '' : 'text-gray-500'">
                  {{ connection.referenceCount }}
                </span>
                <!-- Connected (status) -->
                <span class="rounded px-1">
                  <i
                    class="fas"
                    :class="
                      connection.isConnected.value
                        ? 'fa-check text-success-600'
                        : 'fa-exclamation-circle text-warning-600'
                    "
                  />
                </span>
                <!-- Down (status & toggle) -->
                <button class="rounded px-1 hover:bg-primary-200" @click="connection.togglePaused()">
                  <i
                    :class="
                      connection.isFetching.value
                        ? 'fas fa-spinner-third animate-spin text-gray-500'
                        : connection.isLive && !connection.isPaused.value
                          ? 'fas fa-down text-success-600'
                          : 'fas fa-down text-secondary-500'
                    "
                  />
                </button>
                <!-- Up (toggle) -->
                <button class="rounded px-1 hover:bg-primary-200" @click="connection.txBuffer.togglePaused()">
                  <i
                    class="fas fa-up"
                    :class="connection.txBuffer.isPaused.value ? 'text-secondary-500' : 'text-success-600'"
                  />
                </button>
              </span>
            </li>
          </ul>
        </Scroll>
      </div>
    </template>
  </Popover>
</template>
