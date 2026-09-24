// Deterministic browser tests for the results loading skeleton in assets/app.js.
//
// Every calculation request is intercepted and released by the test, so the
// assertions describe the skeleton lifecycle rather than optimizer runtime. Run
// against a live dev server, e.g.:
//
//   SPREADLAB_URL=http://127.0.0.1:3321 \
//   CHROMIUM_PATH=~/Library/Caches/ms-playwright/chromium_headless_shell-1178/chrome-mac/headless_shell \
//   node --test crates/spreadlab-web/tests/skeleton.test.cjs
const { test, before, after } = require('node:test');
const assert = require('node:assert/strict');
const { chromium } = require('playwright');

const baseURL = process.env.SPREADLAB_URL || 'http://127.0.0.1:3000';
const DEBOUNCE_MS = 320;

const ROWS = [
  { rank: 1, nature: 'Bold', sp_line: 'SPs: 4 HP / 32 Def', total_points: 36, final_stats: { hp: 143, attack: 45, defense: 121, special_attack: 110, special_defense: 100, speed: 90 }, result: { ko_chance: 0.125, min_damage: 25, max_damage: 30, percent_min: 20.8, percent_max: 25.0 } },
  { rank: 2, nature: 'Calm', sp_line: 'SPs: 20 HP / 12 Def / 10 SpD', total_points: 42, final_stats: { hp: 150, attack: 45, defense: 100, special_attack: 110, special_defense: 115, speed: 90 }, result: { ko_chance: 0.0625, min_damage: 22, max_damage: 27, percent_min: 18.3, percent_max: 22.5 } },
  { rank: 3, nature: 'Impish', sp_line: 'SPs: 32 HP / 20 Def', total_points: 52, final_stats: { hp: 160, attack: 45, defense: 110, special_attack: 110, special_defense: 100, speed: 90 }, result: { ko_chance: 0.0, min_damage: 20, max_damage: 24, percent_min: 16.6, percent_max: 20.0 } },
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

// Holds optimizer responses until the test releases them, so the skeleton's
// loading lifetime is decided by the test rather than by optimizer runtime.
async function stubSurvive(page, options = {}) {
  const state = { requests: [], inflight: 0, maxInflight: 0, pending: [] };
  await page.route('**/api/survive', (route) => {
    state.requests.push({ body: JSON.parse(route.request().postData() || '{}'), at: Date.now() });
    state.inflight += 1;
    state.maxInflight = Math.max(state.maxInflight, state.inflight);
    return new Promise((resolve) => state.pending.push(resolve)).then(({ status, json }) => {
      state.inflight -= 1;
      return json === undefined ? route.fulfill({ status }) : route.fulfill({ status, json });
    });
  });
  await page.route('**/api/damage', (route) => route.fulfill({ json: { rolls: options.damageRolls || [] } }));
  state.release = (index, status, json) => state.pending.splice(index, 1)[0]({ status, json });
  return state;
}

// Registered after stubSurvive to hold the best-damage enrichment open instead.
async function stubDamage(page) {
  const state = { requests: 0, inflight: 0, pending: [] };
  await page.route('**/api/damage', (route) => {
    state.requests += 1;
    state.inflight += 1;
    return new Promise((resolve) => state.pending.push(resolve)).then(({ status, json }) => {
      state.inflight -= 1;
      return route.fulfill({ status, json });
    });
  });
  state.release = (index, status, json) => state.pending.splice(index, 1)[0]({ status, json });
  return state;
}

async function openLoading(page, route = '/survive') {
  const stub = await stubSurvive(page);
  await page.goto(`${baseURL}${route}`);
  await page.waitForSelector('.results-panel.loading [data-results-skeleton]');
  await pollUntil(() => stub.requests.length === 1, 'initial auto-run');
  return stub;
}

async function openWithResults(page, route = '/survive', rows = ROWS) {
  const stub = await stubSurvive(page);
  await page.goto(`${baseURL}${route}`);
  await pollUntil(() => stub.requests.length === 1, 'initial auto-run');
  stub.release(0, 200, { matches: rows, best: rows[0] });
  await page.waitForFunction(() => document.querySelector('.results-panel').getAttribute('aria-busy') === 'false');
  await page.waitForSelector('.table-card tbody tr');
  return stub;
}

async function releaseAndSettle(page, stub, index, status, json) {
  stub.release(index, status, json);
  await page.waitForFunction(() => document.querySelector('.results-panel').getAttribute('aria-busy') === 'false'
    && !document.querySelector('.results-panel').classList.contains('loading'));
}

// One page round-trip with everything the assertions need about the loading
// surface, the content it covers, and the accessibility state around it.
function loadingState(page) {
  return page.evaluate(() => {
    const panel = document.querySelector('.results-panel');
    const skeleton = panel.querySelector('[data-results-skeleton]');
    const covered = [...panel.querySelectorAll('[data-skeleton-covered]')];
    const bars = skeleton ? [...skeleton.querySelectorAll('.skeleton-bar, .skeleton-row')] : [];
    // The panel can sit below the fold on mobile; centre it so the coverage
    // probe below hits a real point rather than an empty viewport.
    panel.scrollIntoView({ block: 'center', inline: 'nearest' });
    const rect = (node) => {
      const box = node.getBoundingClientRect();
      return { x: box.x, y: box.y, width: box.width, height: box.height, right: box.right, bottom: box.bottom };
    };
    const panelBox = rect(panel);
    const skeletonBox = skeleton ? rect(skeleton) : null;
    const headerBottom = document.querySelector('.app-top')?.getBoundingClientRect().bottom ?? 0;
    const probeX = Math.round(panelBox.x + Math.min(panelBox.width / 2, 60));
    const probeY = Math.round(Math.max(panelBox.y + 8, headerBottom + 8));
    const insidePanel = probeY > panelBox.y && probeY < panelBox.y + panelBox.height;
    const topNode = insidePanel ? document.elementFromPoint(probeX, probeY) : null;
    return {
      loading: panel.classList.contains('loading'),
      busy: panel.getAttribute('aria-busy'),
      stale: panel.classList.contains('is-stale'),
      text: panel.innerText.replace(/\s+/g, ' ').trim(),
      panel: panelBox,
      skeleton: skeletonBox,
      coversPanel: Boolean(skeletonBox)
        && Math.abs(skeletonBox.x - panelBox.x) <= 1 && Math.abs(skeletonBox.y - panelBox.y) <= 1
        && Math.abs(skeletonBox.width - panelBox.width) <= 1 && Math.abs(skeletonBox.height - panelBox.height) <= 1,
      skeletonOnTop: Boolean(topNode?.closest('[data-results-skeleton]')),
      placeholderCount: bars.length,
      placeholdersHidden: bars.length > 0 && bars.every((bar) => bar.getAttribute('aria-hidden') === 'true'),
      skeletonHidden: skeleton?.getAttribute('aria-hidden') === 'true',
      barsInsidePanel: bars.every((bar) => bar.getBoundingClientRect().right <= skeletonBox.right + 1),
      coveredCount: covered.length,
      coveredInert: covered.length > 0 && covered.every((node) => node.inert === true && node.getAttribute('aria-hidden') === 'true'),
      announcer: document.querySelector('[data-results-announcer]')?.textContent ?? null,
      announcerRole: document.querySelector('[data-results-announcer]')?.getAttribute('role') ?? null,
      mobileResult: document.querySelector('[data-mobile-result]')?.textContent ?? null,
      applyEnabled: panel.querySelectorAll('[data-apply-index]:not([disabled])').length,
      context: currentResultContext ? 'set' : null,
      pseudoAfter: getComputedStyle(panel, '::after').content,
      panelScrolls: panel.scrollHeight > panel.clientHeight + 1,
      pageScrollWidth: document.documentElement.scrollWidth,
      viewportWidth: window.innerWidth,
      animationName: bars.length ? getComputedStyle(bars[0]).animationName : null,
    };
  });
}

test('the initial calculation is covered by an opaque skeleton until fresh results arrive', async () => {
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  const stub = await openLoading(page);
  const state = await loadingState(page);

  assert.equal(state.loading, true, 'the panel is in its loading state before any response');
  assert.equal(state.busy, 'true', 'aria-busy reports the pending calculation');
  assert.ok(state.skeleton, 'a skeleton surface is present');
  assert.equal(state.coversPanel, true, 'the skeleton covers the whole panel');
  assert.equal(state.skeletonOnTop, true, 'the skeleton is the topmost element over the panel');
  assert.equal(state.skeletonHidden, true, 'the skeleton surface itself is hidden from assistive tech');
  assert.ok(state.placeholderCount >= 12, `expected placeholder shapes, saw ${state.placeholderCount}`);
  assert.equal(state.placeholdersHidden, true, 'every placeholder is aria-hidden');
  assert.equal(state.coveredCount > 0, true, 'the previous markup stays in the panel');
  assert.equal(state.coveredInert, true, 'the covered markup is inert and hidden from assistive tech');
  assert.equal(state.applyEnabled, 0, 'no apply action is usable while loading');
  assert.equal(state.context, null, 'no apply context exists while loading');
  assert.equal(state.announcer, 'Recalculating results', 'the loading state is announced once');
  assert.equal(state.announcerRole, 'status');
  assert.match(state.mobileResult, /Recalculating/);
  assert.equal(state.pseudoAfter, 'none', 'the old Recalculating ::after label is gone');
  assert.equal(state.panelScrolls, false, 'the skeleton does not make the panel scroll');
  assert.equal(state.pageScrollWidth, state.viewportWidth, 'the skeleton adds no horizontal overflow');
  assert.equal(stub.requests.length, 1);

  await releaseAndSettle(page, stub, 0, 200, { matches: [], warnings: ['FRESH-A'] });
  const done = await loadingState(page);
  assert.equal(done.loading, false);
  assert.equal(done.busy, 'false');
  assert.equal(done.skeleton, null, 'fresh results replace the skeleton');
  assert.equal(done.coveredCount, 0);
  assert.equal(done.announcer, '', 'the loading announcement is cleared once results land');
  assert.match(done.text, /FRESH-A/);
  await page.close();
});

test('existing rows stay covered and inert while a newer calculation is scheduled', async () => {
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  const stub = await openWithResults(page);
  const defenderBefore = await page.locator('[data-set-card="defender"] .raw-editor').inputValue();
  assert.equal((await loadingState(page)).applyEnabled, ROWS.length, 'fresh rows are applicable');

  // Editing schedules the next run; the skeleton is up before its debounce fires.
  await page.locator('[name="limit"]').fill('24');
  const state = await loadingState(page);
  assert.equal(state.loading, true, 'scheduling a valid input shows the skeleton immediately');
  assert.equal(stub.requests.length, 1, 'the debounce has not submitted the replacement yet');
  assert.equal(state.applyEnabled, 0, 'the displayed rows lost their apply actions');
  assert.equal(state.context, null);
  assert.equal(state.skeletonOnTop, true, 'the old rows are covered');
  assert.equal(state.coveredInert, true, 'the old rows are inert');
  assert.match(state.text, /Bold/, 'the previous rows are still the panel content underneath');

  // A forced click cannot use a row the user can no longer see or reach.
  await page.evaluate(() => document.querySelector('.table-card tbody tr:nth-child(2) [data-apply-index]').click());
  assert.equal(await page.locator('[data-set-card="defender"] .raw-editor').inputValue(), defenderBefore);

  await pollUntil(() => stub.requests.length === 2, 'replacement run');
  assert.equal(stub.requests[1].body.limit, 24);
  await releaseAndSettle(page, stub, 0, 200, { matches: ROWS, best: ROWS[0] });
  const done = await loadingState(page);
  assert.equal(done.skeleton, null);
  assert.equal(done.applyEnabled, ROWS.length, 'fresh rows are applicable again');
  assert.equal(done.context, 'set');
  await page.close();
});

test('a pending replacement keeps the skeleton through a superseded response', async () => {
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  const stub = await openWithResults(page);

  await page.locator('[name="limit"]').fill('26');
  await pollUntil(() => stub.requests.length === 2, 'second run');
  await page.locator('[name="limit"]').fill('27');

  // The superseded response settles while the newer input is still inside its
  // debounce window: it must not render or uncover anything.
  stub.release(0, 200, { matches: [], warnings: ['STALE-OUTPUT'] });
  await page.waitForTimeout(DEBOUNCE_MS / 2);
  const during = await loadingState(page);
  assert.equal(during.loading, true, 'the obsolete response does not end loading');
  assert.ok(during.skeleton, 'the skeleton stays up for the pending replacement');
  assert.equal(during.busy, 'true');
  assert.equal(during.skeletonOnTop, true);
  assert.doesNotMatch(during.text, /STALE-OUTPUT/, 'no obsolete response is rendered');
  assert.match(during.text, /Bold/, 'the covered content is still the last fresh result');

  await pollUntil(() => stub.requests.length === 3, 'pending replacement run');
  assert.equal(stub.requests[2].body.limit, 27);
  assert.equal((await loadingState(page)).loading, true, 'loading continues into the pending run');

  await releaseAndSettle(page, stub, 0, 200, { matches: [], warnings: ['FRESH-B'] });
  const done = await loadingState(page);
  assert.equal(done.skeleton, null);
  assert.match(done.text, /FRESH-B/);
  await page.close();
});

test('the skeleton covers best-damage enrichment and the run it precedes', async () => {
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  const stub = await stubSurvive(page);
  const damage = await stubDamage(page);
  await page.goto(`${baseURL}/survive`);
  await pollUntil(() => stub.requests.length === 1, 'initial auto-run');
  await page.waitForSelector('.results-panel.loading [data-results-skeleton]');

  stub.release(0, 200, { matches: ROWS, best: ROWS[0] });
  await pollUntil(() => damage.requests === 1, 'best-damage enrichment');
  const enriching = await loadingState(page);
  assert.equal(enriching.loading, true, 'the workflow is still loading during enrichment');
  assert.ok(enriching.skeleton);
  assert.equal(enriching.applyEnabled, 0);

  // Editing during enrichment keeps the skeleton and must not overlap requests.
  await page.locator('[name="limit"]').fill('22');
  await page.waitForTimeout(DEBOUNCE_MS + 200);
  assert.equal(stub.requests.length, 1, 'no overlapping calculation while enrichment is in flight');
  assert.equal((await loadingState(page)).loading, true, 'the pending edit keeps the skeleton');

  damage.release(0, 200, { rolls: [10, 20] });
  await pollUntil(() => stub.requests.length === 2, 'latest run after enrichment');
  assert.equal(stub.requests[1].body.limit, 22);
  assert.equal((await loadingState(page)).loading, true, 'loading carries into the replacement run');

  await releaseAndSettle(page, stub, 0, 200, { matches: [], warnings: ['FRESH'] });
  const done = await loadingState(page);
  assert.equal(done.skeleton, null);
  assert.match(done.text, /FRESH/);
  await page.close();
});

test('fresh success, error, empty, and damage-summary responses all clear the skeleton', async () => {
  const cases = [
    { name: 'success', status: 200, body: { matches: ROWS, best: ROWS[0] }, selector: '.table-card' },
    { name: 'error', status: 503, body: { error: 'Server is busy' }, selector: '.error-card' },
    { name: 'empty', status: 200, body: { matches: [] }, selector: '.empty-state' },
    { name: 'summary', status: 200, body: { summary: { min_damage: 10, max_damage: 20, percent_min: 12.5, percent_max: 25, ko_chance: 0.5 }, rolls: [10, 20] }, selector: '.damage-card' },
  ];
  for (const item of cases) {
    const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
    const stub = await openLoading(page);
    await releaseAndSettle(page, stub, 0, item.status, item.body);
    const state = await loadingState(page);
    assert.equal(state.loading, false, `${item.name}: loading class cleared`);
    assert.equal(state.busy, 'false', `${item.name}: aria-busy cleared`);
    assert.equal(state.skeleton, null, `${item.name}: skeleton removed`);
    assert.equal(state.coveredCount, 0, `${item.name}: no covered markup remains`);
    assert.equal(await page.locator(item.selector).count(), 1, `${item.name}: result presentation rendered`);
    await page.close();
  }
});

test('invalid input shows the waiting state and never revives the skeleton', async () => {
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  const stub = await openLoading(page);

  await page.evaluate(() => {
    document.querySelector('[name="attacker_set"]').value = '';
    autoRun();
  });
  await page.waitForSelector('.results-panel .empty-state h2');
  const waiting = await loadingState(page);
  assert.equal(waiting.loading, false, 'invalid input leaves the loading state');
  assert.equal(waiting.busy, 'false');
  assert.equal(waiting.skeleton, null);
  assert.equal(waiting.stale, false);
  assert.match(waiting.text, /Add a move/);
  assert.equal(waiting.announcer, '', 'the loading announcement is cleared');

  // The obsolete request settles while the inputs are still invalid.
  stub.release(0, 200, { matches: ROWS, best: ROWS[0] });
  await page.waitForTimeout(250);
  const afterSettle = await loadingState(page);
  assert.equal(afterSettle.skeleton, null, 'a settled obsolete run cannot revive the skeleton');
  assert.equal(afterSettle.loading, false);
  assert.equal(afterSettle.busy, 'false');
  assert.match(afterSettle.text, /Add a move/, 'the waiting state survives');
  assert.doesNotMatch(afterSettle.text, /Bold/, 'no obsolete rows are published');
  assert.equal(await page.locator('.error-card').count(), 0);
  assert.equal(stub.requests.length, 1, 'no calculation was started for the invalid input');
  await page.close();
});

test('restoring a valid input while obsolete work runs shows the pending skeleton', async () => {
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  const stub = await openLoading(page);

  await page.evaluate(() => {
    document.querySelector('[name="attacker_set"]').value = '';
    autoRun();
  });
  await page.waitForSelector('.results-panel .empty-state h2');

  // The inputs become valid again before the obsolete request settles.
  await page.evaluate(() => {
    document.querySelector('[name="attacker_set"]').value = 'Kingambit @ Black Glasses\nAbility: Defiant\nAdamant Nature\n- Iron Head';
    document.querySelector('[name="limit"]').value = '21';
    autoRun();
  });
  const recovering = await loadingState(page);
  assert.equal(recovering.loading, true, 'a valid pending input shows the skeleton immediately');
  assert.ok(recovering.skeleton);
  assert.equal(recovering.busy, 'true');
  assert.equal(recovering.skeletonOnTop, true);

  await page.waitForTimeout(DEBOUNCE_MS + 250);
  assert.equal(stub.requests.length, 1, 'the restored input waits for the obsolete workflow');
  assert.equal((await loadingState(page)).loading, true, 'the skeleton is held through that wait');

  stub.release(0, 200, { matches: [], warnings: ['OBSOLETE'] });
  await pollUntil(() => stub.requests.length === 2, 'restored run after the obsolete workflow settles');
  assert.equal(stub.requests[1].body.limit, 21);
  assert.equal((await loadingState(page)).loading, true);

  await releaseAndSettle(page, stub, 0, 200, { matches: [], warnings: ['FRESH'] });
  const done = await loadingState(page);
  assert.equal(done.skeleton, null);
  assert.match(done.text, /FRESH/);
  assert.doesNotMatch(done.text, /OBSOLETE/);
  await page.close();
});

test('input controls stay interactive while the skeleton is loading', async () => {
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  const errors = [];
  page.on('pageerror', (error) => errors.push(error.message));
  const stub = await openLoading(page);

  await page.locator('[name="limit"]').fill('23');
  await page.locator('[data-move="Knock Off"] .move-select').click();
  await page.locator('[data-crit-move="Knock Off"]').check();
  await page.locator('[data-set-card="attacker"] [data-ability-toggle]').uncheck();
  await page.locator('[name="terrain"][value="Electric"]').check();

  assert.equal(await page.locator('[name="limit"]').inputValue(), '23');
  assert.equal(await page.locator('[data-move="Knock Off"] .move-select').getAttribute('aria-pressed'), 'true');
  assert.equal(await page.locator('[data-crit-move="Knock Off"]').isChecked(), true);
  assert.equal(await page.locator('[data-set-card="attacker"] [data-ability-toggle]').isChecked(), false);
  assert.equal(await page.locator('[name="terrain"][value="Electric"]').isChecked(), true);
  const state = await loadingState(page);
  assert.equal(state.loading, true, 'the skeleton keeps covering the results while inputs stay live');
  assert.equal(state.skeletonOnTop, true);
  assert.deepEqual(errors, []);

  // The edits coalesced into one pending run behind the held request.
  assert.equal(stub.requests.length, 1);
  await page.waitForTimeout(DEBOUNCE_MS + 200);
  assert.equal(stub.requests.length, 1, 'no overlapping request while the active one is held');

  stub.release(0, 200, { matches: [], warnings: ['AFTER-EDITS'] });
  await pollUntil(() => stub.requests.length === 2, 'coalesced follow-up run');
  assert.equal(stub.requests[1].body.limit, 23);
  assert.equal(stub.requests[1].body.move_name, 'Knock Off');
  await releaseAndSettle(page, stub, 0, 200, { matches: [], warnings: ['FRESH'] });
  assert.equal((await loadingState(page)).loading, false);
  await page.close();
});

test('the expanded damage-rolls preference survives a skeleton episode', async () => {
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  const rows = [{ ...ROWS[0], rolls: [25, 27, 30] }];
  const stub = await stubSurvive(page, { damageRolls: [25, 27, 30] });
  await page.goto(`${baseURL}/survive`);
  await pollUntil(() => stub.requests.length === 1, 'initial auto-run');
  await releaseAndSettle(page, stub, 0, 200, { matches: rows, best: rows[0] });
  await page.waitForSelector('.damage-rolls');
  await page.locator('.damage-rolls summary').click();
  assert.equal(await page.locator('.damage-rolls').evaluate((node) => node.open), true);

  await page.locator('[name="limit"]').fill('21');
  assert.equal((await loadingState(page)).loading, true);
  await pollUntil(() => stub.requests.length === 2, 'recalculation after the edit');
  await releaseAndSettle(page, stub, 0, 200, { matches: rows, best: rows[0] });
  assert.equal(await page.locator('.damage-rolls').evaluate((node) => node.open), true,
    'the expanded damage rolls stay expanded across loading');
  await page.close();
});

test('loading is announced once per episode and cleared with the results', async () => {
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  const stub = await openLoading(page);
  const first = await loadingState(page);
  assert.equal(first.announcer, 'Recalculating results');
  assert.equal(await page.locator('[role="status"]').count(), 1, 'a single status region carries the announcement');

  // A superseded response inside the same episode must not announce again; the
  // region keeps one message until the episode ends.
  await page.locator('[name="limit"]').fill('26');
  stub.release(0, 200, { matches: [], warnings: ['STALE'] });
  await page.waitForTimeout(DEBOUNCE_MS / 2);
  const during = await loadingState(page);
  assert.equal(during.announcer, 'Recalculating results');

  await pollUntil(() => stub.requests.length === 2, 'pending run');
  await releaseAndSettle(page, stub, 0, 200, { matches: [], warnings: ['FRESH'] });
  assert.equal((await loadingState(page)).announcer, '', 'the announcement clears with the skeleton');

  // A later episode announces again.
  await page.locator('[name="limit"]').fill('27');
  const second = await loadingState(page);
  assert.equal(second.announcer, 'Recalculating results', 'the next episode announces again');
  await page.close();
});

test('reduced motion removes the placeholder pulse', async () => {
  const reduced = await browser.newPage({ viewport: { width: 1280, height: 800 }, reducedMotion: 'reduce' });
  await openLoading(reduced);
  assert.equal((await loadingState(reduced)).animationName, 'none', 'no pulse under prefers-reduced-motion');
  await reduced.close();

  const moving = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  await openLoading(moving);
  assert.equal((await loadingState(moving)).animationName, 'skeleton-pulse');
  await moving.close();
});

test('the skeletal layout stays inside a mobile viewport', async () => {
  const page = await browser.newPage({ viewport: { width: 390, height: 844 }, hasTouch: true });
  await openLoading(page);
  const state = await loadingState(page);
  assert.equal(state.pageScrollWidth, 390, 'no horizontal overflow on mobile');
  assert.equal(state.coversPanel, true, 'the skeleton covers the full mobile panel');
  assert.equal(state.skeletonOnTop, true, 'the skeleton covers the mobile panel surface');
  assert.equal(state.barsInsidePanel, true, 'placeholders stay within the panel width');
  assert.equal(state.panel.x >= 0 && state.panel.x + state.panel.width <= 390, true, 'the panel stays inside the viewport');
  await page.close();
});
