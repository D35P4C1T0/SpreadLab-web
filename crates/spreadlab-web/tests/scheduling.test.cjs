// Deterministic browser tests for the calculation scheduler in assets/app.js.
//
// Every calculation request is intercepted and answered by the test, so the
// assertions describe the scheduler rather than optimizer runtime. Run against a
// live dev server, e.g.:
//
//   SPREADLAB_URL=http://127.0.0.1:3210 \
//   CHROMIUM_PATH=~/Library/Caches/ms-playwright/chromium_headless_shell-1178/chrome-mac/headless_shell \
//   node --test crates/spreadlab-web/tests/scheduling.test.cjs
const { test, before, after } = require('node:test');
const assert = require('node:assert/strict');
const { chromium } = require('playwright');

const baseURL = process.env.SPREADLAB_URL || 'http://127.0.0.1:3000';
const DEBOUNCE_MS = 320;

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

// Installs a controllable /api/survive stub. Requests stay in flight until the
// test releases them, so the scheduler sees a real slow calculation.
async function stubSurvive(page) {
  const state = { requests: [], inflight: 0, maxInflight: 0, pending: [] };
  await page.route('**/api/survive', route => {
    state.requests.push({ body: JSON.parse(route.request().postData() || '{}'), at: Date.now() });
    state.inflight += 1;
    state.maxInflight = Math.max(state.maxInflight, state.inflight);
    return new Promise(resolve => {
      state.pending.push(resolve);
    }).then(({ status, json }) => {
      state.inflight -= 1;
      return json === undefined
        ? route.fulfill({ status })
        : route.fulfill({ status, json });
    });
  });
  // The best-damage enrichment is part of the same workflow; answer it instantly.
  await page.route('**/api/damage', route => route.fulfill({ json: { rolls: [] } }));
  state.release = (index, status, json) => state.pending.splice(index, 1)[0]({ status, json });
  return state;
}

// Registers a later /api/damage route that holds enrichment open until released.
// Playwright checks routes in reverse registration order, so this overrides the
// instant stub installed by stubSurvive.
async function stubDamage(page) {
  const state = { requests: 0, inflight: 0, pending: [] };
  await page.route('**/api/damage', route => {
    state.requests += 1;
    state.inflight += 1;
    return new Promise(resolve => {
      state.pending.push(resolve);
    }).then(({ status, json }) => {
      state.inflight -= 1;
      return route.fulfill({ status, json });
    });
  });
  state.release = (index, status, json) => state.pending.splice(index, 1)[0]({ status, json });
  return state;
}

async function panelText(page) {
  return page.locator('.results-panel').innerText();
}

test('a burst of edits keeps one active request and submits only the latest input', async () => {
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  const stub = await stubSurvive(page);
  await page.goto(`${baseURL}/survive`);
  await pollUntil(() => stub.requests.length === 1, 'initial auto-run');
  assert.equal(stub.inflight, 1, 'one active request after load');

  // Three quick edits while the first request is still in flight.
  await page.locator('[name="limit"]').fill('11');
  await page.locator('[name="limit"]').fill('12');
  await page.locator('[name="limit"]').fill('13');
  await page.waitForTimeout(DEBOUNCE_MS + 300);

  assert.equal(stub.requests.length, 1, 'debounce must not launch a second request while one is active');
  assert.equal(stub.maxInflight, 1, 'never more than one active calculation');

  // Releasing the active run must submit exactly one follow-up with the latest input.
  stub.release(0, 200, { matches: [], warnings: ['FIRST'] });
  await pollUntil(() => stub.requests.length === 2, 'single coalesced follow-up');
  assert.equal(stub.requests[1].body.limit, 13, 'follow-up carries the latest input');

  stub.release(0, 200, { matches: [], warnings: ['LATEST'] });
  await page.waitForFunction(() => document.querySelector('.results-panel').getAttribute('aria-busy') === 'false');
  const text = await panelText(page);
  assert.match(text, /LATEST/);
  assert.doesNotMatch(text, /FIRST/, 'obsolete first response must not be rendered');
  assert.equal(stub.requests.length, 2);
  await page.close();
});

