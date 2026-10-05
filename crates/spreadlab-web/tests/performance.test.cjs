const { test, before, after } = require('node:test');
const assert = require('node:assert/strict');
const { chromium } = require('playwright');

const baseURL = process.env.SPREADLAB_URL || 'http://127.0.0.1:3000';
const pixel = Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVQIHWP4z8DwHwAFgAI/ScLbtAAAAABJRU5ErkJggg==', 'base64');
let browser;
before(async () => {
  browser = await chromium.launch({ headless: true, ...(process.env.CHROMIUM_PATH ? { executablePath: process.env.CHROMIUM_PATH } : {}) });
});
after(async () => browser?.close());

async function setup() {
  const page = await browser.newPage();
  await page.route('**/api/sprite/**', route => route.fulfill({ contentType: 'image/png', body: pixel }));
  await page.route('**/api/item-sprite/**', route => route.fulfill({ contentType: 'image/png', body: pixel }));
  await page.route('**/api/damage', route => route.fulfill({ json: { rolls: [] } }));
  return page;
}

test('shell and calculation work while every metadata response is pending', async () => {
  const page = await setup();
  let release;
  const pending = new Promise(resolve => { release = resolve; });
  await page.route(/\/api\/(meta|move-types|species-types|species-abilities|pokemon-list|item-list)$/, async route => {
    await pending;
    await route.continue();
  });
  let damageRequests = 0;
  const metadataRequests = new Map();
  page.on('request', request => {
    const path = new URL(request.url()).pathname;
    if (path === '/api/damage') damageRequests++;
    if (/\/api\/(meta|move-types|species-types|species-abilities|pokemon-list|item-list)$/.test(path)) {
      metadataRequests.set(path, (metadataRequests.get(path) || 0) + 1);
    }
  });
  try {
    await page.goto(`${baseURL}/damage`, { waitUntil: 'domcontentloaded' });
    assert.equal(await page.locator('script[src]').evaluateAll(scripts => scripts.every(script => script.defer)), true);
    await page.locator('[data-open-dialog="guides"]').click();
    assert.equal(await page.locator('#shell-dialog').evaluate(dialog => dialog.open), true);
    await page.waitForFunction(() => document.querySelector('.results-panel').getAttribute('aria-busy') === 'false');
    assert.ok(damageRequests > 0);
    release();
    await page.waitForFunction(() => speciesList.length > 0 && itemList.length > 0 && moveList.length > 0);
    assert.ok(await page.locator('[data-pokemon-option]').count() > 0);
    assert.equal(metadataRequests.size, 6);
    assert.ok([...metadataRequests.values()].every(count => count === 1), 'fetch reuses metadata preloads');
  } finally {
    release();
    await page.close();
  }
});

test('mode prefetch waits for intent, cancels early leave, deduplicates focus', async () => {
  const page = await setup();
  const prefetched = [];
  page.on('request', request => { if (new URL(request.url()).pathname === '/ko') prefetched.push(request); });
  await page.goto(`${baseURL}/damage`);
  const tab = page.locator('.mode-tabs a[href="/ko"]');
  await tab.dispatchEvent('pointerover', { pointerType: 'touch' });
  await page.waitForTimeout(180);
  assert.equal(prefetched.length, 0);
  await tab.dispatchEvent('pointerover', { pointerType: 'mouse' });
  await tab.dispatchEvent('pointerout', { pointerType: 'mouse' });
  await page.waitForTimeout(180);
  assert.equal(prefetched.length, 0);
  await tab.hover();
  await page.waitForTimeout(300);
  assert.equal(prefetched.length, 1);
  assert.equal(prefetched[0].method(), 'GET');
  await tab.focus();
  await page.waitForTimeout(180);
  assert.equal(prefetched.length, 1);
  await page.close();
});

test('preset hover warms final Mega form and item; keyboard warms item', async () => {
  const page = await setup();
  await page.goto(`${baseURL}/damage`);
  await page.waitForFunction(() => speciesList.length > 0 && itemList.length > 0 && builtInSets.some(set => set.text.includes('Mega Charizard Y')));
  const preset = await page.evaluate(() => {
    const set = builtInSets.find(set => set.text.includes('Mega Charizard Y'));
    const parsed = parseSet(setToMegaFormFromItem(set.text));
    return { id: set.id, species: parsed.name, item: parsed.item };
  });
  await page.locator('[data-set-card="attacker"] .pokemon-choice').click();
  await page.locator('[data-set-card="attacker"] [data-pokemon-selector]').fill('Charizard');
  const requests = [];
  page.on('request', request => requests.push(new URL(request.url()).pathname));
  const option = page.locator(`[data-set-card="attacker"] [data-set-id="${preset.id}"]`);
  await option.hover();
  await page.waitForTimeout(250);
  assert.ok(requests.includes(`/api/sprite/${encodeURIComponent(preset.species)}`));
  assert.ok(requests.includes(`/api/item-sprite/${encodeURIComponent(preset.item)}`));
  await option.click();
  await page.locator('[data-set-card="attacker"] .item-choice').click();
  const input = page.locator('[data-set-card="attacker"] [data-item-selector]');
  await input.fill('');
  await input.press('ArrowDown');
  const name = await page.locator('[data-set-card="attacker"] [data-item-option].is-active').getAttribute('data-item-name');
  await page.waitForTimeout(250);
  assert.ok(requests.includes(`/api/item-sprite/${encodeURIComponent(name)}`));
  await page.close();
});

test('Save-Data disables speculative requests', async () => {
  const page = await setup();
  await page.addInitScript(() => Object.defineProperty(navigator, 'connection', { value: { saveData: true, effectiveType: '4g' } }));
  let requests = 0;
  page.on('request', request => { if (new URL(request.url()).pathname === '/ko') requests++; });
  await page.goto(`${baseURL}/damage`);
  await page.locator('.mode-tabs a[href="/ko"]').hover();
  await page.waitForTimeout(250);
  assert.equal(requests, 0);
  await page.close();
});

test('sprite warming keeps at most two requests active', async () => {
  const page = await setup();
  await page.goto(`${baseURL}/damage`);
  await page.waitForFunction(() => itemList.length > 0);
  await page.locator('[data-set-card="attacker"] .item-choice').click();
  let release;
  const pending = new Promise(resolve => { release = resolve; });
  let active = 0;
  let peak = 0;
  let total = 0;
  await page.route('**/api/item-sprite/**', async route => {
    active++;
    total++;
    peak = Math.max(peak, active);
    await pending;
    await route.fulfill({ contentType: 'image/png', body: pixel });
    active--;
  });
  try {
    const options = page.locator('[data-set-card="attacker"] [data-item-option]');
    for (let index = 0; index < 4; index++) {
      await options.nth(index).hover();
      await page.waitForTimeout(160);
    }
    assert.equal(total, 2, 'remaining warmups wait in queue');
    release();
    await page.waitForTimeout(250);
    assert.equal(total, 4);
    assert.equal(peak, 2);
  } finally {
    release();
    await page.close();
  }
});
