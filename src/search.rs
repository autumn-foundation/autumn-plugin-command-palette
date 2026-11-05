//! Find and rank the commands for a query.
//!
//! `take_ranked` and `parse_recent` have the loops of the Verus model in
//! `verification/palette.rs`. The model proves the bounds, the order of
//! `take_ranked` and that each item shows one time. The property tests
//! check the same and also that no match is lost.

use crate::command::{Command, CommandRegistry};

/// The longest query, in characters. The palette ignores the rest.
pub const MAX_QUERY_CHARS: usize = 200;

/// The most recent ids that the palette reads from one request.
pub const MAX_RECENT_IDS: usize = 20;

/// The score of a command that does not match.
pub const NO_MATCH: u8 = u8::MAX;

/// The last match rank.
pub const WORST: u8 = 3;

/// One group of results.
#[derive(Debug, Clone)]
pub struct ResultGroup<'a> {
    /// The group label.
    pub label: String,
    /// `true` for the recent items group.
    pub recent: bool,
    /// The commands, best first.
    pub commands: Vec<&'a Command>,
}

/// The results for one query.
#[derive(Debug, Clone, Default)]
pub struct SearchResults<'a> {
    /// The query, trimmed and cut to [`MAX_QUERY_CHARS`].
    pub query: String,
    /// The groups, in order.
    pub groups: Vec<ResultGroup<'a>>,
}

impl SearchResults<'_> {
    /// The number of commands in all groups.
    #[must_use]
    pub fn len(&self) -> usize {
        self.groups.iter().map(|group| group.commands.len()).sum()
    }

    /// `true` when no command matches.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The commands of all groups, in order.
    pub fn commands(&self) -> impl Iterator<Item = &Command> {
        self.groups
            .iter()
            .flat_map(|group| group.commands.iter().copied())
    }
}

/// The limits of one search.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// The most results, recent items not included.
    pub results: usize,
    /// The most recent items. They show only for an empty query.
    pub recent: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            results: 20,
            recent: 5,
        }
    }
}

/// Trim `raw`, cut it to [`MAX_QUERY_CHARS`] characters.
#[must_use]
pub fn normalize_query(raw: &str) -> String {
    raw.trim()
        .chars()
        .take(MAX_QUERY_CHARS)
        .collect::<String>()
        .trim_end()
        .to_owned()
}

/// The rank of `command` for `query`, from 0 (best) to [`WORST`], or
/// [`NO_MATCH`]. `query` is lowercase and not empty.
///
/// | Rank | The command matches when |
/// |---|---|
/// | 0 | the title starts with the query |
/// | 1 | a word of the title starts with the query |
/// | 2 | the title holds the query |
/// | 3 | each query word starts a word of the title, the keywords, the group or the hint |
///
/// The text is lowercase, without combining marks (see [`fold`]).
#[must_use]
pub fn score(command: &Command, query: &str) -> u8 {
    let query = fold(query);
    let query = query.as_str();
    let title = fold(command.title());
    if title.starts_with(query) {
        return 0;
    }
    if title
        .split(|c: char| !c.is_alphanumeric())
        .any(|word| word.starts_with(query))
    {
        return 1;
    }
    if title.contains(query) {
        return 2;
    }
    let mut text = title;
    for extra in command
        .keyword_list()
        .iter()
        .map(String::as_str)
        .chain([command.group_name()])
        .chain(command.hint_text())
    {
        text.push(' ');
        text.push_str(&fold(extra));
    }
    let words: Vec<&str> = text.split(|c: char| !c.is_alphanumeric()).collect();
    if query
        .split_whitespace()
        .all(|q| words.iter().any(|word| word.starts_with(q)))
    {
        WORST
    } else {
        NO_MATCH
    }
}

/// Lowercase `text` and remove the combining marks U+0300…U+036F. Thus
/// `İst` and `ist` match `İstanbul`.
#[must_use]
pub fn fold(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .filter(|c| !('\u{300}'..='\u{36f}').contains(c))
        .collect()
}

