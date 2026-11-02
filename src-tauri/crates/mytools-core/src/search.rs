//! Fuzzy match scoring over the indexed plugin/app catalog.
//!
//! Day 1: minimal Skim-based scorer. Real weights and pinyin support
//! land in Day 3 (see docs/02-architecture.md §3.2).

use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use serde::Serialize;

/// A single fuzzy match hit with its score. Higher is better.
#[derive(Debug, Clone, Serialize)]
pub struct FuzzyHit {
    pub id: String,
    pub title: String,
    pub score: i64,
}

/// Score `query` against each candidate's searchable text.
///
/// Empty query returns an empty vec — callers should display a default
/// "recent / pinned" list instead.
pub fn fuzzy_match<S: AsRef<str>>(query: &str, candidates: &[(S, S)]) -> Vec<FuzzyHit> {
    if query.is_empty() {
        return Vec::new();
    }
    let matcher = SkimMatcherV2::default();
    let mut hits: Vec<FuzzyHit> = candidates
        .iter()
        .filter_map(|(id, title)| {
            matcher
                .fuzzy_match(title.as_ref(), query)
                .map(|score| FuzzyHit {
                    id: id.as_ref().to_string(),
                    title: title.as_ref().to_string(),
                    score,
                })
        })
        .collect();
    hits.sort_by_key(|hit| std::cmp::Reverse(hit.score));
    hits
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_query_returns_empty() {
        let hits = fuzzy_match("", &[("calc", "Calculator")]);
        assert!(hits.is_empty());
    }

    #[test]
    fn exact_substring_ranks_highest() {
        let hits = fuzzy_match(
            "json",
            &[("a", "Random"), ("b", "JSON Formatter"), ("c", "Base64")],
        );
        assert!(!hits.is_empty());
        assert_eq!(hits[0].id, "b");
    }

    #[test]
    fn ranks_better_matches_first() {
        let hits = fuzzy_match(
            "calc",
            &[("a", "Calendar"), ("b", "Calculator"), ("c", "Cam")],
        );
        assert_eq!(hits[0].id, "b");
    }

    #[test]
    fn no_match_returns_empty() {
        let hits = fuzzy_match("zzzzzz", &[("a", "Calculator")]);
        assert!(hits.is_empty());
    }
}
