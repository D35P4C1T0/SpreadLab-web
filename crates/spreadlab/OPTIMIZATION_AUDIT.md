# Survival optimization audit

This records the pre-fix findings. The replacement is now implemented; see
[the search contract and migration guide](docs/survival-search.md).

## Reproduction

Run `cargo run --offline --example audit_survival` against the pinned damage engine.
The fixture uses Hardy Dragapult with 32 SpA and Life Orb, Hardy Mega Salamence,
Dragon Pulse, full HP, and a maximum KO probability of 0.25.

Both reported damage-roll lists reproduce exactly. Existing search returns
26 HP / 0 SpD (26 points, 25% KO). The proposed 19 HP / 4 SpD costs 23 points
at the same probability. Exhaustively evaluating all 1,089 HP/SpD pairs finds
a unique minimum in that domain: **5 HP / 14 SpD, 19 points, 18.75% KO**.
This is a fixed-neutral-nature result, not a claim across every nature or battle configuration.
These are Champions Stat Points (SPs), not conventional EV units.

## Correct optimization contract

Fix the attacker, defender identity, items, abilities, boosts, field, starting-HP
policy, allowed natures N, and battle-model semantics before searching.
Let x = (h, d, s) be HP, Defense, and Special Defense investment; let L be
the total locked investment in other stats. Define the legal domain:

    n in N
    h, d, s are integers in [0, 32]
    h + d + s + L <= 66
    all caller-specified locks and lower bounds hold

Minimize C(x) = h + d + s + L subject to P(KO | x, n, context) <= alpha.
If minimizing added investment instead, explicitly define the baseline and
subtract it; do not silently erase it. Alpha is a probability in [0, 1].

For one ordinary hit with 16 equiprobable rolls and no HP-changing mechanics:

    H(x) = starting HP under the selected policy
    P(KO | x) = count(i: damage_i(x) >= H(x)) / 16

Equality is a KO. For alpha = 0.25, at most four rolls may KO. With rolls sorted
ascending, this means H(x) > damage_12(x), using one-based indexing. Comparing
maximum damage percentages alone cannot decide this constraint.

Define output semantics separately:

- All global minima: every feasible (x, n) with cost C*.
- Top K feasible spreads: sort by cost, then documented tie-breakers; may include
  nonminimal costs. This is the current `matches` shape.
- Pareto frontier: spreads undominated in cost and KO probability; a different query.

Multiple independent benchmarks require every individual constraint to hold.
Surviving an ordered sequence requires probability of remaining alive through
that sequence under an explicit state-transition model. These are different
problems; neither is equivalent to summing arbitrary benchmark scores.

## Confirmed flaws and contract gaps

1. **Missing search dimension.** `src/optimize.rs`, both survival loops, constructs
   `StatPoints::new(hp, 0, defense, 0, 0, 0)`. SpD never varies. The search is
   exhaustive only on the HP/Defense plane. Ordinary special damage cannot
   benefit from the Defense points being tested, explaining all-HP results.
   Physical mixed allocations already exist: the existing Iron Head test expects
   4 HP / 20 Defense. There is no greedy allocation or pruning bug here.

2. **Different objective in the general optimizer.** `score_result` maximizes
   `10000 * (1 - KO) - max_percent` for defense, summed across benchmarks.
   Total points break only equal scores. This can deliberately spend more points
   for better survival and permits tradeoffs between benchmarks. It cannot
   promise minimum points subject to a KO ceiling. The finite weighted score
   is also not a strict lexicographic ordering of KO probability and damage.

3. **Nature defaults change the problem.** In `src/api.rs`, no explicit nature
   plus `optimize_nature: false` searches all 25 natures, rather than preserving
   the parsed defender nature. `true` narrows to Bold/Calm according to category.
   Nature restrictions must be explicit; comparisons between different allowed
   nature sets are not comparisons of the same optimization problem.

4. **Category is not a complete stat-dependency model.** The pinned engine's
   `damage.rs` uses `deals_physical_damage` for defensive stat selection;
   Psyshock is Special with that flag set. Category-only defensive nature
   selection chooses Calm incorrectly for this case. Offensive search has the
   same structural issue: Body Press uses Defense, but search invests Attack;
   Foul Play uses the defender's attack source. Merely replacing Defense with
   SpD for every Special move is not a general fix.

5. **Existing allocation is discarded.** Survival candidates replace the entire
   defender allocation with their two searched coordinates. Requests have no
   survival-search locks or remaining-budget parameter. A minimum computed this
   way need not fit a user's existing Attack/Speed investment. The current
   two-dimensional loop cannot exceed 64 points; adding a third dimension must
   explicitly enforce the 66-point total cap.

6. **Single and combined searches use different feasibility models.** Single
   search uses upstream `ko_chance`; combined search traverses `damage_rolls`
   and its own end-turn state. README explicitly documents their Focus Sash
   discrepancy. The sequence applies end-turn effects after every listed attack,
   including the last; it does not expose same-turn versus next-turn boundaries.
   Its state carries HP, item and toxic counter, not a full battle state. An exact
   search cannot repair an inaccurate or mismatched feasibility predicate.

7. **Validation and uncertainty are underspecified.** Probability/HP floats are
   not checked for finiteness or valid ranges. `None` KO probability is treated
   as zero. Define upstream outcome-specific behavior rather than interpreting
   an absent probability as a generic survival guarantee. Low-level combined
   search also assumes all benchmarks refer to one defender without validating it.

## Exact replacement strategy

Use exhaustive legal HP/Defense/SpD enumeration as the correctness baseline:
at most 33^3 = 35,937 allocations per nature before budget filtering. Preserve
locked other stats. Evaluate the exact feasibility predicate for every candidate.
This guarantees a global minimum within that declared domain and battle model.
If other stats influence damage and may vary, include them in the declared search
domain; a three-stat search alone does not claim unrestricted six-stat optimality.

Enumerating by increasing total cost allows a safe early exit: evaluate the whole
first feasible cost layer, return all its feasible candidates as global minima,
then stop. Stop after one candidate only if one arbitrary optimum is the contract.
For top K feasible results, continue through enough cost layers and resolve ties.

Reduce dimensions only after proving the skipped stat cannot affect this context.
Do not assume category alone proves independence, or assume globally smooth,
convex, or monotone behavior. Integer rounding creates plateaus and jumps;
HP-dependent damage, items, abilities and turn effects require additional care.
No local hill-climb or single-axis search guarantees the mixed optimum.

For sequences, merge probability mass for identical complete future-relevant
states rather than recursively replaying every roll path. This improves scaling
without changing probabilities, provided the state includes every modeled effect.

## Validation needed for implementation

- Reproduce this exact fixture; require the 19-point fixed-nature optimum and
  independently rule out every cheaper HP/SpD allocation.
- Compare production results with a simple exhaustive oracle across physical,
  special and mixed sequences, several natures, budgets and probability thresholds.
- Test defensive-stat overrides, Body Press and Foul Play dependency handling.
- Test preservation of locks, total legality, invalid inputs, and all-minima versus
  top-K output behavior.
- Specify sequence timing and probability semantics before asserting agreement
  between single-hit and one-element sequence queries.

The runnable reproduction now asserts the corrected 19-point optimum against
an independent exhaustive HP/SpD oracle. The integration tests additionally
cover three-dimensional search, multiple constraints, and explicit sequence timing.
