// Browser tests for explicit "Apply spread" actions on ranked optimizer rows.
//
// Optimizer responses are intercepted so the assertions describe the apply
// behavior rather than optimizer runtime. Run against a live dev server, e.g.:
//
//   SPREADLAB_URL=http://127.0.0.1:3311 node --test crates/spreadlab-web/tests/apply.test.cjs
const { test, before, after } = require('node:test');
const assert = require('node:assert/strict');
const { chromium } = require('playwright');

const baseURL = process.env.SPREADLAB_URL || 'http://127.0.0.1:3000';

const ROWS = [
  { rank: 1, nature: 'Bold', sp_line: 'SPs: 4 HP / 32 Def', total_points: 36, final_stats: { hp: 143, attack: 45, defense: 121, special_attack: 110, special_defense: 100, speed: 90 }, result: { ko_chance: 0.125, min_damage: 25, max_damage: 30, percent_min: 20.8, percent_max: 25.0 } },
  { rank: 2, nature: 'Calm', sp_line: 'SPs: 20 HP / 12 Def / 10 SpD', total_points: 42, final_stats: { hp: 150, attack: 45, defense: 100, special_attack: 110, special_defense: 115, speed: 90 }, result: { ko_chance: 0.0625, min_damage: 22, max_damage: 27, percent_min: 18.3, percent_max: 22.5 } },
  { rank: 3, nature: 'Impish', sp_line: 'SPs: 32 HP / 20 Def', total_points: 52, final_stats: { hp: 160, attack: 45, defense: 110, special_attack: 110, special_defense: 100, speed: 90 }, result: { ko_chance: 0.0, min_damage: 20, max_damage: 24, percent_min: 16.6, percent_max: 20.0 } },
];

const OFFENSIVE_ROWS = [
  { rank: 1, nature: 'Jolly', sp_line: 'SPs: 32 Atk / 4 Spe', total_points: 36, final_stats: { hp: 100, attack: 180, defense: 90, special_attack: 60, special_defense: 90, speed: 120 }, result: { ko_chance: 0.5, min_damage: 120, max_damage: 140, percent_min: 60.0, percent_max: 70.0 } },
  { rank: 2, nature: 'Adamant', sp_line: 'SPs: 32 Atk', total_points: 32, final_stats: { hp: 100, attack: 190, defense: 90, special_attack: 60, special_defense: 90, speed: 80 }, result: { ko_chance: 0.4, min_damage: 130, max_damage: 150, percent_min: 65.0, percent_max: 75.0 } },
  { rank: 3, nature: 'Naive', sp_line: 'SPs: 24 Atk / 12 Spe', total_points: 36, final_stats: { hp: 100, attack: 170, defense: 90, special_attack: 60, special_defense: 90, speed: 110 }, result: { ko_chance: 0.3, min_damage: 110, max_damage: 130, percent_min: 55.0, percent_max: 65.0 } },
];

let browser;
before(async () => {
  browser = await chromium.launch({
    ...(process.env.CHROMIUM_PATH ? { executablePath: process.env.CHROMIUM_PATH } : {}),
    headless: true,
  });
});
after(async () => browser?.close());

async function pollUntil(predicate, message, timeout = 10000) {
  const deadline = Date.now() + timeout;
  while (Date.now() < deadline) {
    if (predicate()) return;
    await new Promise(resolve => setTimeout(resolve, 20));
  }
  throw new Error(`timed out waiting for ${message}`);
}

// Holds optimizer responses until the test releases them, so apply/stale timing
// is deterministic. The best-damage enrichment is answered instantly.
async function stubOptimizer(page, endpoint, rows) {
  const state = { requests: 0, pending: [] };
  await page.route(`**${endpoint}`, (route) => {
    state.requests += 1;
    return new Promise((resolve) => state.pending.push(resolve)).then(() => route.fulfill({ json: { matches: rows, best: rows[0] } }));
  });
  await page.route('**/api/damage', (route) => route.fulfill({ json: { rolls: [] } }));
  state.release = (index = 0) => {
    const resolve = state.pending.splice(index, 1)[0];
    if (resolve) resolve();
  };
  return state;
}

