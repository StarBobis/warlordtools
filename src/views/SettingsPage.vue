<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import { invoke } from '@tauri-apps/api/core';
import { getVersion } from '@tauri-apps/api/app';
import { configManager } from '../utils/ConfigManager';

const appVersion = ref('1.0.0');
const filterPath = ref(configManager.getSettings().filterStoragePath || '');

onMounted(async () => {
  try {
    appVersion.value = await getVersion();
  } catch (e) {
    console.error('Failed to get version:', e);
  }
});

const selectFolder = async () => {
  try {
    const selected = await openDialog({
      directory: true,
      multiple: false,
      title: '选择过滤器文件夹',
      defaultPath: filterPath.value || undefined,
    });

    if (selected) {
      const path = Array.isArray(selected) ? selected[0] : selected;
      if (path) {
        filterPath.value = path;
        await saveSettings();
      }
    }
  } catch (e) {
    console.error('Dialog failed', e);
  }
};

const saveSettings = async () => {
  await configManager.saveSettings({
    filterStoragePath: filterPath.value,
  });
};

const openContainingFolder = async () => {
  try {
    if (!filterPath.value) return;
    await invoke('open_folder_cmd', { path: filterPath.value });
  } catch (e) {
    console.error('Failed to open folder', e);
  }
};
</script>

<template>
  <div class="settings-page">
    <div class="settings-layout">
      <!-- Left: Main Settings Area -->
      <div class="settings-main">
        <div class="settings-section">
          <div class="section-title">
            <span class="title-text">基础设置</span>
            <span class="title-desc">存储路径配置</span>
          </div>
          <div class="glass-card">
              <div class="form-item">
                <label class="form-label">过滤器存储路径</label>
                <div class="path-selector">
                  <input
                    v-model="filterPath"
                    placeholder="请选择过滤器存储路径"
                    class="glass-input"
                    @change="saveSettings"
                  />
                </div>

                <div class="button-row">
                  <button class="glass-button" @click="selectFolder">更改目录</button>
                  <button class="glass-button" @click="openContainingFolder">打开所在文件夹</button>
                </div>
              </div>
          </div>
        </div>
      </div>

      <!-- Right: About Info Area -->
      <div class="settings-sidebar">
         <div class="about-container">
             <div class="app-logo-mini">
                <img src="/tauri.svg" class="logo-image-mini" alt="App Logo" />
             </div>
             <div class="app-info-mini">
               <h2 class="app-name-mini">Warlord Tools</h2>
               <span class="app-version-mini">v{{ appVersion }}</span>
             </div>
             
             <div class="divider-mini"></div>
             
             <div class="link-group-mini">
                <a href="#" class="link-item-mini">
                  GitHub地址
                </a>
                <a href="#" class="link-item-mini">
                  提交问题反馈
                </a>
             </div>

             <div class="copyright-mini">
                MIT License<br>&copy; 2026 Warlord Team
             </div>
         </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
/* Reset box-sizing for standard elements just in case */
*, *::before, *::after {
  box-sizing: border-box;
}

/* Overall Layout Container */
.settings-page {
  height: 100%;
  width: 100%;
  padding: 40px 60px;
  overflow-y: auto;
}

.settings-layout {
  display: flex;
  gap: 60px;
  max-width: 1400px;
  margin: 0 auto;
  align-items: flex-start;
}

/* Left Content */
.settings-main {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 32px;
  min-width: 0;
}

.settings-section {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

/* Titles */
.section-title {
  display: flex;
  flex-direction: column;
  margin-left: 8px;
  margin-bottom: 8px;
}

.title-text {
  font-size: 24px;
  font-weight: 300;
  color: #fff;
  letter-spacing: 1px;
}

.title-desc {
  font-size: 14px;
  color: rgba(255, 255, 255, 0.4);
  margin-top: 6px;
}

/* Glass Card */
.glass-card {
  background-color: rgba(255, 255, 255, 0.03);
  backdrop-filter: blur(20px);
  -webkit-backdrop-filter: blur(20px);
  border: 1px solid rgba(255, 255, 255, 0.05);
  border-radius: 20px;
  padding: 32px;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.1);
  transition: all 0.3s cubic-bezier(0.25, 0.8, 0.25, 1);
  /* Force compositing and hint the browser to prepare backdrop-filter
     to avoid a delayed first-frame rendering of the glass effect. */
  will-change: backdrop-filter, transform;
  transform: translateZ(0);
  -webkit-transform: translate3d(0,0,0);
  backface-visibility: hidden;
}