test('stale success responses do not overwrite current results', async () => {
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  const stub = await stubSurvive(page);
  await page.goto(`${baseURL}/survive`);

  await pollUntil(() => stub.requests.length === 1, 'initial auto-run');
  stub.release(0, 200, { matches: [], warnings: ['FRESH-A'] });
  await page.waitForFunction(() => document.querySelector('.results-panel').innerText.includes('FRESH-A'));

  await page.locator('[name="limit"]').fill('14');
  await pollUntil(() => stub.requests.length === 2, 'second run');
  // Newer input arrives while the second run is in flight: it must be invalidated.
  await page.locator('[name="limit"]').fill('15');
  assert.match(await page.locator('.results-panel').getAttribute('class'), /is-stale/);

  stub.release(0, 200, { matches: [], warnings: ['STALE'] });
  await page.waitForTimeout(200);
  const duringStale = await panelText(page);
  assert.match(duringStale, /FRESH-A/, 'stale response must not replace displayed results');
  assert.doesNotMatch(duringStale, /STALE/);

  await pollUntil(() => stub.requests.length === 3, 'latest input run');
  assert.equal(stub.requests[2].body.limit, 15);
  stub.release(0, 200, { matches: [], warnings: ['FRESH-B'] });
  await page.waitForFunction(() => document.querySelector('.results-panel').innerText.includes('FRESH-B'));
  assert.doesNotMatch(await page.locator('.results-panel').getAttribute('class'), /is-stale/);
  await page.close();
});

test('stale error responses do not overwrite current results or the input display', async () => {
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  const stub = await stubSurvive(page);
  await page.goto(`${baseURL}/survive`);

  await pollUntil(() => stub.requests.length === 1, 'initial auto-run');
  stub.release(0, 200, { matches: [], warnings: ['FRESH-A'] });
  await page.waitForFunction(() => document.querySelector('.results-panel').innerText.includes('FRESH-A'));

  await page.locator('[name="limit"]').fill('16');
  await pollUntil(() => stub.requests.length === 2, 'second run');
  await page.locator('[name="limit"]').fill('17');
  stub.release(0, 503, { error: 'Server is busy' });
  await page.waitForTimeout(200);

  assert.equal(await page.locator('.error-card').count(), 0, 'stale error must not render an error card');
  assert.match(await panelText(page), /FRESH-A/);

  await pollUntil(() => stub.requests.length === 3, 'latest input run');
  stub.release(0, 200, { matches: [], warnings: ['FRESH-B'] });
  await page.waitForFunction(() => document.querySelector('.results-panel').innerText.includes('FRESH-B'));
  assert.equal(await page.locator('.error-card').count(), 0);
  await page.close();
});

test('explicit submit consumes the pending debounce without duplicating requests', async () => {
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  const stub = await stubSurvive(page);
  await page.goto(`${baseURL}/survive`);

  await pollUntil(() => stub.requests.length === 1, 'initial auto-run');
  stub.release(0, 200, { matches: [], warnings: ['FRESH-A'] });
  await page.waitForFunction(() => document.querySelector('.results-panel').getAttribute('aria-busy') === 'false');

  await page.locator('[name="limit"]').fill('18');
  await page.locator('.primary').click();
  await pollUntil(() => stub.requests.length === 2, 'explicit submit run');
  assert.equal(stub.requests[1].body.limit, 18);

  // The debounce timer must have been consumed, not fired a duplicate.
  await page.waitForTimeout(DEBOUNCE_MS + 400);
  assert.equal(stub.requests.length, 2, 'explicit submit must not duplicate the pending run');

  stub.release(0, 200, { matches: [], warnings: ['FRESH-B'] });
  await page.waitForFunction(() => document.querySelector('.results-panel').innerText.includes('FRESH-B'));
  await page.close();
});

test('invalid input clears pending work, invalidates results, and recovers', async () => {
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  const stub = await stubSurvive(page);
  await page.goto(`${baseURL}/survive`);

  await pollUntil(() => stub.requests.length === 1, 'initial auto-run');
  stub.release(0, 200, { matches: [], warnings: ['FRESH-A'] });
  await page.waitForFunction(() => document.querySelector('.results-panel').innerText.includes('FRESH-A'));

  await page.evaluate(() => {
    document.querySelector('[name="attacker_set"]').value = '';
    autoRun();
  });
  await page.waitForSelector('.results-panel .empty-state h2');
  assert.match(await panelText(page), /Add a move/);
  await page.waitForTimeout(DEBOUNCE_MS + 400);
  assert.equal(stub.requests.length, 1, 'invalid state must not schedule a run');
  assert.equal(await page.locator('.results-panel').getAttribute('aria-busy'), 'false');

  await page.evaluate(() => {
    document.querySelector('[name="attacker_set"]').value = 'Kingambit @ Black Glasses\nAbility: Defiant\nAdamant Nature\n- Iron Head';
    autoRun();
  });
  await pollUntil(() => stub.requests.length === 2, 'run after recovery');
  stub.release(0, 200, { matches: [], warnings: ['FRESH-B'] });
  await page.waitForFunction(() => document.querySelector('.results-panel').innerText.includes('FRESH-B'));
  await page.close();
});

