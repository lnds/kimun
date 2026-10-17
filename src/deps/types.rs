//! Types by their qualified name, for languages where a file uses the
//! types it names rather than the files it imports: Java and C#.
//!
//! A file sees some namespaces whole (its own package, an import on
//! demand, a `using`) and some types by a name of its own (a single-type
//! import, an alias). A name it writes, or a qualified one such as
//! `models.User`, names a type when it is one of those it sees, or when
//! its qualifier is a namespace, whole or from one the file is in.

use std::collections::HashMap;

/// The files that declare each type, by qualified name. A C# partial
/// class is declared by several.
#[derive(Default)]
pub struct Types(HashMap<String, Vec<usize>>);

/// `namespace.name`, or `name` in the namespace without a name.
pub fn qualified(namespace: &str, name: &str) -> String {
    if namespace.is_empty() {
        name.to_string()
    } else {
        format!("{namespace}.{name}")
    }
}

/// What a file sees.
#[derive(Debug, Default)]
pub struct Scope {
    /// Namespaces whose types the file names by their simple name.
    pub whole: Vec<String>,
    /// Names the file gives to a type or a namespace, with what they stand
    /// for, qualified.
    pub aliases: HashMap<String, String>,
    /// Namespaces a qualified name may be written from: those the file is
    /// in (C#).
    pub enclosing: Vec<String>,
}

impl Types {
    pub fn declare(&mut self, name: String, file: usize) {
        self.0.entry(name).or_default().push(file);
    }

    /// The files that declare the type `name`, qualified.
    pub fn get(&self, name: &str) -> &[usize] {
        self.0.get(name).map_or(&[], Vec::as_slice)
    }

    /// The files that declare the type `name` stands for, qualified: the
    /// type itself, or the one that holds it when it is nested or a member
    /// (`a.Outer.Inner`, `a.Util.max`).
    pub fn holding(&self, name: &str) -> &[usize] {
        let mut name = name;
        loop {
            let files = self.get(name);
            if !files.is_empty() {
                return files;
            }
            match name.rsplit_once('.') {
                Some((outer, _)) => name = outer,
                None => return &[],
            }
        }
    }

    /// The files that declare the type a chain of names (`a.b.C.d`) starts
    /// with, as `scope` sees it.
    pub fn resolve(&self, chain: &[&str], scope: &Scope) -> &[usize] {
        let Some((first, rest)) = chain.split_first() else {
            return &[];
        };
        if let Some(target) = scope.aliases.get(*first) {
            let files = self.get(target);
            if !files.is_empty() {
                return files;
            }
            let mut expanded: Vec<&str> = target.split('.').collect();
            expanded.extend_from_slice(rest);
            return self.qualified_in(&expanded, &[String::new()]);
        }
        let seen = scope.whole.iter().map(|ns| self.get(&qualified(ns, first)));
        if let Some(files) = seen.into_iter().find(|f| !f.is_empty()) {
            return files;
        }
        let mut from = vec![String::new()];
        from.extend(scope.enclosing.iter().cloned());
        self.qualified_in(chain, &from)
    }

    /// The type a qualified chain names, its qualifier taken from each of
    /// `from` in turn: `a.b.C` is the type `C` of the namespace `a.b`.
    fn qualified_in(&self, chain: &[&str], from: &[String]) -> &[usize] {
        for split in 1..chain.len() {
            let qualifier = chain[..split].join(".");
            for base in from {
                let files = self.get(&qualified(&qualified(base, &qualifier), chain[split]));
                if !files.is_empty() {
                    return files;
                }
            }
        }
        &[]
    }
}

#[cfg(test)]
#[path = "types_test.rs"]
mod tests;
