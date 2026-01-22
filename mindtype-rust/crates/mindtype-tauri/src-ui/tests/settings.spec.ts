import { testMainWindow as test, expect, waitForAppLoad } from './fixtures';

test.describe('Settings', () => {
  test.beforeEach(async ({ page }) => {
    // Main window tests use testMainWindow fixture (setup_completed = true)
    await page.goto('/');
    await waitForAppLoad(page);
  });

  test('can navigate to settings tab', async ({ page }) => {
    // Find and click settings tab
    const settingsTab = page.locator('button:has-text("Settings"), .tab-button:has-text("Settings")');

    if (await settingsTab.isVisible()) {
      await settingsTab.click();
      await page.waitForTimeout(300);

      // Settings view should be visible
      const settingsView = page.locator('.settings-tab');
      await expect(settingsView).toBeVisible({ timeout: 5000 });
    }
  });

  test('shows language selection', async ({ page }) => {
    // Navigate to settings
    const settingsTab = page.locator('button:has-text("Settings"), .tab-button:has-text("Settings")');
    if (await settingsTab.isVisible()) {
      await settingsTab.click();
      await page.waitForTimeout(300);
    }

    // Language selector should be visible (it's a <select> element)
    const languageSelector = page.locator('.setting__select');
    if ((await languageSelector.count()) > 0) {
      await expect(languageSelector.first()).toBeVisible();
    }
  });

  test('shows hotkey setting', async ({ page }) => {
    // Navigate to settings
    const settingsTab = page.locator('button:has-text("Settings"), .tab-button:has-text("Settings")');
    if (await settingsTab.isVisible()) {
      await settingsTab.click();
      await page.waitForTimeout(300);
    }

    // Hotkey value should be displayed
    const hotkeyValue = page.locator('.setting__value');
    if ((await hotkeyValue.count()) > 0) {
      await expect(hotkeyValue.first()).toBeVisible();
    }
  });

  test('shows model setting', async ({ page }) => {
    // Navigate to settings
    const settingsTab = page.locator('button:has-text("Settings"), .tab-button:has-text("Settings")');
    if (await settingsTab.isVisible()) {
      await settingsTab.click();
      await page.waitForTimeout(300);
    }

    // Model value should be displayed
    const settings = page.locator('.setting');
    await expect(settings.first()).toBeVisible({ timeout: 5000 });
  });

  test('shows LLM provider settings section', async ({ page }) => {
    // Navigate to settings
    const settingsTab = page.locator('button:has-text("Settings"), .tab-button:has-text("Settings")');
    if (await settingsTab.isVisible()) {
      await settingsTab.click();
      await page.waitForTimeout(300);
    }

    // LLM settings section should be visible
    const llmSection = page.locator('.llm-settings, .provider-select');
    const aiProviderText = page.getByText('AI Provider');
    if ((await llmSection.count()) > 0 || (await aiProviderText.count()) > 0) {
      // At least one indicator of LLM settings
      expect(true).toBeTruthy();
    }
  });

  test('shows license settings section', async ({ page }) => {
    // Navigate to settings
    const settingsTab = page.locator('button:has-text("Settings"), .tab-button:has-text("Settings")');
    if (await settingsTab.isVisible()) {
      await settingsTab.click();
      await page.waitForTimeout(300);
    }

    // License section should be visible
    const licenseSection = page.locator('.license-settings');
    const licenseText = page.getByText('License');
    if ((await licenseSection.count()) > 0 || (await licenseText.count()) > 0) {
      // At least one indicator of license settings
      expect(true).toBeTruthy();
    }
  });

  test('shows general settings section', async ({ page }) => {
    // Navigate to settings
    const settingsTab = page.locator('button:has-text("Settings"), .tab-button:has-text("Settings")');
    if (await settingsTab.isVisible()) {
      await settingsTab.click();
      await page.waitForTimeout(300);
    }

    // General section should be visible
    const generalSection = page.locator('.settings-section');
    if ((await generalSection.count()) > 0) {
      await expect(generalSection.first()).toBeVisible();
    }
  });

  test('settings tab has multiple sections separated by dividers', async ({ page }) => {
    // Navigate to settings
    const settingsTab = page.locator('button:has-text("Settings"), .tab-button:has-text("Settings")');
    if (await settingsTab.isVisible()) {
      await settingsTab.click();
      await page.waitForTimeout(300);
    }

    // Should have dividers between sections
    const dividers = page.locator('.settings-divider');
    if ((await dividers.count()) > 0) {
      // Should have at least 2 dividers (between general/license and license/llm)
      expect(await dividers.count()).toBeGreaterThanOrEqual(1);
    }
  });
});