/// The indices of the matched scores: best rank first, registry order in
/// a rank, `limit` or fewer, each index one time.
#[must_use]
pub fn take_ranked(scores: &[u8], limit: usize) -> Vec<usize> {
    // One pass for each rank. The model proves this loop in
    // `verification/palette.rs`.
    let mut out = Vec::new();
    for rank in 0..=WORST {
        for (i, &score) in scores.iter().enumerate() {
            if score == rank && out.len() < limit {
                out.push(i);
            }
        }
    }
    out
}

/// The ids in a comma-separated `recent` value: trimmed, not empty, first
/// copy only, [`MAX_RECENT_IDS`] or fewer.
#[must_use]
pub fn parse_recent(raw: &str) -> Vec<&str> {
    let mut out: Vec<&str> = Vec::new();
    for id in raw.split(',').map(str::trim).filter(|id| !id.is_empty()) {
        if out.len() < MAX_RECENT_IDS && !out.contains(&id) {
            out.push(id);
        }
    }
    out
}

/// Find the commands for `query`.
///
/// An empty query gives the recent items first (in `recent` order), then
/// the other commands in registry order, collected by group. A query gives the matched
/// commands. A group shows at the place of its best command.
/// `visible` removes a command; it removes a recent item too.
#[must_use]
pub fn search<'a>(
    registry: &'a CommandRegistry,
    query: &str,
    recent: &[&str],
    visible: &dyn Fn(&Command) -> bool,
    limits: Limits,
    recent_label: &str,
) -> SearchResults<'a> {
    let query = normalize_query(query);
    let commands = registry.commands();
    let mut groups: Vec<ResultGroup<'a>> = Vec::new();
    if query.is_empty() {
        let mut shown: Vec<&'a Command> = Vec::new();
        for command in recent
            .iter()
            .take(MAX_RECENT_IDS)
            .filter_map(|id| registry.get(id))
        {
            if shown.len() < limits.recent
                && visible(command)
                && !shown.iter().any(|c| c.id() == command.id())
            {
                shown.push(command);
            }
        }
        let recent = shown;
        let rest: Vec<&'a Command> = commands
            .iter()
            .filter(|command| visible(command) && !recent.iter().any(|r| r.id() == command.id()))
            .take(limits.results)
            .collect();
        if !recent.is_empty() {
            groups.push(ResultGroup {
                label: recent_label.to_owned(),
                recent: true,
                commands: recent,
            });
        }
        push_grouped(&mut groups, rest.into_iter());
    } else {
        let lower = fold(&query);
        let scores: Vec<u8> = commands
            .iter()
            .map(|command| {
                if visible(command) {
                    score(command, &lower)
                } else {
                    NO_MATCH
                }
            })
            .collect();
        let ranked = take_ranked(&scores, limits.results);
        push_grouped(&mut groups, ranked.into_iter().map(|i| &commands[i]));
    }
    SearchResults { query, groups }
}