async function openWithRows(page, route, endpoint, rows) {
  const stub = await stubOptimizer(page, endpoint, rows);
  await page.goto(`${baseURL}${route}`);
  await pollUntil(() => stub.requests === 1, 'initial optimizer request');
  stub.release();
  await page.waitForFunction((expected) => document.querySelectorAll('.table-card tbody tr').length === expected, rows.length);
  await page.waitForFunction(() => document.querySelector('.results-panel').getAttribute('aria-busy') === 'false');
  return stub;
}

function parsedSetOf(page, key) {
  return page.evaluate((cardKey) => parseSet(document.querySelector(`[data-set-card="${cardKey}"] .raw-editor`).value), key);
}

test('a non-first defensive row applies its SPs and nature only when clicked', async () => {
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  const stub = await openWithRows(page, '/survive', '/api/survive', ROWS);
  const defender = page.locator('[data-set-card="defender"]');

  // Arriving results must not change the set or the card display.
  assert.equal(await defender.locator('[data-card-nature]').inputValue(), 'Timid');
  assert.doesNotMatch(await defender.locator('.raw-editor').inputValue(), /Calm Nature/);
  assert.deepEqual(await defender.locator('[data-preview-sp]').allTextContents(), ['26', '0', '13', '5', '0', '22']);

  // Unrelated fields that must survive the apply.
  await defender.locator('[data-status-select]').selectOption('Burned');
  await page.locator('[name="defender_special_defense"]').fill('2');
  const attackerBefore = await page.locator('[data-set-card="attacker"] .raw-editor').inputValue();
  // Editing invalidates the displayed rows; wait for the replacement run before
  // the rows become actionable again.
  await page.locator('[name="defender_special_defense"]').blur();
  await pollUntil(() => stub.requests === 2, 'recalculation after edits');
  stub.release();
  await page.waitForFunction(() => document.querySelector('.results-panel').getAttribute('aria-busy') === 'false');

  await page.locator('.table-card tbody tr').nth(1).locator('[data-apply-index]').click();
  await pollUntil(() => stub.requests === 3, 'recalculation after apply');
  await page.waitForFunction(() => document.querySelector('[data-set-card="defender"] .raw-editor').value.includes('Calm Nature'));
  stub.release();
  await page.waitForFunction(() => document.querySelector('.results-panel').getAttribute('aria-busy') === 'false');

  const parsed = await parsedSetOf(page, 'defender');
  assert.equal(parsed.nature, 'Calm');
  assert.deepEqual(parsed.sps, { hp: 20, atk: 0, def: 12, spa: 0, spd: 10, spe: 0 });
  assert.match(await defender.locator('.raw-editor').inputValue(), /SPs: 20 HP \/ 0 Atk \/ 12 Def \/ 0 SpA \/ 10 SpD \/ 0 Spe/);
  // Species, item, ability, moves, status and opponent are preserved.
  assert.equal(parsed.name, 'Floette-Mega');
  assert.equal(parsed.item, 'Floettite');
  assert.equal(parsed.ability, 'Fairy Aura');
  assert.equal(parsed.status, 'Burned');
  assert.deepEqual(parsed.moves, ['Dazzling Gleam', 'Draining Kiss', 'Calm Mind', 'Protect']);
  assert.equal(await page.locator('[name="defender_special_defense"]').inputValue(), '2');
  assert.equal(await page.locator('[data-set-card="attacker"] .raw-editor').inputValue(), attackerBefore);
  // The preview now reflects the applied input, not the best row.
  assert.deepEqual(await defender.locator('[data-preview-sp]').allTextContents(), ['20', '0', '12', '0', '10', '0']);
  await page.close();
});

