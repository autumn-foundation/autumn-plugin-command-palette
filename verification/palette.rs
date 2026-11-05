// Verus model of the palette result selection (autumn#3416).
//
// `take_ranked` mirrors `search::take_ranked` in `src/search.rs`.
// `dedupe_capped` mirrors `search::parse_recent` in `src/search.rs`.
// A score is the match rank of one command: 0 is best, `WORST` is the last
// rank, and `NO_MATCH` is no match. A `u64` stands in for a command id
// string: the model needs equality only.
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

/// Take the indices of the matched commands, best rank first, in
/// registry order inside a rank. Stop at `limit`.
pub fn take_ranked(scores: &Vec<u8>, limit: usize) -> (out: Vec<usize>)
    ensures
        out.len() <= limit,
        distinct(out@),
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
                let ghost before = out@;
                out.push(i);
                proof {
                    assert(out@ == before.push(i));
                    assert forall|j: int, k: int| 0 <= j < k < out@.len() implies out@[j]
                        != out@[k] by {
                        if k < before.len() {
                            assert(before[j] != before[k]);
                        } else {
                            assert(out@[k] == i);
                            assert(out@[j] == before[j]);
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
        forall|j: int| 0 <= j < out.len() ==> items@.contains(out@[j]),
{
    let mut out: Vec<u64> = Vec::new();
    let mut i: usize = 0;
    while i < items.len()
        invariant
            i <= items.len(),
            out.len() <= max,
            distinct(out@),
            forall|j: int| 0 <= j < out.len() ==> items@.contains(out@[j]),
        decreases items.len() - i,
    {
        let item = items[i];
        if out.len() < max && !contains(&out, item) {
            let ghost before = out@;
            out.push(item);
            proof {
                assert(out@ == before.push(item));
                assert(items@[i as int] == item);
                assert forall|j: int| 0 <= j < out@.len() implies items@.contains(out@[j]) by {
                    if j < before.len() {
                        assert(out@[j] == before[j]);
                    } else {
                        assert(out@[j] == item);
                    }
                }
                assert forall|j: int, k: int| 0 <= j < k < out@.len() implies out@[j]
                    != out@[k] by {
                    if k < before.len() {
                        assert(before[j] != before[k]);
                    } else {
                        assert(out@[k] == item);
                        assert(out@[j] == before[j]);
                        assert(before.contains(before[j]));
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
