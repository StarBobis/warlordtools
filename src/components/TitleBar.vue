<script setup lang="ts">
import { getCurrentWindow } from '@tauri-apps/api/window';

defineProps<{
  currentView: string;
  isSettingsOpen: boolean;
}>();

const emit = defineEmits<{
  (e: 'update:currentView', view: string): void;
  (e: 'toggleSettings'): void;
}>();

const appWindow = getCurrentWindow();

const minimize = () => appWindow.minimize();
const maximize = async () => {
    if (await appWindow.isMaximized()) {
        appWindow.unmaximize();
    } else {
        appWindow.maximize();
    }
};
const close = () => appWindow.close();
</script>

<template>
  <div class="title-bar" data-tauri-drag-region>
    <!-- Left Navigation (Tabs) -->
    <div class="nav-tabs">
      <div 
        class="tab-item" 
        :class="{ active: currentView === 'filter' }"
        @click="emit('update:currentView', 'filter')"
      >
        <span class="tab-text">过滤器</span>
        <div class="active-indicator"></div>
      </div>
      <div 
        class="tab-item" 
        :class="{ active: currentView === 'market' }"
        @click="emit('update:currentView', 'market')"
      >
        <span class="tab-text">市集</span>
        <div class="active-indicator"></div>
      </div>
    </div>

    <!-- Right Controls -->
    <div class="window-controls">
      <!-- Settings Button -->
      <div 
        class="caption-btn settings-btn" 
        :class="{ active: isSettingsOpen }"
        @click="emit('toggleSettings')"
        title="Settings"
      >
        <el-icon :size="16"><Setting /></el-icon>
      </div>

      <!-- Native-like Window Controls -->
      <div class="caption-btn" @click="minimize" title="Minimize">
         <el-icon :size="16"><Minus /></el-icon>
      </div>

      <div class="caption-btn" @click="maximize" title="Maximize">
         <el-icon :size="16"><FullScreen /></el-icon>
      </div>

      <div class="caption-btn close-btn" @click="close" title="Close">
         <el-icon :size="18"><Close /></el-icon>
      </div>
    </div>
  </div>
</template>

<style scoped>
.title-bar {
  height: 40px;
  /* Slightly denser glass for better foreground contrast */
  background: rgba(18, 18, 24, 0.78);
  backdrop-filter: blur(12px);
  -webkit-backdrop-filter: blur(12px);
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.08);

  display: flex;
  justify-content: space-between;
  align-items: center;
  user-select: none;
  font-family: "Segoe UI", sans-serif;
  border-bottom: 1px solid rgba(255, 255, 255, 0.12);
}

/* --- Tabs --- */
.nav-tabs {
  display: flex;
  height: 100%;
  padding-left: 10px;
  -webkit-app-region: no-drag;
}

.tab-item {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0 16px;
  cursor: pointer;
  
  /* Text Color Logic */
  color: rgba(255, 255, 255, 0.6);
  font-size: 13px;
  font-weight: 500;
  transition: all 0.2s;
  height: 100%;
}

.tab-item:hover {
  background-color: rgba(255, 255, 255, 0.05);
  color: rgba(255, 255, 255, 0.9);
}

.tab-item.active {
  color: #fff;
  background-color: rgba(255, 255, 255, 0.1);
  text-shadow: 0 0 10px rgba(255, 255, 255, 0.3); /* Subtle glow for active text */
}

.active-indicator {
  position: absolute;
  bottom: 0;
  left: 0;
  width: 100%;
  height: 2px;
  background-color: var(--el-color-primary);
  box-shadow: 0 -2px 6px var(--el-color-primary); /* Glow effect for the line */
  transform: scaleX(0);
  transition: transform 0.2s cubic-bezier(0.25, 0.8, 0.25, 1);
}

.tab-item.active .active-indicator {
  transform: scaleX(1);
}

/* --- Window Controls --- */
.window-controls {
  display: flex;
  height: 100%;
  -webkit-app-region: no-drag;
}

.caption-btn {
  width: 46px;
  height: 100%;
  display: flex;
  justify-content: center;
  align-items: center;
  cursor: default;
  
  /* Icon Color Configuration */
  color: rgba(255, 255, 255, 0.9); /* Brighter base for dark glass */
  
  transition: background-color 0.15s, color 0.15s;
}

/* Force el-icon to use inherit color */
.caption-btn :deep(.el-icon) {
    color: inherit;
}

.caption-btn:hover {
  background-color: rgba(255, 255, 255, 0.14);
  color: #ffffff; /* Bright white on hover */
}

/* Close Button Special Handling */
.caption-btn.close-btn:hover {
  background-color: #c42b1c; /* Windows 11 Close Red */
  box-shadow: inset 0 0 10px rgba(0,0,0,0.25); /* Slight depth */
  color: #ffffff;
}

/* Settings Button */
.settings-btn {
  border-left: 1px solid rgba(255, 255, 255, 0.08); /* Slightly stronger separator */
}

.settings-btn.active {
  background-color: rgba(255, 255, 255, 0.18);
  color: var(--el-color-primary-light-3); /* Active state matches theme */
}
</style>