test('offensive mode applies a non-first row to the attacker', async () => {
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  const stub = await openWithRows(page, '/ko', '/api/ko', OFFENSIVE_ROWS);
  const attacker = page.locator('[data-set-card="attacker"]');
  const defenderBefore = await page.locator('[data-set-card="defender"] .raw-editor').inputValue();

  await page.locator('.table-card tbody tr').nth(2).locator('[data-apply-index]').click();
  await pollUntil(() => stub.requests === 2, 'recalculation after apply');
  await page.waitForFunction(() => document.querySelector('[data-set-card="attacker"] .raw-editor').value.includes('Naive Nature'));

  const parsed = await parsedSetOf(page, 'attacker');
  assert.equal(parsed.nature, 'Naive');
  assert.deepEqual(parsed.sps, { hp: 0, atk: 24, def: 0, spa: 0, spd: 0, spe: 12 });
  assert.equal(await attacker.locator('[data-card-nature]').inputValue(), 'Naive');
  assert.match(await attacker.locator('.raw-editor').inputValue(), /SPs: 0 HP \/ 24 Atk \/ 0 Def \/ 0 SpA \/ 0 SpD \/ 12 Spe/);
  assert.equal(await page.locator('[data-set-card="defender"] .raw-editor').inputValue(), defenderBefore);
  await page.close();
});

test('apply rows are keyboard reachable with row-specific names', async () => {
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  const stub = await openWithRows(page, '/survive', '/api/survive', ROWS);
  const button = page.locator('.table-card tbody tr').nth(1).locator('[data-apply-index]');
  assert.equal(await button.getAttribute('type'), 'button');
  assert.match(await button.getAttribute('aria-label'), /Apply spread rank 2: Calm, 20 HP \/ 12 Def \/ 10 SpD/);

  await button.focus();
  await page.waitForFunction(() => getComputedStyle(document.querySelector('.table-card tbody tr:nth-child(2) .apply-spread')).opacity === '1');
  await page.keyboard.press('Enter');
  await pollUntil(() => stub.requests === 2, 'Enter applies the focused row');
  await page.waitForFunction(() => document.querySelector('[data-set-card="defender"] .raw-editor').value.includes('Calm Nature'));

  stub.release();
  await page.waitForFunction(() => document.querySelector('.results-panel').getAttribute('aria-busy') === 'false');
  const first = page.locator('.table-card tbody tr').first().locator('[data-apply-index]');
  await first.focus();
  await page.keyboard.press('Space');
  await pollUntil(() => stub.requests === 3, 'Space applies the focused row');
  await page.waitForFunction(() => document.querySelector('[data-set-card="defender"] .raw-editor').value.includes('Bold Nature'));
  await page.close();
});

test('apply actions become inert as soon as the inputs change', async () => {
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  const stub = await openWithRows(page, '/survive', '/api/survive', ROWS);
  const setBefore = await page.locator('[data-set-card="defender"] .raw-editor').inputValue();

  await page.locator('[name="limit"]').fill('7');
  await page.waitForFunction(() => document.querySelector('.results-panel').classList.contains('is-stale'));
  const button = page.locator('.table-card tbody tr').nth(1).locator('[data-apply-index]');
  assert.equal(await button.isDisabled(), true);
  assert.equal(await page.evaluate(() => currentResultContext), null);

  // Direct dispatch bypasses Playwright's actionability checks; a stale action
  // must not touch the set.
  await page.evaluate(() => document.querySelectorAll('.table-card tbody tr')[1].querySelector('[data-apply-index]').click());
  assert.equal(await page.locator('[data-set-card="defender"] .raw-editor').inputValue(), setBefore);

  // The replacement run stays pending until released, so stale state is stable.
  await pollUntil(() => stub.requests === 2, 'replacement recalculation');
  stub.release();
  await page.waitForFunction(() => document.querySelector('.results-panel').getAttribute('aria-busy') === 'false');
  await page.close();
});

