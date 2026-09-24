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

for (const [width, height] of [[1536, 960], [1440, 900], [1280, 800], [1024, 768], [390, 844]]) {
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
      const optimization = await page.locator('.calc-panel').boundingBox();
      const field = await page.locator('.field').boundingBox();
      assert.ok(optimization.y < attacker.y);
      assert.ok(field.y < results.y);
      assert.equal(field.x, results.x);
      assert.equal(await page.locator('.app-sidebar').isVisible(), true);
    } else {
      assert.ok(results.y > defender.y + defender.height);
      if (width >= 900) assert.equal(attacker.y, defender.y);
      else assert.ok(defender.y >= attacker.y + attacker.height);
    }
    assert.match(await page.locator('.damage-grid').innerText(), /134–158/);
    assert.match(await page.locator('.damage-title').innerText(), /Iron Head[\s\S]*Floette-Mega/);
    // The card display now shows the set's actual investment instead of the
    // optimizer's best spread, so the default Floette-Mega EVs survive untouched.
    assert.deepEqual(await page.locator('[data-preview-sp]').allTextContents(), ['26', '0', '13', '5', '0', '22']);
    await page.locator('.damage-rolls summary').click();
    assert.equal((await page.locator('.damage-rolls code').innerText()).split(',').length, 16);
    if (screenshots) {
      fs.mkdirSync(screenshots, { recursive: true });
      await page.evaluate(() => window.scrollTo(0, 0));
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
  // spreadlab-rs db5d932 pins the defensive stat its engine cannot read for the
  // request, so the default physical benchmark no longer returns SpD-only variants
  // of the same objective. The reachable domain for this request is 17 rows
  // (1 + 2 + 5 + 9 over the 63..66 SP layers), and every one of them is rendered.
  assert.equal(matches.length, 17);
  assert.equal(await page.locator('.table-scroll tbody tr').count(), 17);
  await page.locator('[name="limit"]').fill('0');
  assert.equal(await page.locator('[name="limit"]').evaluate(node => node.validity.valid), false);
  await page.close();
});

