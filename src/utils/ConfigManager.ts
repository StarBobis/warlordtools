import { BaseDirectory, exists, mkdir, readTextFile, writeTextFile } from '@tauri-apps/plugin-fs';
import { reactive } from 'vue';

export interface AppSettings {
    width: number;
    height: number;
    x?: number;
    y?: number;
    maximized: boolean;
    filterStoragePath?: string;
    // last selected filter file path (persist UI state)
    lastSelectedFilter?: string;
    // Navigation items order
    navOrder?: string[];
    // Background Settings
    backgroundType?: 'default' | 'image' | 'video';
    backgroundPath?: string;
    backgroundVolume?: number; // 0-100 for video volume
}

const CONFIG_DIR = 'WarlordToolsConfig';
const CONFIG_FILE = 'Settings.json';

// Default settings
const DEFAULT_SETTINGS: AppSettings = {
    width: 1280,
    height: 720,
    maximized: false,
    filterStoragePath: '',
    navOrder: ['filter', 'market', 'workshop', 'poedb'],
    backgroundType: 'default',
    backgroundPath: '',
    backgroundVolume: 0
};

class ConfigManager {
    private settings = reactive<AppSettings>({ ...DEFAULT_SETTINGS });
    private initialized = false;

    async init() {
        if (this.initialized) return;
        try {
            // Get LocalAppData path
            // In Tauri v2 with path plugin, we can use BaseDirectory.LocalData directly with fs calls
            // or resolve the path manually.
            // Let's rely on BaseDirectory.LocalData which maps to %LOCALAPPDATA% on Windows.
            
            // Check if directory exists
            const dirExists = await exists(CONFIG_DIR, { baseDir: BaseDirectory.LocalData });
            if (!dirExists) {
                await mkdir(CONFIG_DIR, { baseDir: BaseDirectory.LocalData, recursive: true });
            }

            // Check if file exists
            // const configPath = await join(CONFIG_DIR, CONFIG_FILE); 
            // Note: 'exists' and 'readTextFile' take the relative path if baseDir is provided.
            // But we can just pass the path relative to baseDir.

            const fileExists = await exists(`${CONFIG_DIR}/${CONFIG_FILE}`, { baseDir: BaseDirectory.LocalData });
            
            if (fileExists) {
               const content = await readTextFile(`${CONFIG_DIR}/${CONFIG_FILE}`, { baseDir: BaseDirectory.LocalData });
               Object.assign(this.settings, { ...DEFAULT_SETTINGS, ...JSON.parse(content) });
            } else {
               await this.saveSettings();
            }
        } catch (e) {
            console.error('Failed to initialize config:', e);
            // Fallback to defaults
        }
        this.initialized = true;
    }

    getSettings(): AppSettings {
        return this.settings;
    }

    async saveSettings(newSettings?: Partial<AppSettings>) {
        if (newSettings) {
            Object.assign(this.settings, newSettings);
        }
        
        try {
            await writeTextFile(
                `${CONFIG_DIR}/${CONFIG_FILE}`, 
                JSON.stringify(this.settings, null, 2), 
                { baseDir: BaseDirectory.LocalData }
            );
        } catch (e) {
            console.error('Failed to save settings:', e);
        }
    }
}

export const configManager = new ConfigManager();
