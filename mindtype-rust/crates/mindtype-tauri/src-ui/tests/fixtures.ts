import { test as base, expect, Page } from '@playwright/test';

/**
 * Mock Tauri API responses for E2E testing.
 */

interface MockSettings {
  language: string;
  model_id: string;
  hotkey: string;
  setup_completed: boolean;
}

interface MockAppState {
  settings: MockSettings;
  recording_state: 'idle' | 'recording' | 'transcribing' | 'inserting';
  setup_completed: boolean;
}

interface TauriMocks {
  appState: MockAppState;
  updateState: (partial: Partial<MockAppState>) => void;
  updateSettings: (partial: Partial<MockSettings>) => void;
}

// Setup Tauri mocks on a page
async function setupTauriMocks(page: Page, state: MockAppState) {
  await page.addInitScript((initialState) => {
    (window as any).__MOCK_STATE__ = initialState;

    (window as any).__TAURI_INTERNALS__ = {
      invoke: async (cmd: string, args?: any) => {
        const s = (window as any).__MOCK_STATE__;

        switch (cmd) {
          case 'get_app_state':
            return s;
          case 'start_recording':
            s.recording_state = 'recording';
            return null;
          case 'stop_recording':
            s.recording_state = 'transcribing';
            return new Array(16000).fill(0);
          case 'transcribe':
            s.recording_state = 'inserting';
            setTimeout(() => { s.recording_state = 'idle'; }, 100);
            return { text: 'Hello world', segments: [] };
          case 'save_settings':
            Object.assign(s.settings, args?.settings || {});
            return null;
          case 'set_language':
            s.settings.language = args?.lang || 'en';
            return null;
          case 'complete_setup':
            s.setup_completed = true;
            s.settings.setup_completed = true;
            return null;
          case 'get_recent_transcriptions':
            return [];
          case 'get_license_status':
            return { status: 'trial', days_left: 7, minutes_left: 30.0 };
          case 'activate_license':
          case 'deactivate_license':
            return { success: true };
          case 'validate_license':
            return true;
          case 'get_available_models':
            return [
              { id: 'tiny', name: 'Tiny (75MB)', size: 75000000 },
              { id: 'base', name: 'Base (140MB)', size: 140000000 },
              { id: 'small', name: 'Small (460MB)', size: 460000000 },
            ];
          case 'download_model':
          case 'set_llm_config':
            return null;
          case 'get_llm_providers':
            return [
              { id: 'mindtype_cloud', name: 'MindType Cloud', requires_key: false },
              { id: 'openai', name: 'OpenAI', requires_key: true },
            ];
          case 'add_files_to_queue':
            return { queued: args?.paths?.length || 0 };
          case 'get_audio_devices':
            return [{ id: 'default', name: 'Default Microphone' }];
          default:
            return null;
        }
      },
      transformCallback: () => {},
    };

    // Event listener support
    const listeners = new Map<string, Function[]>();
    (window as any).__mockEmitEvent = (event: string, payload: any) => {
      (listeners.get(event) || []).forEach((h) => h({ payload }));
    };
    (window as any).__TAURI_LISTEN__ = async (event: string, handler: Function) => {
      if (!listeners.has(event)) listeners.set(event, []);
      listeners.get(event)!.push(handler);
      return () => {
        const arr = listeners.get(event);
        if (arr) arr.splice(arr.indexOf(handler), 1);
      };
    };
  }, state);
}

// Test with main window (setup completed)
export const testMainWindow = base.extend<{ tauriMocks: TauriMocks }>({
  tauriMocks: async ({ page }, use) => {
    const state: MockAppState = {
      settings: { language: 'en', model_id: 'tiny', hotkey: 'ctrl+shift+space', setup_completed: true },
      recording_state: 'idle',
      setup_completed: true,
    };
    await setupTauriMocks(page, state);
    await use({
      appState: state,
      updateState: (p) => Object.assign(state, p),
      updateSettings: (p) => Object.assign(state.settings, p),
    });
  },
});

// Test with wizard (setup not completed)
export const testWizard = base.extend<{ tauriMocks: TauriMocks }>({
  tauriMocks: async ({ page }, use) => {
    const state: MockAppState = {
      settings: { language: 'en', model_id: 'tiny', hotkey: 'ctrl+shift+space', setup_completed: false },
      recording_state: 'idle',
      setup_completed: false,
    };
    await setupTauriMocks(page, state);
    await use({
      appState: state,
      updateState: (p) => Object.assign(state, p),
      updateSettings: (p) => Object.assign(state.settings, p),
    });
  },
});

// Legacy export
export const test = testMainWindow;

export { expect };

// Wait for app load
export async function waitForAppLoad(page: Page) {
  await page.waitForSelector('.main, .wizard', { timeout: 10000 });
}

// Emit mock Tauri event
export async function emitTauriEvent(page: Page, event: string, payload: any) {
  await page.evaluate(([e, p]) => (window as any).__mockEmitEvent?.(e, p), [event, payload]);
}
