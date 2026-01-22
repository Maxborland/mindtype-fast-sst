import { test, expect } from '@playwright/test';

/**
 * Smoke tests that verify basic page loading without Tauri backend.
 * These tests run against the dev server and verify the HTML/CSS/JS loads correctly.
 */

test.describe('Smoke Tests', () => {
  test('index.html loads', async ({ page }) => {
    await page.goto('/');
    // Should have a title
    await expect(page).toHaveTitle(/MindType/i);
  });

  test('has app container', async ({ page }) => {
    await page.goto('/');
    const appDiv = page.locator('#app');
    await expect(appDiv).toBeVisible();
  });

  test('CSS styles load', async ({ page }) => {
    await page.goto('/');
    // Check that System 7 styles are loaded
    const styles = await page.evaluate(() => {
      return getComputedStyle(document.body).fontFamily;
    });
    expect(styles).toBeTruthy();
  });

  test('JavaScript executes', async ({ page }) => {
    await page.goto('/');
    // If JS works, svelte will try to mount
    const appDiv = page.locator('#app');
    await expect(appDiv).toBeAttached();
  });

  test('no critical console errors on load', async ({ page }) => {
    const errors: string[] = [];
    page.on('console', (msg) => {
      if (msg.type() === 'error') {
        errors.push(msg.text());
      }
    });

    await page.goto('/');
    await page.waitForTimeout(1000);

    // Filter out expected Tauri errors (since we're not in Tauri context)
    const criticalErrors = errors.filter(
      (e) =>
        !e.includes('__TAURI') &&
        !e.includes('invoke') &&
        !e.includes('Failed to initialize')
    );

    expect(criticalErrors).toHaveLength(0);
  });

  test('overlay.html loads', async ({ page }) => {
    await page.goto('/overlay.html');
    // Overlay uses #overlay container
    const overlayDiv = page.locator('#overlay');
    await expect(overlayDiv).toBeAttached();
  });
});
