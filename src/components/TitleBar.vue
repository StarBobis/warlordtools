<script setup lang="ts">
import { getCurrentWindow } from '@tauri-apps/api/window';

const { currentView } = defineProps<{
  currentView: string;
}>();

const emit = defineEmits<{
  (e: 'update:currentView', view: string): void;
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
const closeWindow = async () => {
  console.log('Close button clicked');
  await appWindow.close();
};
</script>

<template>
  <div class="title-bar-container">
    <div class="nav-section">
      <div class="nav-tab" :class="{ active: currentView === 'filter' }" @click="emit('update:currentView', 'filter')">
        <span>过滤器</span>
        <div class="tab-indicator"></div>
      </div>
      <div class="nav-tab" :class="{ active: currentView === 'market' }" @click="emit('update:currentView', 'market')">
        <span>市集</span>
        <div class="tab-indicator"></div>
      </div>
    </div>

    <div class="controls-section">
      <div class="control-button settings-button" :class="{ active: currentView === 'settings' }" @click="emit('update:currentView', 'settings')" title="Settings">
        <span class="gear-icon" aria-hidden="true"></span>
      </div>
      <div class="window-buttons-group">
        <div class="control-button win-btn" @click.stop.prevent="minimize" title="Minimize">
          <svg aria-hidden="true" width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round">
            <path d="M1.2 5h7.6" />
          </svg>
        </div>
        <div class="control-button win-btn" @click.stop.prevent="maximize" title="Maximize">
          <svg aria-hidden="true" width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round">
            <rect x="1.5" y="1.5" width="7" height="7" rx="1" />
          </svg>
        </div>
        <div class="control-button win-btn close-btn" @click.stop.prevent="closeWindow" title="Close">
          <svg aria-hidden="true" width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round">
            <path d="M2 2l6 6M8 2l-6 6" />
          </svg>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.title-bar-container {
  height: 34px;
  width: 100%;
  display: flex;
  justify-content: space-between;
  align-items: center;
  background: rgba(0, 0, 0, 0.2);
  backdrop-filter: blur(4px);
  user-select: none;
  -webkit-app-region: drag;
  border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  box-sizing: border-box;
}

.nav-section {
  display: flex;
  height: 100%;
  padding-left: 8px;
  -webkit-app-region: no-drag;
}

.nav-tab {
  height: 100%;
  padding: 0 12px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 14px;
  color: #cccccc;
  position: relative;
  cursor: pointer;
  transition: color 0.2s, background-color 0.2s;
}

.nav-tab:hover {
  background-color: rgba(255, 255, 255, 0.1);
  color: #ffffff;
}

.nav-tab.active {
  color: #ffffff;
  font-weight: bold;
}

.tab-indicator {
  position: absolute;
  bottom: 0;
  left: 0;
  width: 100%;
  height: 3px;
  background-color: #409eff;
  transform: scaleX(0);
  transition: transform 0.2s;
}

.nav-tab.active .tab-indicator {
  transform: scaleX(1);
}

.controls-section {
  display: flex;
  height: 100%;
  align-items: center;
  -webkit-app-region: no-drag;
  --btn-size: 40px;
  --settings-btn-size: 34px;
  --icon-color: #f5f5f7;
  --icon-opacity: 0.82;
  --hover-bg: rgba(255, 255, 255, 0.08);
  --pressed-bg: rgba(255, 255, 255, 0.16);
  --close-hover-bg: #f1707a;
  --close-pressed-bg: #d13438;
  --anim: 120ms ease;
}

.control-button {
  height: 100%;
  display: flex;
  justify-content: center;
  align-items: center;
  cursor: pointer;
  transition: background-color var(--anim), color var(--anim), opacity var(--anim);
  -webkit-app-region: no-drag;
}

.settings-button {
  width: var(--settings-btn-size);
  height: 100%;
  margin-right: 6px;
  border-radius: 4px;
  transition: background-color var(--anim), opacity var(--anim);
}

.gear-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 16px;
  height: 16px;
  color: var(--icon-color);
  opacity: var(--icon-opacity);
  transition: opacity var(--anim), color var(--anim);
  font-family: "Segoe Fluent Icons", "Segoe MDL2 Assets", "Segoe UI Symbol", sans-serif;
  font-size: 16px;
  line-height: 1;
}

.gear-icon::before {
  content: "\E713"; /* Fluent/MDL2 settings gear */
}

.settings-button:hover .gear-icon,
.settings-button.active .gear-icon {
  opacity: 1;
  color: #ffffff;
}

.settings-button:hover {
  background-color: var(--hover-bg);
}

.settings-button.active {
  background-color: rgba(64, 158, 255, 0.4);
}

.window-buttons-group {
  height: 100%;
  display: flex;
  -webkit-app-region: no-drag;
}

.win-btn {
  width: var(--btn-size);
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: background-color var(--anim), color var(--anim), opacity var(--anim);
  -webkit-app-region: no-drag;
}

.win-btn svg {
  width: 10px;
  height: 10px;
  color: var(--icon-color);
  stroke: var(--icon-color);
  fill: none;
  opacity: var(--icon-opacity);
}

.win-btn:hover {
  background-color: var(--hover-bg);
}

.win-btn:hover svg {
  opacity: 1;
}

.close-btn:hover {
  background-color: var(--close-hover-bg) !important;
}

.close-btn:hover svg {
  color: #ffffff !important;
  stroke: #ffffff !important;
}

.win-btn:active {
  background-color: var(--pressed-bg);
}

.close-btn:active {
  background-color: var(--close-pressed-bg) !important;
}
</style>
