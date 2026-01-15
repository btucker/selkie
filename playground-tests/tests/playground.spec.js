import { test, expect } from '@playwright/test';

test.describe('Selkie Playground', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    // Wait for WASM to load
    await expect(page.getByText(/Rendered in/)).toBeVisible({ timeout: 30000 });
  });

  test.describe('Initial Load', () => {
    test('should display the page title', async ({ page }) => {
      await expect(page).toHaveTitle('Selkie Playground - Mermaid Diagram Editor');
    });

    test('should show the header with logo and title', async ({ page }) => {
      await expect(page.getByRole('heading', { name: 'Selkie Playground' })).toBeVisible();
      await expect(page.getByText('Fast Mermaid diagram rendering in Rust/WASM')).toBeVisible();
    });

    test('should load default example diagram', async ({ page }) => {
      const editor = page.locator('#editor');
      const value = await editor.inputValue();
      expect(value).toContain('flowchart TD');
    });

    test('should render default diagram as SVG', async ({ page }) => {
      const preview = page.locator('#preview');
      await expect(preview.locator('svg')).toBeVisible();
    });

    test('should display render time', async ({ page }) => {
      await expect(page.getByText(/Rendered in \d+\.\d+ms/)).toBeVisible();
    });
  });

  test.describe('Theme Selection', () => {
    test('should have theme selector with all themes', async ({ page }) => {
      const themeSelect = page.getByLabel('Select theme');
      await expect(themeSelect).toBeVisible();

      const options = themeSelect.locator('option');
      await expect(options).toHaveCount(5);
      await expect(options.nth(0)).toHaveText('Default');
      await expect(options.nth(1)).toHaveText('Dark');
      await expect(options.nth(2)).toHaveText('Forest');
      await expect(options.nth(3)).toHaveText('Neutral');
      await expect(options.nth(4)).toHaveText('Base');
    });

    test('should switch to dark theme and update background', async ({ page }) => {
      const themeSelect = page.getByLabel('Select theme');
      await themeSelect.selectOption('Dark');

      // Check URL includes theme
      await expect(page).toHaveURL(/#dark:/);

      // Check preview background changed (dark theme background is #1f2020)
      const previewContainer = page.locator('#preview-container');
      await expect(previewContainer).toHaveCSS('background-color', 'rgb(31, 32, 32)');
    });

    test('should switch to forest theme', async ({ page }) => {
      const themeSelect = page.getByLabel('Select theme');
      await themeSelect.selectOption('Forest');

      await expect(page).toHaveURL(/#forest:/);
      // Forest theme has white background
      const previewContainer = page.locator('#preview-container');
      await expect(previewContainer).toHaveCSS('background-color', 'rgb(255, 255, 255)');
    });

    test('should persist theme in URL', async ({ page }) => {
      // Switch to dark theme
      await page.getByLabel('Select theme').selectOption('Dark');

      // Get current URL
      const url = page.url();
      expect(url).toContain('#dark:');

      // Reload page
      await page.reload();
      await expect(page.getByText(/Rendered in/)).toBeVisible({ timeout: 30000 });

      // Verify theme is restored
      await expect(page.getByLabel('Select theme')).toHaveValue('dark');
    });
  });

  test.describe('Example Selection', () => {
    test('should have example selector', async ({ page }) => {
      const exampleSelect = page.getByLabel('Select example diagram');
      await expect(exampleSelect).toBeVisible();
    });

    test('should load flowchart example', async ({ page }) => {
      await page.getByLabel('Select example diagram').selectOption('flowchart-simple');
      await page.waitForTimeout(200); // Wait for example to load

      const editor = page.locator('#editor');
      const value = await editor.inputValue();
      expect(value).toContain('flowchart TD');
    });

    test('should load sequence diagram example', async ({ page }) => {
      await page.getByLabel('Select example diagram').selectOption('sequence-simple');
      await page.waitForTimeout(200); // Wait for example to load

      const editor = page.locator('#editor');
      const value = await editor.inputValue();
      expect(value).toContain('sequenceDiagram');
      await expect(page.getByText(/Rendered in/)).toBeVisible();
    });

    test('should load pie chart example', async ({ page }) => {
      await page.getByLabel('Select example diagram').selectOption('pie-simple');
      await page.waitForTimeout(200); // Wait for example to load

      const editor = page.locator('#editor');
      const value = await editor.inputValue();
      expect(value).toContain('pie');
    });
  });

  test.describe('Editor Functionality', () => {
    test('should update diagram on typing', async ({ page }) => {
      const editor = page.getByRole('textbox', { name: /Enter Mermaid diagram/ });

      // Clear and type new diagram
      await editor.fill('flowchart LR\n  A --> B');

      // Wait for render
      await expect(page.getByText(/Rendered in/)).toBeVisible();

      // Check SVG contains expected text
      const preview = page.locator('#preview svg');
      await expect(preview).toBeVisible();
    });

    test('should show error for invalid syntax', async ({ page }) => {
      const editor = page.getByRole('textbox', { name: /Enter Mermaid diagram/ });

      // Type invalid diagram
      await editor.fill('invalid diagram syntax !!!');

      // Wait a bit for render attempt
      await page.waitForTimeout(500);

      // Check error display
      const errorDisplay = page.locator('#error-display');
      await expect(errorDisplay).toBeVisible();
    });

    test('should clear error when valid syntax entered', async ({ page }) => {
      const editor = page.getByRole('textbox', { name: /Enter Mermaid diagram/ });

      // Type invalid, then valid
      await editor.fill('invalid!!!');
      await page.waitForTimeout(300);

      await editor.fill('flowchart TD\n  A --> B');
      await expect(page.getByText(/Rendered in/)).toBeVisible();

      const errorDisplay = page.locator('#error-display');
      await expect(errorDisplay).toBeHidden();
    });
  });

  test.describe('Zoom Controls', () => {
    test('should zoom in', async ({ page }) => {
      const zoomIn = page.getByRole('button', { name: '+' });
      const zoomReset = page.getByRole('button', { name: /\d+%/ });

      await zoomIn.click();
      await expect(zoomReset).toHaveText('125%');
    });

    test('should zoom out', async ({ page }) => {
      const zoomOut = page.getByRole('button', { name: '-' });
      const zoomReset = page.getByRole('button', { name: /\d+%/ });

      await zoomOut.click();
      await expect(zoomReset).toHaveText('75%');
    });

    test('should reset zoom', async ({ page }) => {
      const zoomIn = page.getByRole('button', { name: '+' });
      const zoomReset = page.getByRole('button', { name: /\d+%/ });

      // Zoom in first
      await zoomIn.click();
      await zoomIn.click();
      await expect(zoomReset).toHaveText('150%');

      // Reset
      await zoomReset.click();
      await expect(zoomReset).toHaveText('100%');
    });
  });

  test.describe('Download SVG', () => {
    test('should have download button', async ({ page }) => {
      const downloadBtn = page.getByRole('button', { name: 'Download SVG' });
      await expect(downloadBtn).toBeVisible();
    });

    test('should trigger download on click', async ({ page }) => {
      const downloadPromise = page.waitForEvent('download');

      await page.getByRole('button', { name: 'Download SVG' }).click();

      const download = await downloadPromise;
      expect(download.suggestedFilename()).toBe('diagram.svg');
    });
  });

  test.describe('URL State', () => {
    test('should update URL when diagram changes', async ({ page }) => {
      const editor = page.getByRole('textbox', { name: /Enter Mermaid diagram/ });

      await editor.fill('flowchart LR\n  X --> Y');

      // URL should be updated
      await expect(page).toHaveURL(/#/);
    });
  });
});

// Separate test without beforeEach to test URL hash loading
test('should load diagram from URL hash', async ({ page }) => {
  // Navigate with a specific hash (base64 encoded "flowchart LR\n  Custom")
  const diagram = 'flowchart LR\n  Custom';
  const encoded = btoa(encodeURIComponent(diagram));

  await page.goto(`/#${encoded}`);
  await expect(page.getByText(/Rendered in/)).toBeVisible({ timeout: 30000 });

  const editor = page.locator('#editor');
  const value = await editor.inputValue();
  expect(value).toContain('Custom');
});
