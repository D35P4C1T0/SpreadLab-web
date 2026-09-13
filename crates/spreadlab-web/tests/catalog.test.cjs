const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const { test } = require('node:test');

// Exercise the browser's pure catalog functions without a DOM dependency.
const source = fs.readFileSync(path.join(__dirname, '../assets/app.js'), 'utf8');
const names = ['setPokemonList', 'baseSpeciesName', 'defaultSpeciesForBase', 'canonicalSpeciesName', 'sortedUniqueNames', 'normalizeName'];
const functions = names.map(name => {
  const start = source.indexOf(`function ${name}(`);
  assert.ok(start >= 0, name);
  const end = source.indexOf('\n}', start) + 2;
  return source.slice(start, end);
}).join('\n');
const stones = source.slice(source.indexOf('const megaStonePokemon ='), source.indexOf('const storageKey ='));
const context = vm.createContext({});
vm.runInContext(`let speciesList, speciesForms, pokemonList, pokemonSearchList;
function rebuildPokemonOptionSearchList() {}
${functions}
${stones}`, context);
const evaluate = code => vm.runInContext(code, context);

test('M-C form imports select canonical forms and stable defaults', () => {
  const aliases = {
    'Persian-Alola': 'Persian (Alolan)',
    'Toxtricity': 'Toxtricity (Amped Form)',
    'Toxtricity-Low-Key': 'Toxtricity (Low Key Form)',
    'Indeedee': 'Indeedee (Male)',
    'Indeedee-F': 'Indeedee (Female)',
    'Squawkabilly': 'Squawkabilly (Green Plumage)',
    'Squawkabilly-Blue': 'Squawkabilly (Blue Plumage)',
    'Squawkabilly-Yellow': 'Squawkabilly (Yellow Plumage)',
    'Squawkabilly-White': 'Squawkabilly (White Plumage)',
  };
  evaluate(`setPokemonList(${JSON.stringify(Object.values(aliases))})`);
  for (const [alias, canonical] of Object.entries(aliases)) {
    assert.equal(evaluate(`canonicalSpeciesName(${JSON.stringify(alias)})`), canonical);
  }
});

test('all six M-C Mega Stones select their correct Mega', () => {
  for (const [stone, species] of Object.entries({
    'Absolite Z': 'Mega Absol Z', 'Salamencite': 'Mega Salamence',
    'Garchompite Z': 'Mega Garchomp Z', 'Lucarionite Z': 'Mega Lucario Z',
    'Golisopite': 'Mega Golisopod', 'Baxcalibrite': 'Mega Baxcalibur',
  })) {
    assert.equal(evaluate(`megaStonePokemon[${JSON.stringify(stone)}]`), species);
  }
});