test('editing the raw set invalidates apply actions before its debounce', async () => {
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  const stub = await openWithRows(page, '/survive', '/api/survive', ROWS);
  const defender = page.locator('[data-set-card="defender"]');
  await defender.locator('.raw-toggle').click();
  const typed = 'Floette-Mega @ Floettite\nAbility: Fairy Aura\nLevel: 50\nEVs: 30 HP / 4 Def / 8 SpA / 12 Spe\nTimid Nature\n- Dazzling Gleam';
  await defender.locator('.raw-editor').fill(typed);

  // The raw editor debounces recalculation, but the displayed rows go stale at
  // once so a stale Apply cannot overwrite what the user just typed.
  const button = page.locator('.table-card tbody tr').nth(1).locator('[data-apply-index]');
  assert.equal(await button.isDisabled(), true);
  assert.equal(await page.evaluate(() => currentResultContext), null);

  await page.evaluate(() => document.querySelectorAll('.table-card tbody tr')[1].querySelector('[data-apply-index]').click());
  const value = await defender.locator('.raw-editor').inputValue();
  assert.match(value, /EVs: 30 HP \/ 4 Def \/ 8 SpA \/ 12 Spe/);
  assert.doesNotMatch(value, /Calm Nature/);

  // The existing debounce still produces exactly one recalculation.
  await pollUntil(() => stub.requests === 2, 'raw-editor debounce recalculation');
  stub.release();
  await page.waitForFunction(() => document.querySelector('.results-panel').getAttribute('aria-busy') === 'false');
  await page.close();
});

test('an applied spread is persisted across reload', async () => {
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  const stub = await openWithRows(page, '/survive', '/api/survive', ROWS);
  await page.locator('.table-card tbody tr').nth(1).locator('[data-apply-index]').click();
  await pollUntil(() => stub.requests === 2, 'recalculation after apply');
  stub.release();
  await page.waitForFunction(() => document.querySelector('.results-panel').getAttribute('aria-busy') === 'false');
  await page.waitForTimeout(200);

  await page.reload();
  await page.waitForFunction(() => document.querySelector('[data-set-card="defender"] .raw-editor').value.includes('Calm Nature'));
  await page.waitForFunction(() => document.querySelector('[data-set-card="defender"] [data-preview-sp="hp"]').textContent.trim() === '20');
  const parsed = await parsedSetOf(page, 'defender');
  assert.deepEqual(parsed.sps, { hp: 20, atk: 0, def: 12, spa: 0, spd: 10, spe: 0 });
  assert.equal(await page.locator('[data-set-card="defender"] [data-card-nature]').inputValue(), 'Calm');
  assert.deepEqual(await page.locator('[data-set-card="defender"] [data-preview-sp]').allTextContents(), ['20', '0', '12', '0', '10', '0']);
  stub.release();
  await page.close();
});

test('apply rows stay visible on touch and hover reveals them without shifting layout', async () => {
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  await openWithRows(page, '/survive', '/api/survive', ROWS);
  const row = page.locator('.table-card tbody tr').nth(1);
  const button = row.locator('[data-apply-index]');

  const before = await row.boundingBox();
  await row.hover();
  await page.waitForFunction(() => getComputedStyle(document.querySelector('.table-card tbody tr:nth-child(2) .apply-spread')).opacity === '1');
  const after = await row.boundingBox();
  assert.equal(after.height, before.height);
  assert.equal(after.width, before.width);
  await page.close();

  const touch = await browser.newPage({ viewport: { width: 390, height: 844 }, hasTouch: true });
  await openWithRows(touch, '/survive', '/api/survive', ROWS);
  assert.equal(await touch.evaluate(() => matchMedia('(hover: none)').matches), true);
  const touchButton = touch.locator('.table-card tbody tr').nth(1).locator('[data-apply-index]');
  assert.equal(await touchButton.evaluate(node => getComputedStyle(node).opacity), '1', 'touch devices always show the action');
  await touchButton.click();
  assert.equal(await touch.locator('[data-set-card="defender"] [data-card-nature]').inputValue(), 'Calm');
  await touch.close();
});
