const { test, before, after } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const { chromium } = require('playwright');

const baseURL = process.env.SPREADLAB_URL || 'http://127.0.0.1:3000';
const screenshots = process.env.SPREADLAB_SCREENSHOTS;
let browser;
before(async () => {
  browser = await chromium.launch({
    ...(process.env.CHROMIUM_PATH ? { executablePath: process.env.CHROMIUM_PATH } : {}),
    headless: true,
  });
});
after(async () => browser?.close());

async function ready(page, route = '/survive') {
  await page.goto(`${baseURL}${route}`);
  await page.waitForSelector('.damage-card');
  await page.waitForFunction(() => document.querySelector('.results-panel').getAttribute('aria-busy') === 'false');
}
async function recalculate(page, action, endpoint = '/api/survive') {
  const response = page.waitForResponse(response => new URL(response.url()).pathname === endpoint && response.request().method() === 'POST');
  await action();
  const result = await response;
  assert.equal(result.status(), 200);
  await page.waitForFunction(() => document.querySelector('.results-panel').getAttribute('aria-busy') === 'false');
  return result.json();
}

for (const [width, height] of [[1440, 900], [1280, 800], [1024, 768], [390, 844]]) {
  test(`workspace geometry and real damage at ${width}×${height}`, async () => {
    const page = await browser.newPage({ viewport: { width, height } });
    page.setDefaultTimeout(10000);
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    await ready(page);
    assert.deepEqual(errors, []);
    assert.equal(await page.evaluate(() => document.documentElement.scrollWidth), width);
    const attacker = await page.locator('[data-set-card="attacker"]').boundingBox();
    const defender = await page.locator('[data-set-card="defender"]').boundingBox();
    const results = await page.locator('.results-panel').boundingBox();
    if (width >= 1200) {
      assert.equal(attacker.y, defender.y);
      assert.ok(results.x > defender.x);
      assert.equal(await page.locator('.results-panel').evaluate(node => getComputedStyle(node).position), 'sticky');
    } else {
      assert.ok(results.y > defender.y + defender.height);
      if (width >= 900) assert.equal(attacker.y, defender.y);
      else assert.ok(defender.y >= attacker.y + attacker.height);
    }
    assert.match(await page.locator('.damage-grid').innerText(), /134–158 HP/);
    assert.match(await page.locator('.damage-title').innerText(), /Iron Head[\s\S]*Floette-Mega/);
    assert.deepEqual(await page.locator('[data-optimized-sp]').allTextContents(), ['4', '0', '32', '0', '0', '0']);
    await page.locator('.damage-rolls summary').click();
    assert.equal((await page.locator('.damage-rolls code').innerText()).split(',').length, 16);
    if (screenshots) {
      fs.mkdirSync(screenshots, { recursive: true });
      await page.screenshot({ path: path.join(screenshots, `workspace-${width}.png`), fullPage: true });
    }
    await page.close();
  });
}

test('spread, nature, boosts, status, crit, keyboard conditions, and all ranks retain their payloads', async () => {
  let page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  await ready(page);
  const attacker = page.locator('[data-set-card="attacker"]');
  await attacker.locator('[data-sp-key="atk"]').fill('24');
  await attacker.locator('[data-card-nature]').selectOption('Jolly');
  await attacker.locator('[data-status-select]').selectOption('Burned');
  await page.locator('[name="attacker_attack"]').fill('2');
  await attacker.locator('[data-ability-toggle]').uncheck();
  await page.locator('[data-move="Knock Off"] .move-select').click();
  assert.equal(await page.locator('[data-move="Knock Off"] .move-select').getAttribute('aria-pressed'), 'true');
  await page.locator('[data-crit-move="Knock Off"]').check();
  // Radios were previously display:none and unreachable by keyboard.
  await page.locator('[name="terrain"][value="None"]').focus();
  await page.keyboard.press('ArrowRight');
  assert.equal(await page.locator('[name="terrain"][value="Electric"]').isChecked(), true);
  await page.locator('[name="gravity"]').focus();
  await page.keyboard.press('Space');
  const payload = await page.evaluate(() => currentPayload());
  assert.equal(payload.body.move_name, 'Knock Off');
  assert.equal(payload.body.critical, true);
  assert.match(payload.body.attacker_set, /24 Atk/);
  assert.match(payload.body.attacker_set, /Jolly Nature/);
  assert.match(payload.body.attacker_set, /Status: Burned/);
  assert.equal(payload.body.field.terrain, 'Electric');
  assert.equal(payload.body.field.gravity, true);
  assert.equal(await page.locator('[name="attacker_attack"]').inputValue(), '2');
  await ready(page);
  assert.equal(await page.locator('[data-crit-move="Knock Off"]').isChecked(), true);
  assert.equal(await page.locator('[name="terrain"][value="Electric"]').isChecked(), true);
  // A fresh browser context avoids the pagehide persistence handler restoring the old set.
  await page.close();
  page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  await ready(page);
  await page.locator('.damage-rolls summary').click();
  const data = await recalculate(page, () => page.locator('[name="limit"]').fill('20'));
  assert.equal(await page.locator('.damage-rolls').evaluate(node => node.open), true);
  const matches = Array.isArray(data) ? data : data.matches;
  assert.equal(matches.length, 20);
  assert.equal(await page.locator('.table-scroll tbody tr').count(), 20);
  await page.locator('[name="limit"]').fill('0');
  assert.equal(await page.locator('[name="limit"]').evaluate(node => node.validity.valid), false);
  await page.close();
});