.glass-card:hover {
  background-color: rgba(255, 255, 255, 0.06);
  border-color: rgba(255, 255, 255, 0.1);
  transform: translateY(-2px);
  box-shadow: 0 12px 48px rgba(0, 0, 0, 0.15);
}

/* Right Sidebar */
.settings-sidebar {
  width: 160px;
  flex-shrink: 0;
  padding-top: 10px;
}

.about-container {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  opacity: 0.6;
  transition: opacity 0.3s ease;
}

.about-container:hover {
  opacity: 1;
}

.app-logo-mini {
  margin-bottom: 12px;
}
.logo-image-mini {
  width: 56px;
  height: 56px;
  border-radius: 14px;
  box-shadow: 0 4px 12px rgba(0,0,0,0.2);
}

.app-info-mini {
  display: flex;
  flex-direction: column;
}

.app-name-mini {
  margin: 0;
  font-size: 20px;
  font-weight: 500;
  color: #fff;
  letter-spacing: 0.5px;
}

.app-version-mini {
  font-size: 13px;
  color: rgba(255, 255, 255, 0.5);
  font-family: monospace;
}

.divider-mini {
  width: 24px;
  height: 2px;
  background: rgba(255, 255, 255, 0.2);
  margin: 24px 0;
}

.link-group-mini {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.link-item-mini {
  font-size: 14px;
  color: rgba(255, 255, 255, 0.6);
  text-decoration: none;
  display: flex;
  align-items: center;
  gap: 10px;
  transition: color 0.2s;
}

.link-item-mini:hover {
  color: #409eff;
}

.copyright-mini {
  margin-top: 40px;
  font-size: 12px;
  color: rgba(255, 255, 255, 0.3);
  line-height: 1.6;
}

/* Custom Form Styles (Replacing Element Plus) */
.form-item {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.form-label {
  font-size: 15px;
  color: rgba(255, 255, 255, 0.8);
  font-weight: 500;
  padding-left: 4px;
}

.path-selector {
  display: flex;
  gap: 12px;
  width: 100%;
  align-items: center;
}

.button-row {
  display: flex;
  gap: 12px;
  margin-top: 8px;
}

/* Standard Input Styling to match Glass feel */
.glass-input {
  /* Flex behavior */
  flex: 1; /* Take remaining space */
  width: 0; /* Important for flex items to shrink properly */
  
  /* Visuals */
  background-color: rgba(0, 0, 0, 0.2);
  box-shadow: 0 0 0 1px rgba(255, 255, 255, 0.1) inset;
  border: none;
  border-radius: 8px;
  padding: 0 16px;
  height: 48px;
  color: #fff;
  font-family: inherit;
  font-size: 15px;
  outline: none;
  transition: all 0.3s ease;
}

.glass-input:hover {
  background-color: rgba(0, 0, 0, 0.3);
  box-shadow: 0 0 0 1px rgba(255, 255, 255, 0.25) inset;
}

.glass-input:focus {
  background-color: rgba(0, 0, 0, 0.4);
  box-shadow: 0 0 0 1px rgba(64, 158, 255, 0.6) inset, 0 0 12px rgba(64, 158, 255, 0.2);
}

.glass-input::placeholder {
  color: rgba(255, 255, 255, 0.4);
}

/* Standard Button Styling */
.glass-button {
  /* Visuals */
  background-color: rgba(255, 255, 255, 0.08);
  border: 1px solid rgba(255, 255, 255, 0.15);
  color: #fff;
  border-radius: 8px;
  height: 48px;
  padding: 0 24px;
  font-weight: 500;
  font-size: 14px;
  cursor: pointer;
  transition: all 0.3s;
  white-space: nowrap; /* Prevent text wrapping */
}

.glass-button:hover {
  background-color: rgba(255, 255, 255, 0.15);
  border-color: rgba(255, 255, 255, 0.3);
  transform: translateY(-1px);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.2);
}

.glass-button:active {
  transform: translateY(0);
  background-color: rgba(255, 255, 255, 0.05);
}
</style>
