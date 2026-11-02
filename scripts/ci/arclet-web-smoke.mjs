/**
 * Real Chromium smoke for Trunk-built Arclet. Fails closed on unsupported
 * GPU backends, failed canvas startup, download, or re-upload.
 * Usage: node scripts/ci/arclet-web-smoke.mjs http://127.0.0.1:8782
 */
import { chromium } from 'playwright';
import { mkdir, readFile, stat, writeFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';

const origin = process.argv[2] || 'http://127.0.0.1:8782';
const output = resolve(process.env.ARCLET_E2E_DIR || 'dist/arclet-web-smoke');
await mkdir(output, { recursive: true });
const report = { origin, timestamp: new Date().toISOString(), cases: [] };
let failed = false;

async function fileAction(page, name, shortcut) {
  const menu = page.getByRole('button', { name: 'File', exact: true });
  if (await menu.count()) {
    await menu.first().click();
    const action = page.getByRole('button', { name });
    if (await action.count()) {
      await action.first().click();
      return 'file menu';
    }
    await page.keyboard.press('Escape');
  }
  // The canvas renderer sometimes exposes no HTML accessibility buttons.
  await page.locator('#arclet-canvas').click({ position: { x: 730, y: 460 } });
  await page.keyboard.press(shortcut);
  return 'keyboard shortcut';
}

async function downloadAndReupload(page) {
  const canvas = page.locator('#arclet-canvas');
  await canvas.click({ position: { x: 730, y: 460 } });
  // Native integration tests separately verify actual shape-editing roundtrips.
  const downloadEvent = page.waitForEvent('download', { timeout: 25000 });
  const viaSave = await fileAction(page, /Save as/i, 'ControlOrMeta+Shift+S');
  const download = await downloadEvent;
  const name = download.suggestedFilename();
  if (!name.toLowerCase().endsWith('.sc')) {
    throw new Error('Expected a .sc CAD download, got: ' + name);
  }
  const saved = join(output, 'arclet-browser-roundtrip.sc');
  await download.saveAs(saved);
  if ((await stat(saved)).size < 128) {
    throw new Error('The saved browser design is empty');
  }
  const chooserEvent = page.waitForEvent('filechooser', { timeout: 25000 });
  const viaOpen = await fileAction(page, /^Open$/i, 'ControlOrMeta+O');
  const chooser = await chooserEvent;
  const bytes = await readFile(saved);
  await chooser.setFiles({ name: 'arclet-browser-roundtrip.sc', mimeType: 'application/octet-stream', buffer: bytes });
  await page.waitForTimeout(1800);
  if (!(await canvas.isVisible())) throw new Error('Canvas disappeared after opening the design');
  await page.screenshot({ path: join(output, 'webgl2-after-upload.png') });
  return { name, bytes: bytes.length, viaSave, viaOpen };
}

for (const mode of ['webgl2', 'webgpu']) {
  const result = { mode, status: 'FAIL', errors: [] };
  report.cases.push(result);
  let browser;
  try {
    browser = await chromium.launch({
      headless: true,
      args: ['--enable-webgl', '--enable-unsafe-webgpu', '--enable-unsafe-swiftshader', '--use-angle=swiftshader'],
    });
    const context = await browser.newContext({
      viewport: { width: 1440, height: 900 },
      deviceScaleFactor: 1,
      acceptDownloads: true,
    });
    const page = await context.newPage();
    page.on('pageerror', error => result.errors.push(error.message));
    page.on('console', message => {
      if (message.type() === 'error') result.errors.push(message.text());
    });
    const url = mode === 'webgl2' ? origin + '/?webgl' : origin + '/';
    await page.goto(url, { waitUntil: 'domcontentloaded', timeout: 30000 });
    if (mode === 'webgpu') {
      const usable = await page.evaluate(async () => !!(navigator.gpu && await navigator.gpu.requestAdapter()));
      if (!usable) throw new Error('Chromium has no usable WebGPU adapter on this runner');
    } else {
      const usable = await page.evaluate(() => !!document.createElement('canvas').getContext('webgl2'));
      if (!usable) throw new Error('Chromium has no WebGL2 context on this runner');
    }
    await page.locator('#arclet-loading').waitFor({ state: 'detached', timeout: 120000 });
    await page.waitForTimeout(1400);
    const dimensions = await page.locator('#arclet-canvas').evaluate(canvas => ({ width: canvas.width, height: canvas.height }));
    if (dimensions.width < 800 || dimensions.height < 500) {
      throw new Error('Canvas has invalid dimensions: ' + JSON.stringify(dimensions));
    }
    const screenshot = await page.screenshot({ path: join(output, mode + '-render.png'), fullPage: true });
    if (screenshot.length < 15000) throw new Error('Browser screenshot is unexpectedly small');
    result.dimensions = dimensions;
    if (mode === 'webgl2') result.files = await downloadAndReupload(page);
    if (result.errors.length) throw new Error(result.errors.join(' | ').slice(0, 3000));
    result.status = 'PASS';
    await context.close();
  } catch (error) {
    failed = true;
    result.error = String(error);
    console.error(mode + ' smoke failed: ' + error);
  } finally {
    if (browser) await browser.close();
  }
}
await writeFile(join(output, 'report.json'), JSON.stringify(report, null, 2) + '\n');
console.log(JSON.stringify(report, null, 2));
if (failed) process.exitCode = 1;