test('set editing, saving, loading, forms, items, swapping, and offensive mode stay reachable', async () => {
  let page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  page.setDefaultTimeout(10000);
  await ready(page);
  const attacker = page.locator('[data-set-card="attacker"]');
  await attacker.locator('.raw-toggle').click();
  assert.equal(await attacker.locator('.raw-editor').isVisible(), true);
  await attacker.locator('.raw-editor').fill('Rillaboom @ Grassy Seed\nAbility: Grassy Surge\nAdamant Nature\nSPs: 32 Atk\n- Drum Beating\n- Knock Off');
  await attacker.locator('.raw-editor').blur();
  await attacker.locator('.raw-toggle').click();
  assert.equal(await page.locator('[name="terrain"][value="Grassy"]').isChecked(), true);
  await attacker.locator('[data-save-set]').click();
  await attacker.locator('[data-save-set-name]').fill('Browser test set');
  await attacker.locator('[data-confirm-save]').click();
  await attacker.locator('[data-pokemon-choice]').click();
  await attacker.locator('[data-pokemon-selector]').fill('Rillaboom');
  await attacker.locator('.saved-option').filter({ hasText: 'Browser test set' }).click();
  assert.match(await attacker.locator('.raw-editor').inputValue(), /Drum Beating/);
  await page.locator('.swap-action').click();
  assert.match(await page.locator('[data-set-card="defender"] .raw-editor').inputValue(), /Rillaboom/);
  // Load a species through the keyboard-operated selector, then its Mega form.
  await attacker.locator('[data-pokemon-choice]').click();
  await attacker.locator('[data-pokemon-selector]').fill('Lucario');
  await attacker.locator('[data-pokemon-selector]').press('Enter');
  await attacker.locator('[data-form-selector]').selectOption('Mega Lucario Z');
  assert.match(await attacker.locator('.raw-editor').inputValue(), /Mega Lucario Z @ Lucarionite Z/);
  await page.close();
  page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  await page.goto(`${baseURL}/ko`);
  await page.waitForFunction(() => document.querySelector('.results-panel').textContent.includes('No matching spread'));
  await recalculate(page, () => page.locator('[name="min_ko_chance"]').selectOption('0'), '/api/ko');
  await page.waitForSelector('.damage-card');
  assert.equal(await page.locator('[data-set-card="attacker"] [data-optimized-sp]').count(), 6);
  assert.equal(await page.locator('[data-set-card="defender"] [data-sp-key]').count(), 6);
  assert.equal(await page.locator('[name="min_ko_chance"]').isVisible(), true);
  const payload = await page.evaluate(() => currentPayload());
  assert.equal(payload.path, '/api/ko');
  await page.close();
});

test('loading, no-match, and error presentation do not claim success', async () => {
  const page = await browser.newPage({ viewport: { width: 390, height: 844 } });
  await ready(page);
  await page.route('**/api/survive', async route => {
    await new Promise(resolve => setTimeout(resolve, 300));
    await route.fulfill({ json: { matches: [] } });
  });
  await page.locator('.primary').click();
  await page.waitForSelector('.results-panel.loading');
  assert.match(await page.locator('[data-mobile-result]').innerText(), /Recalculating/);
  await page.waitForSelector('.empty-state');
  assert.equal(await page.locator('.result-status').count(), 0);
  assert.equal(await page.locator('[data-mobile-result]').innerText(), 'No matching spread');
  await page.unroute('**/api/survive');
  await page.route('**/api/survive', route => route.fulfill({ status: 400, json: { error: 'Test validation error' } }));
  await page.locator('.primary').click();
  await page.waitForSelector('.error-card');
  assert.match(await page.locator('.error-card').innerText(), /Test validation error/);
  assert.equal(await page.locator('[data-mobile-result]').innerText(), 'Calculation failed');
  await page.close();
});
