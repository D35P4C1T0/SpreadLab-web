const { test, before, after } = require('node:test');
const assert = require('node:assert/strict');
const { chromium } = require('playwright');
const baseURL = process.env.SPREADLAB_URL || 'http://127.0.0.1:3000';
let browser;
before(async () => {
  browser = await chromium.launch({ headless: true, ...(process.env.CHROMIUM_PATH ? { executablePath: process.env.CHROMIUM_PATH } : {}) });
});
after(async () => browser?.close());

async function ready(page, url = `${baseURL}/damage`) {
  await page.goto(url);
  await page.waitForFunction(() => speciesList.length > 0 && moveList.length > 0);
  await page.waitForFunction(() => document.querySelector('.results-panel').getAttribute('aria-busy') === 'false');
}
const toggle = side => `[data-set-card="${side}"] [data-ability-toggle]`;
const raw = side => `[data-set-card="${side}"] .raw-editor`;

test('legacy saved state migrates; clean JSON flags still control real damage', async () => {
  const page = await browser.newPage();
  await page.addInitScript(() => localStorage.setItem('spreadlab.webui.state.v1', JSON.stringify({
    attacker: 'Lucario\nAbility: Inner Focus\n- Close Combat',
    defender: 'Mega Lucario Z @ Lucarionite Z\nAbility: Aura Guard\nAbility Enabled: false',
    move: 'Close Combat',
  })));
  await ready(page);
  assert.equal(await page.locator(toggle('defender')).isChecked(), false);
  assert.doesNotMatch(await page.locator(raw('defender')).inputValue(), /Ability Enabled:/);
  const body = await page.evaluate(() => currentPayload().body);
  assert.equal(body.defender_ability_enabled, false);
  assert.doesNotMatch(body.defender_set, /Ability Enabled:/);
  const unguardedResponse = await page.request.post(`${baseURL}/api/damage`, { data: body });
  const guardedResponse = await page.request.post(`${baseURL}/api/damage`, { data: { ...body, defender_ability_enabled: true } });
  assert.equal(unguardedResponse.status(), 200);
  assert.equal(guardedResponse.status(), 200);
  const unguarded = await unguardedResponse.json();
  const guarded = await guardedResponse.json();
  assert.deepEqual(guarded.rolls, unguarded.rolls.map(value => Math.floor(value / 2)));
  const malformed = await page.request.post(`${baseURL}/api/damage`, { data: { ...body, defender_ability_enabled: 'false' } });
  assert.equal(malformed.status(), 422);
  await page.close();
});

test('saved sets, swaps, shared links and reload retain separate toggle metadata', async () => {
  const page = await browser.newPage();
  await ready(page);
  await page.locator(toggle('attacker')).uncheck();
  assert.doesNotMatch(await page.locator(raw('attacker')).inputValue(), /Ability Enabled:/);
  await page.locator('[data-set-card="attacker"] [data-save-set]').click();
  await page.locator('[data-set-card="attacker"] [data-save-set-name]').fill('Disabled ability');
  await page.locator('[data-set-card="attacker"] [data-confirm-save]').click();
  const saved = await page.evaluate(() => JSON.parse(localStorage.getItem(savedSetsKey))[0]);
  assert.equal(saved.abilityEnabled, false);
  assert.doesNotMatch(saved.text, /Ability Enabled:/);
  await page.locator(toggle('attacker')).check();
  await page.locator('[data-open-dialog="saved"]').click();
  await page.locator('[data-library-set]').click();
  assert.equal(await page.locator(toggle('attacker')).isChecked(), false);
  await page.locator('.swap-action').click();
  assert.equal(await page.locator(toggle('attacker')).isChecked(), true);
  assert.equal(await page.locator(toggle('defender')).isChecked(), false);
  // Share is currently disabled in the UI; exercise its existing serialization.
  await page.evaluate(() => {
    initShare();
    document.querySelector('.share-action').dispatchEvent(new MouseEvent('click'));
  });
  const url = page.url();
  const share = JSON.parse(Buffer.from(new URL(url).hash.slice(1), 'base64').toString('utf8'));
  assert.equal(share.defenderAbilityEnabled, false);
  assert.doesNotMatch(share.d, /Ability Enabled:/);
  const other = await browser.newPage();
  await ready(other, url);
  assert.equal(await other.locator(toggle('defender')).isChecked(), false);
  await page.evaluate(() => saveStateNow());
  await ready(page, `${baseURL}/damage`);
  assert.equal(await page.locator(toggle('defender')).isChecked(), false);
  const state = await page.evaluate(() => JSON.parse(localStorage.getItem(storageKey)));
  assert.equal(state.defenderAbilityOn, false);
  assert.doesNotMatch(state.defender, /Ability Enabled:/);
  await other.close();
  await page.close();
});