/// Add each command to the group with its name. A new group goes at the
/// end. The recent group is not a target.
fn push_grouped<'a>(
    groups: &mut Vec<ResultGroup<'a>>,
    commands: impl Iterator<Item = &'a Command>,
) {
    for command in commands {
        let name = command.group_name();
        if let Some(group) = groups.iter_mut().find(|g| !g.recent && g.label == name) {
            group.commands.push(command);
        } else {
            groups.push(ResultGroup {
                label: name.to_owned(),
                recent: false,
                commands: vec![command],
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn registry() -> CommandRegistry {
        CommandRegistry::new()
            .add(Command::route("home", "Dashboard", "/"))
            .add(Command::route("posts", "Posts", "/posts").keywords(["articles"]))
            .add(Command::route("settings", "Account settings", "/settings").group("Account"))
            .add(Command::action("posts.new", "New post", "/posts").hint("Write a draft"))
            .add(Command::action("out", "Sign out", "/logout").group("Account"))
    }

    fn all(_: &Command) -> bool {
        true
    }

    fn ids<'a>(results: &'a SearchResults<'_>) -> Vec<&'a str> {
        results.commands().map(Command::id).collect()
    }

    fn labels<'a>(results: &'a SearchResults<'_>) -> Vec<&'a str> {
        results.groups.iter().map(|g| g.label.as_str()).collect()
    }

    fn find(query: &str, recent: &[&str]) -> Vec<String> {
        let registry = registry();
        let results = search(&registry, query, recent, &all, Limits::default(), "Recent");
        ids(&results).into_iter().map(str::to_owned).collect()
    }

    #[test]
    fn normalize_query_trims_and_cuts() {
        assert_eq!(normalize_query("  New Post \n"), "New Post");
        assert_eq!(
            normalize_query(&"é".repeat(300)).chars().count(),
            MAX_QUERY_CHARS
        );
    }

    #[test]
    fn score_ranks_each_kind_of_match() {
        let posts = Command::route("p", "Posts", "/p").keywords(["articles"]);
        let settings = Command::route("s", "Account settings", "/s").hint("Your profile");
        assert_eq!(score(&posts, "pos"), 0);
        assert_eq!(score(&settings, "set"), 1);
        assert_eq!(score(&settings, "count"), 2);
        assert_eq!(score(&posts, "articles"), 3);
        assert_eq!(score(&settings, "profile account"), 3);
        assert_eq!(score(&posts, "go to"), 3, "the group matches");
        assert_eq!(score(&posts, "zebra"), NO_MATCH);
        assert_eq!(score(&settings, "profile zebra"), NO_MATCH);
    }

    #[test]
    fn rank_3_needs_a_word_start() {
        let posts = Command::route("p", "Posts", "/p").keywords(["articles"]);
        assert_eq!(score(&posts, "rticles"), NO_MATCH);
        assert_eq!(score(&posts, "o"), 2, "the title holds `o`");
        let dash = Command::route("d", "Dashboard", "/d");
        assert_eq!(score(&dash, "t"), 3, "`to` in `Go to` starts with `t`");
        assert_eq!(
            score(&dash, "e"),
            NO_MATCH,
            "no `e` and no word starts with `e`"
        );
    }

    #[test]
    fn combining_marks_do_not_block_a_match() {
        let city = Command::route("c", "İstanbul", "/c");
        for query in ["ist", "İst", "istanbul"] {
            assert_eq!(score(&city, query), 0, "{query}");
        }
        assert_eq!(fold("İ"), "i");
    }

    #[test]
    fn a_repeated_recent_id_shows_one_time() {
        let registry = registry();
        let results = search(
            &registry,
            "",
            &["out", "out", "home"],
            &all,
            Limits::default(),
            "Recent",
        );
        assert_eq!(&ids(&results)[..2], ["out", "home"]);
    }

    #[test]
    fn score_ignores_case() {
        let posts = Command::route("p", "Posts", "/p");
        assert_eq!(score(&posts, "posts"), 0);
        let upper = Command::route("q", "ÜBER", "/q");
        assert_eq!(score(&upper, "über"), 0);
    }

    #[test]
    fn take_ranked_orders_by_rank_then_index() {
        assert_eq!(take_ranked(&[2, NO_MATCH, 0, 1, 0], 10), [2, 4, 3, 0]);
        assert_eq!(take_ranked(&[2, NO_MATCH, 0, 1, 0], 2), [2, 4]);
        assert_eq!(take_ranked(&[0, 1], 0), Vec::<usize>::new());
        assert_eq!(take_ranked(&[], 5), Vec::<usize>::new());
    }

    #[test]
    fn parse_recent_dedupes_and_caps() {
        assert_eq!(parse_recent(" a, b,,a , c"), ["a", "b", "c"]);
        assert_eq!(parse_recent(""), Vec::<&str>::new());
        let many: Vec<String> = (0..50).map(|i| format!("id{i}")).collect();
        assert_eq!(parse_recent(&many.join(",")).len(), MAX_RECENT_IDS);
    }

    #[test]
    fn an_empty_query_shows_recent_then_all_in_groups() {
        let registry = registry();
        let results = search(
            &registry,
            "  ",
            &["out", "gone", "home"],
            &all,
            Limits::default(),
            "Recent",
        );
        assert_eq!(results.query, "");
        assert_eq!(labels(&results), ["Recent", "Go to", "Account", "Actions"]);
        assert!(results.groups[0].recent);
        assert!(!results.groups[1].recent);
        assert_eq!(
            ids(&results),
            ["out", "home", "posts", "settings", "posts.new"]
        );
        assert_eq!(results.len(), 5);
    }

    #[test]
    fn a_query_ranks_and_has_no_recent_group() {
        assert_eq!(find("post", &["out"]), ["posts", "posts.new"]);
        assert_eq!(find("new", &[]), ["posts.new"]);
        assert_eq!(find("zebra", &[]), Vec::<String>::new());
    }

    #[test]
    fn a_group_shows_at_its_best_command() {
        let registry = registry();
        let results = search(&registry, "s", &[], &all, Limits::default(), "Recent");
        // `Sign out` (rank 0) puts Account first, before `Dashboard` (rank 2).
        assert_eq!(labels(&results), ["Account", "Go to", "Actions"]);
        assert_eq!(
            ids(&results),
            ["out", "settings", "home", "posts", "posts.new"]
        );
        let results = search(&registry, "sign", &[], &all, Limits::default(), "Recent");
        assert_eq!(labels(&results), ["Account"]);
    }

    #[test]
    fn limits_cut_results_and_recent() {
        let registry = registry();
        let limits = Limits {
            results: 2,
            recent: 1,
        };
        let results = search(&registry, "", &["out", "home"], &all, limits, "Recent");
        assert_eq!(ids(&results), ["out", "home", "posts"]);
    }

    #[test]
    fn visible_filters_results_and_recent() {
        let registry = registry();
        let hide_out = |command: &Command| command.id() != "out";
        let results = search(
            &registry,
            "",
            &["out"],
            &hide_out,
            Limits::default(),
            "Recent",
        );
        assert!(!ids(&results).contains(&"out"));
        assert!(results.groups.iter().all(|g| !g.recent));
        let results = search(
            &registry,
            "sign",
            &[],
            &hide_out,
            Limits::default(),
            "Recent",
        );
        assert!(results.is_empty());
    }

    fn scores() -> impl Strategy<Value = Vec<u8>> {
        proptest::collection::vec(prop_oneof![0..=WORST, Just(NO_MATCH)], 0..40)
    }

    proptest! {
        /// The postconditions of `take_ranked` in `verification/palette.rs`,
        /// plus the order.
        #[test]
        fn take_ranked_meets_the_model(scores in scores(), limit in 0usize..50) {
            let out = take_ranked(&scores, limit);
            prop_assert!(out.len() <= limit);
            let mut seen = std::collections::HashSet::new();
            for &i in &out {
                prop_assert!(i < scores.len());
                prop_assert!(scores[i] <= WORST);
                prop_assert!(seen.insert(i), "index {} two times", i);
            }
            for pair in out.windows(2) {
                prop_assert!((scores[pair[0]], pair[0]) < (scores[pair[1]], pair[1]));
            }
            let matched = scores.iter().filter(|&&s| s <= WORST).count();
            prop_assert_eq!(out.len(), matched.min(limit));
        }

        /// The postconditions of `dedupe_capped` in `verification/palette.rs`.
        #[test]
        fn parse_recent_meets_the_model(ids in proptest::collection::vec("[a-c]{1,2}", 0..40)) {
            let raw = ids.join(",");
            let out = parse_recent(&raw);
            prop_assert!(out.len() <= MAX_RECENT_IDS);
            let unique: std::collections::HashSet<_> = out.iter().collect();
            prop_assert_eq!(unique.len(), out.len());
            for id in &out {
                prop_assert!(ids.iter().any(|x| x == id));
            }
        }

        /// Each command shows at most one time, and the limits hold.
        #[test]
        fn search_has_no_duplicates(query in "[a-z ]{0,6}", recent in proptest::collection::vec("[a-z.]{1,9}", 0..6)) {
            let registry = registry();
            let recent: Vec<&str> = recent.iter().map(String::as_str).collect();
            let limits = Limits { results: 3, recent: 2 };
            let results = search(&registry, &query, &recent, &all, limits, "Recent");
            let found = ids(&results);
            let unique: std::collections::HashSet<_> = found.iter().collect();
            prop_assert_eq!(unique.len(), found.len());
            prop_assert!(found.len() <= limits.results + limits.recent);
        }
    }
}