test('invalid input during an active request keeps the workflow held until it settles', async () => {
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  const stub = await stubSurvive(page);
  await page.goto(`${baseURL}/survive`);
  await pollUntil(() => stub.requests.length === 1, 'initial auto-run');

  // Invalidate while the first request is still in flight. The already-running
  // server job must keep its slot; the client must not abort and resubmit.
  await page.evaluate(() => {
    document.querySelector('[name="attacker_set"]').value = '';
    autoRun();
  });
  await page.waitForSelector('.results-panel .empty-state h2');
  assert.match(await panelText(page), /Add a move/);
  assert.equal(await page.locator('.results-panel').getAttribute('aria-busy'), 'false');

  // Restore a valid input before the obsolete request is released.
  await page.evaluate(() => {
    document.querySelector('[name="attacker_set"]').value = 'Kingambit @ Black Glasses\nAbility: Defiant\nAdamant Nature\n- Iron Head';
    document.querySelector('[name="limit"]').value = '21';
    autoRun();
  });
  await page.waitForTimeout(DEBOUNCE_MS + 350);
  assert.equal(stub.requests.length, 1, 'must not submit until the obsolete workflow settles');

  stub.release(0, 200, { matches: [], warnings: ['STALE-OUTPUT'] });
  await pollUntil(() => stub.requests.length === 2, 'latest run after the obsolete workflow settles');
  assert.equal(stub.requests[1].body.limit, 21, 'latest restored input is submitted once');

  stub.release(0, 200, { matches: [], warnings: ['FRESH'] });
  await page.waitForFunction(() => document.querySelector('.results-panel').innerText.includes('FRESH'));
  assert.doesNotMatch(await panelText(page), /STALE-OUTPUT/);
  await page.close();
});

test('active workflow stays held through best-damage enrichment and stale enrichment is suppressed', async () => {
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  const stub = await stubSurvive(page);
  const damage = await stubDamage(page);
  await page.goto(`${baseURL}/survive`);
  await pollUntil(() => stub.requests.length === 1, 'initial auto-run');

  const best = {
    rank: 1,
    nature: 'Bold',
    sp_line: 'SPs: 32 HP / 32 Def',
    total_points: 64,
    result: { ko_chance: 0.1, min_damage: 1, max_damage: 2, percent_min: 10, percent_max: 20 },
  };
  stub.release(0, 200, { matches: [best], best });
  await pollUntil(() => damage.requests === 1, 'best-damage enrichment');
  assert.equal(damage.inflight, 1);

  // The workflow is still held while enrichment is in flight, even after edits.
  await page.locator('[name="limit"]').fill('22');
  await page.waitForTimeout(DEBOUNCE_MS + 300);
  assert.equal(stub.requests.length, 1, 'must not overlap while enrichment is in flight');

  damage.release(0, 200, { rolls: [10, 20] });
  await pollUntil(() => stub.requests.length === 2, 'latest run after enrichment settles');
  assert.equal(stub.requests[1].body.limit, 22);
  const previews = await page.locator('[data-set-card="defender"] [data-preview-sp]').allInnerTexts();
  assert.deepEqual(
    previews,
    ['26', '0', '13', '5', '0', '22'],
    'the input display must reflect the current set, never the stale best row'
  );

  stub.release(0, 200, { matches: [], warnings: ['FRESH'] });
  await page.waitForFunction(() => document.querySelector('.results-panel').innerText.includes('FRESH'));
  await page.close();
});

test('a busy error unlocks the scheduler without auto-retrying', async () => {
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  const stub = await stubSurvive(page);
  await page.goto(`${baseURL}/survive`);

  await pollUntil(() => stub.requests.length === 1, 'initial auto-run');
  stub.release(0, 200, { matches: [], warnings: ['FRESH-A'] });
  await page.waitForFunction(() => document.querySelector('.results-panel').innerText.includes('FRESH-A'));

  await page.locator('[name="limit"]').fill('19');
  await pollUntil(() => stub.requests.length === 2, 'busy run');
  stub.release(0, 503, { error: 'Server is busy; another optimizer calculation is already running. Retry in a moment.' });
  await page.waitForSelector('.error-card');

  // No persistent auto-retry loop for busy responses.
  await page.waitForTimeout(900);
  assert.equal(stub.requests.length, 2, 'busy response must not auto-retry');

  // The scheduler is unlocked: a new edit submits again.
  await page.locator('[name="limit"]').fill('20');
  await pollUntil(() => stub.requests.length === 3, 'recovery after busy');
  assert.equal(stub.requests[2].body.limit, 20);
  stub.release(0, 200, { matches: [], warnings: ['RECOVERED'] });
  await page.waitForFunction(() => document.querySelector('.results-panel').innerText.includes('RECOVERED'));
  await page.close();
});