test('viewer delegates ability effects and serializes the Active toggle', async () => {
  const page = await browser.newPage();
  await ready(page, '/damage');
  await page.evaluate(() => {
    const attacker = document.querySelector('[data-set-card="attacker"] .raw-editor');
    const defender = document.querySelector('[data-set-card="defender"] .raw-editor');
    attacker.value = 'Lucario\nAbility: Inner Focus\n- Close Combat';
    defender.value = 'Mega Lucario Z @ Lucarionite Z\nAbility: Aura Guard';
    syncRawEditor(attacker);
    syncRawEditor(defender);
    document.querySelector('[name="move_name"]').value = 'Close Combat';
  });
  // Compare fixed spreads, not the optimizer's different best spreads on/off.
  const damageForInputs = async () => {
    const body = await page.evaluate(() => currentPayload().body);
    const response = await page.request.post(`${baseURL}/api/damage`, { data: body });
    assert.equal(response.status(), 200);
    return response.json();
  };
  const guarded = await damageForInputs();
  await recalculate(page, () => page.locator('[data-set-card="defender"] [data-ability-toggle]').uncheck());
  const unguarded = await damageForInputs();
  assert.deepEqual(guarded.rolls, unguarded.rolls.map(value => Math.floor(value / 2)));
  const payload = await page.evaluate(() => currentPayload().body);
  assert.match(payload.defender_set, /Ability: Aura Guard/);
  assert.match(payload.defender_set, /Ability Enabled: false/);
  await page.evaluate(() => {
    const defender = document.querySelector('[data-set-card="defender"] .raw-editor');
    defender.value = 'Mega Meganium\nAbility: Mega Sol';
    syncRawEditor(defender);
    document.querySelector('[name="weather"][value="Rain"]').checked = true;
  });
  assert.equal((await page.evaluate(() => currentPayload().body)).field.weather, 'Rain');
  await page.evaluate(() => {
    const defender = document.querySelector('[data-set-card="defender"] .raw-editor');
    defender.value = 'Salamence\nAbility: Intimidate';
    syncRawEditor(defender);
    document.querySelector('[name="attacker_attack"]').value = '2';
  });
  assert.equal((await page.evaluate(() => currentPayload().body)).field.attacker_boosts.attack, 2);
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
  // The viewer submits manual conditions; library preprocessing applies Grassy Surge.
  assert.equal(await page.locator('[name="terrain"][value="None"]').isChecked(), true);
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
  assert.equal(await page.locator('[data-set-card="attacker"] [data-preview-sp]').count(), 6);
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

test('app navigation, palette, saved library, and ability selector work', async () => {
  const page = await browser.newPage({ viewport: { width: 1536, height: 960 } });
  await ready(page);
  await page.locator('[data-open-dialog="settings"]').click();
  await page.locator('[data-theme-picker]').selectOption('slate');
  assert.equal(await page.locator('html').getAttribute('data-theme'), 'slate');
  await page.keyboard.press('Escape');
  assert.equal(await page.locator('#shell-dialog').isVisible(), false);
  await page.locator('[data-open-dialog="guides"]').click();
  assert.match(await page.locator('#shell-dialog').innerText(), /Defensive mode/);
  await page.locator('[data-close-dialog]').click();
  const attacker = page.locator('[data-set-card="attacker"]');
  await attacker.locator('[data-ability-select]').selectOption('Pressure');
  assert.match(await attacker.locator('.raw-editor').inputValue(), /Ability: Pressure/);
  await attacker.locator('[data-save-set]').click();
  await attacker.locator('[data-save-set-name]').fill('Sidebar set');
  await attacker.locator('[data-confirm-save]').click();
  await page.locator('[data-open-dialog="saved"]').click();
  await page.locator('[data-library-side]').selectOption('defender');
  await page.locator('[data-library-set]').filter({ hasText: 'Sidebar set' }).click();
  assert.match(await page.locator('[data-set-card="defender"] .raw-editor').inputValue(), /Kingambit/);
  await page.locator('[data-open-dialog="metagame"]').click();
  await page.locator('[data-library-search]').fill('Venusaur');
  assert.ok(await page.locator('[data-library-set]').count() > 0);
  await page.locator('[data-library-set]').first().click();
  assert.match(await attacker.locator('.raw-editor').inputValue(), /Venusaur/);
  await page.reload();
  await page.waitForFunction(() => document.documentElement.dataset.theme === 'slate');
  await page.close();
});

test('stat modifier clicks update nature on editable and optimized sides', async () => {
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  await ready(page);
  for (const side of ['attacker', 'defender']) {
    const card = page.locator(`[data-set-card="${side}"]`);
    const nature = card.locator('[data-card-nature]');
    const stat = key => card.locator(`[data-sp-row] [data-sp-key="${key}"], [data-preview-sp="${key}"]`);
    await nature.selectOption('Adamant');
    await stat('spe').click({ modifiers: ['Control'] });
    assert.equal(await nature.inputValue(), 'Jolly');
    await stat('def').click({ modifiers: ['Alt'] });
    assert.equal(await nature.inputValue(), 'Hasty');
    assert.match(await card.locator('.raw-editor').inputValue(), /Hasty Nature/);
    await stat('def').click({ modifiers: ['Control'] });
    assert.equal(await nature.inputValue(), 'Relaxed');
    await stat('hp').click({ modifiers: ['Alt'] });
    assert.equal(await nature.inputValue(), 'Relaxed');
    await stat('atk').click();
    assert.equal(await nature.inputValue(), 'Relaxed');
    await nature.selectOption('Hardy');
    await stat('spa').click({ modifiers: ['Control'] });
    assert.equal(await nature.inputValue(), 'Modest');
    assert.equal(await stat('spa').evaluate(node => node.classList.contains('nature-boost')), true);
    assert.equal(await stat('atk').evaluate(node => node.classList.contains('nature-nerf')), true);
  }
  const payload = await page.evaluate(() => currentPayload());
  assert.match(payload.body.attacker_set, /Modest Nature/);
  assert.equal(payload.body.nature, 'Modest');
  await page.reload();
  await page.waitForFunction(() => [...document.querySelectorAll('[data-card-nature]')].every(node => node.value === 'Modest'));
  await page.close();
});


test('species search excludes unrelated presets with fuzzy title matches', async () => {
  const page = await browser.newPage();
  await ready(page);
  const card = page.locator('[data-set-card="attacker"]');
  await card.locator('[data-pokemon-choice]').click();
  const search = card.locator('[data-pokemon-selector]');
  for (const query of ['Lucario', 'luca']) {
    await search.fill(query);
    const options = card.locator('[data-pokemon-option]');
    await page.waitForFunction(() => {
      const options = [...document.querySelectorAll('[data-set-card="attacker"] [data-pokemon-option]')];
      return options.length > 0 && options.every(node => node.dataset.pokemonName === 'Lucario');
    });
    assert.ok(await options.count() > 1, 'species and its presets remain available');
    assert.equal(await options.filter({ hasText: 'Yurine' }).count(), 0);
  }
  await search.fill('Yurine');
  await page.waitForFunction(() => [...document.querySelectorAll('[data-set-card="attacker"] [data-pokemon-option]')].some(node => node.dataset.pokemonName === 'Talonflame'));
  await page.close();
});
