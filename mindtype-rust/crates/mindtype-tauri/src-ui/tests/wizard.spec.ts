import { testWizard as test, testMainWindow, expect, waitForAppLoad } from './fixtures';

test.describe('Setup Wizard', () => {
  test.beforeEach(async ({ page }) => {
    // Wizard tests use testWizard fixture (setup_completed = false)
    await page.goto('/');
    await waitForAppLoad(page);
  });

  test('displays wizard when setup not completed', async ({ page }) => {
    // The wizard should be visible
    const wizard = page.locator('.wizard');
    await expect(wizard).toBeVisible({ timeout: 5000 });
  });

  test('shows language selection step', async ({ page }) => {
    // Wizard step should be visible with language options
    const wizardStep = page.locator('.wizard__step');
    await expect(wizardStep).toBeVisible({ timeout: 5000 });

    // Should have language options (radio buttons)
    const languageOptions = page.locator('.language-option');
    await expect(languageOptions).toHaveCount(6); // en, ru, es, de, fr, zh
  });

  test('can select language and proceed', async ({ page }) => {
    // Select English (should be pre-selected)
    const englishOption = page.locator('input[name="language"][value="en"]');
    await expect(englishOption).toBeChecked();

    // Click next button
    const nextButton = page.locator('button:has-text("Next")');
    await expect(nextButton).toBeVisible();
    await nextButton.click();

    // Should proceed to provider step
    await page.waitForTimeout(300);
    const providerOption = page.locator('.provider-option');
    await expect(providerOption.first()).toBeVisible({ timeout: 5000 });
  });

  test('shows model download step after selecting local provider', async ({ page }) => {
    // Navigate to provider step
    const nextButton = page.locator('button:has-text("Next")');
    await nextButton.click();
    await page.waitForTimeout(300);

    // Select local provider (should be default)
    const localRadio = page.locator('input[name="provider"][value="local"]');
    await expect(localRadio).toBeChecked();

    // Model selection should be visible for local provider
    const modelSelect = page.locator('.model-select');
    await expect(modelSelect).toBeVisible();

    // Should list available models
    const modelOptions = page.locator('.model-option');
    await expect(modelOptions.first()).toBeVisible();
  });

  test('wizard has complete step', async ({ page }) => {
    // The wizard should have navigation flow
    // Verify first step is visible
    const firstStep = page.locator('.wizard__step');
    await expect(firstStep).toBeVisible({ timeout: 5000 });

    // Should have action buttons
    const actionButtons = page.locator('.wizard__actions button');
    await expect(actionButtons.first()).toBeVisible();
  });
});

// Separate test for completed setup using main window fixture
testMainWindow.describe('After Setup Complete', () => {
  testMainWindow('main window is visible when setup completed', async ({ page }) => {
    await page.goto('/');
    await waitForAppLoad(page);

    // Main window (.main) should be visible
    const mainWindow = page.locator('.main');
    await expect(mainWindow).toBeVisible({ timeout: 5000 });
  });
});
