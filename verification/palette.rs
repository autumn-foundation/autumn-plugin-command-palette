// Verus model of the palette result selection (autumn#3416).
//
// `take_ranked` mirrors `search::take_ranked` in `src/search.rs`.
// `dedupe_capped` mirrors `search::parse_recent` in `src/search.rs`.
// A score is the match rank of one command: 0 is best, `WORST` is the last
// rank, and `NO_MATCH` is no match. A `u64` replaces the command id
// string. The model uses only equality.
//
// The model proves the bounds, the rank order of `take_ranked`, and that
// each item shows one time. The property tests in `src/search.rs` also
// check that no match is lost.
//
// Verify: `verus verification/palette.rs`.

use vstd::prelude::*;

verus! {

/// The score of a command that does not match.
pub const NO_MATCH: u8 = 255;

/// The last match rank.
pub const WORST: u8 = 3;

/// No two items of `s` are equal.
pub open spec fn distinct<T>(s: Seq<T>) -> bool {
    forall|j: int, k: int| 0 <= j < k < s.len() ==> s[j] != s[k]
}

/// Index `a` comes before index `b`: a better rank, or the same rank and
/// an earlier index.
pub open spec fn ahead(scores: Seq<u8>, a: usize, b: usize) -> bool {
    scores[a as int] < scores[b as int] || (scores[a as int] == scores[b as int] && a < b)
}

/// The indices in `out` are in rank order.
pub open spec fn ranked(scores: Seq<u8>, out: Seq<usize>) -> bool {
    forall|j: int, k: int| 0 <= j < k < out.len() ==> #[trigger] ahead(scores, out[j], out[k])
}

/// Take the indices of the matched commands, best rank first, in
/// registry order inside a rank. Stop at `limit`.
pub fn take_ranked(scores: &Vec<u8>, limit: usize) -> (out: Vec<usize>)
    ensures
        out.len() <= limit,
        distinct(out@),
        ranked(scores@, out@),
        forall|j: int|
            0 <= j < out.len() ==> out@[j] < scores.len() && scores@[out@[j] as int] <= WORST,
{
    let mut out: Vec<usize> = Vec::new();
    let mut rank: u8 = 0;
    while rank <= WORST
        invariant
            rank <= WORST + 1,
            out.len() <= limit,
            distinct(out@),
            ranked(scores@, out@),
            forall|j: int|
                0 <= j < out.len() ==> out@[j] < scores.len() && scores@[out@[j] as int] < rank,
        decreases WORST + 1 - rank,
    {
        let mut i: usize = 0;
        while i < scores.len()
            invariant
                rank <= WORST,
                i <= scores.len(),
                out.len() <= limit,
                distinct(out@),
                ranked(scores@, out@),
                forall|j: int|
                    0 <= j < out.len() ==> out@[j] < scores.len() && scores@[out@[j] as int] <= rank
                        && (scores@[out@[j] as int] == rank ==> out@[j] < i),
            decreases scores.len() - i,
        {
            if scores[i] == rank && out.len() < limit {
                proof {
                    assert forall|j: int| 0 <= j < out.len() implies out@[j] != i by {
                        if out@[j] == i {
                            assert(scores@[i as int] == rank);
                        }
                    }
                }
                let ghost prev = out@;
                out.push(i);
                proof {
                    assert(out@ == prev.push(i));
                    assert forall|j: int, k: int| 0 <= j < k < out@.len() implies out@[j]
                        != out@[k] by {
                        if k < prev.len() {
                            assert(prev[j] != prev[k]);
                        } else {
                            assert(out@[k] == i);
                            assert(out@[j] == prev[j]);
                        }
                    }
                    assert forall|j: int, k: int| 0 <= j < k < out@.len() implies #[trigger] ahead(
                        scores@,
                        out@[j],
                        out@[k],
                    ) by {
                        if k < prev.len() {
                            assert(ahead(scores@, prev[j], prev[k]));
                        } else {
                            assert(out@[k] == i);
                            assert(out@[j] == prev[j]);
                            assert(scores@[prev[j] as int] <= rank);
                            assert(scores@[i as int] == rank);
                        }
                    }
                }
            }
            i += 1;
        }
        rank += 1;
    }
    out
}

/// `true` when `v` holds `x`.
pub fn contains(v: &Vec<u64>, x: u64) -> (found: bool)
    ensures
        found == v@.contains(x),
{
    let mut i: usize = 0;
    while i < v.len()
        invariant
            i <= v.len(),
            forall|j: int| 0 <= j < i ==> v@[j] != x,
        decreases v.len() - i,
    {
        if v[i] == x {
            assert(v@[i as int] == x);
            return true;
        }
        i += 1;
    }
    proof {
        assert forall|j: int| 0 <= j < v.len() implies v@[j] != x by {}
        if v@.contains(x) {
            let j = choose|j: int| 0 <= j < v.len() && v@[j] == x;
            assert(v@[j] != x);
        }
    }
    false
}

/// Keep the first copy of each id, in order. Stop at `max` ids.
pub fn dedupe_capped(items: &Vec<u64>, max: usize) -> (out: Vec<u64>)
    ensures
        out.len() <= max,
        distinct(out@),
        forall|j: int| 0 <= j < out.len() ==> items@.contains(#[trigger] out@[j]),
{
    let mut out: Vec<u64> = Vec::new();
    let mut i: usize = 0;
    while i < items.len()
        invariant
            i <= items.len(),
            out.len() <= max,
            distinct(out@),
            forall|j: int| 0 <= j < out.len() ==> items@.contains(#[trigger] out@[j]),
        decreases items.len() - i,
    {
        let item = items[i];
        if out.len() < max && !contains(&out, item) {
            let ghost prev = out@;
            out.push(item);
            proof {
                assert(out@ == prev.push(item));
                assert(items@[i as int] == item);
                assert forall|j: int| 0 <= j < out@.len() implies items@.contains(#[trigger] out@[j]) by {
                    if j < prev.len() {
                        assert(out@[j] == prev[j]);
                    } else {
                        assert(out@[j] == item);
                    }
                }
                assert forall|j: int, k: int| 0 <= j < k < out@.len() implies out@[j]
                    != out@[k] by {
                    if k < prev.len() {
                        assert(prev[j] != prev[k]);
                    } else {
                        assert(out@[k] == item);
                        assert(out@[j] == prev[j]);
                        assert(prev.contains(prev[j]));
                    }
                }
            }
        }
        i += 1;
    }
    out
}

fn main() {
}

} // verus!
