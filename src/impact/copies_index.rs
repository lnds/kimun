//! The edits of a change by the values they held before it, so that the
//! search for copies compares a group only with the edits it may copy.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use super::copies::{Edit, literals};

/// The edits by the values they held before the change, so that a group is
/// compared only with the edits it holds enough values of, and a file that
/// holds too few for any of them is not read for groups at all.
pub(super) struct Index<'e> {
    edits: &'e [Edit],
    /// The edits, by position, whose `before` holds each value.
    holders: HashMap<&'e str, Vec<usize>>,
}

impl<'e> Index<'e> {
    pub(super) fn new(edits: &'e [Edit]) -> Self {
        let mut holders: HashMap<&str, Vec<usize>> = HashMap::new();
        for (at, edit) in edits.iter().enumerate() {
            for value in &edit.before {
                holders.entry(value).or_default().push(at);
            }
        }
        Self { edits, holders }
    }

    /// Whether `text` holds enough values of some edit for one of its
    /// groups to be a copy: a group holds no more than its file does.
    pub(super) fn may_hold_copy(&self, text: &str) -> bool {
        let mut seen: HashSet<&str> = HashSet::new();
        let mut shared: HashMap<usize, usize> = HashMap::new();
        text.lines().flat_map(literals).any(|value| {
            let Some((&value, holders)) = self.holders.get_key_value(value.as_str()) else {
                return false;
            };
            seen.insert(value)
                && holders
                    .iter()
                    .any(|&at| self.shares_enough(&mut shared, at))
        })
    }

    /// Count one more value shared with the edit at `at`, and tell whether
    /// that makes enough of them.
    fn shares_enough(&self, shared: &mut HashMap<usize, usize>, at: usize) -> bool {
        let count = shared.entry(at).or_default();
        *count += 1;
        *count >= self.edits[at].needed()
    }

    /// The edits `values` hold enough values of, in the order of the edits.
    pub(super) fn candidates(&self, values: &BTreeSet<String>) -> Vec<&'e Edit> {
        let mut shared: BTreeMap<usize, usize> = BTreeMap::new();
        let holders = values.iter().filter_map(|v| self.holders.get(v.as_str()));
        for &at in holders.flatten() {
            *shared.entry(at).or_default() += 1;
        }
        shared
            .into_iter()
            .filter(|&(at, count)| count >= self.edits[at].needed())
            .map(|(at, _)| &self.edits[at])
            .collect()
    }
}
