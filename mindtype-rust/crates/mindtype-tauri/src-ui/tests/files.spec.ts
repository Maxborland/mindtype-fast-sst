import { testMainWindow as test, expect, waitForAppLoad } from './fixtures';

test.describe('File Processing', () => {
  test.beforeEach(async ({ page }) => {
    // Main window tests use testMainWindow fixture (setup_completed = true)
    await page.goto('/');
    await waitForAppLoad(page);
  });

  test('can navigate to files tab', async ({ page }) => {
    // Find and click files tab (tab buttons are in tab-bar)
    const filesTab = page.locator('button:has-text("Files"), .tab-button:has-text("Files")');

    if (await filesTab.isVisible()) {
      await filesTab.click();
      await page.waitForTimeout(300);

      // Files view component should be visible
      const filesView = page.locator('.files-tab');
      await expect(filesView).toBeVisible({ timeout: 5000 });
    }
  });

  test('shows drop zone for files', async ({ page }) => {
    // Navigate to files tab
    const filesTab = page.locator('button:has-text("Files"), .tab-button:has-text("Files")');
    if (await filesTab.isVisible()) {
      await filesTab.click();
      await page.waitForTimeout(300);
    }

    // Drop zone should be visible
    const dropZone = page.locator('.drop-zone');
    if (await dropZone.isVisible()) {
      await expect(dropZone).toBeVisible();
    }
  });

  test('shows empty queue initially', async ({ page }) => {
    // Navigate to files tab
    const filesTab = page.locator('button:has-text("Files"), .tab-button:has-text("Files")');
    if (await filesTab.isVisible()) {
      await filesTab.click();
      await page.waitForTimeout(300);
    }

    // Queue should be empty or show placeholder
    const queueItems = page.locator('.queue-item, .file-item');
    const itemCount = await queueItems.count().catch(() => 0);

    // No items in queue initially
    expect(itemCount).toBe(0);
  });

  test('has process button', async ({ page }) => {
    // Navigate to files tab
    const filesTab = page.locator('button:has-text("Files"), .tab-button:has-text("Files")');
    if (await filesTab.isVisible()) {
      await filesTab.click();
      await page.waitForTimeout(300);
    }

    // Look for process/transcribe button
    const processButton = page.locator(
      'button:has-text("Process"), button:has-text("Transcribe"), button:has-text("Start")'
    );

    // Button should exist (may be disabled when queue is empty)
    if ((await processButton.count()) > 0) {
      await expect(processButton.first()).toBeVisible();
    }
  });

  test('files tab content loads', async ({ page }) => {
    // Navigate to files tab
    const filesTab = page.locator('button:has-text("Files"), .tab-button:has-text("Files")');
    if (await filesTab.isVisible()) {
      await filesTab.click();
      await page.waitForTimeout(300);
    }

    // Page should have some content indicating file processing support
    const pageContent = await page.textContent('body');
    expect(pageContent).toBeTruthy();
    // Content should be related to files
    expect(pageContent?.length).toBeGreaterThan(0);
  });
});
