//! Security smells of infrastructure code, read lexically line by line:
//! the catalog of Rahman et al. for scripts of infrastructure, plus
//! images and actions not pinned and the permissions of CI workflows.

use std::collections::HashSet;

use clap::ValueEnum;
use serde::Serialize;

use super::lines::{self, Format, Line, Value};
use super::secrets;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum Smell {
    /// A password, key or token written into the code.
    HardCodedSecret,
    /// A password set to the empty string.
    EmptyPassword,
    /// Running as root or with every privilege.
    AdminByDefault,
    /// `0.0.0.0/0` or `::/0`: every address.
    UnrestrictedAddress,
    /// A workflow that writes everywhere, or runs the code of a pull
    /// request with the secrets of the base.
    CiPermissions,
    /// An endpoint over plain HTTP.
    HttpWithoutTls,
    /// MD5 or SHA-1.
    WeakCryptography,
    /// An image without a tag or on `latest`, an action not on a commit.
    Unpinned,
    /// A comment about a bug, a hack or security.
    SuspiciousComment,
}

pub const ALL: [Smell; 9] = [
    Smell::HardCodedSecret,
    Smell::EmptyPassword,
    Smell::AdminByDefault,
    Smell::UnrestrictedAddress,
    Smell::CiPermissions,
    Smell::HttpWithoutTls,
    Smell::WeakCryptography,
    Smell::Unpinned,
    Smell::SuspiciousComment,
];

impl Smell {
    pub fn title(self) -> &'static str {
        match self {
            Smell::HardCodedSecret => "Hard-coded secret",
            Smell::EmptyPassword => "Empty password",
            Smell::AdminByDefault => "Admin by default",
            Smell::UnrestrictedAddress => "Unrestricted address",
            Smell::CiPermissions => "CI permissions",
            Smell::HttpWithoutTls => "HTTP without TLS",
            Smell::WeakCryptography => "Weak cryptography",
            Smell::Unpinned => "Unpinned",
            Smell::SuspiciousComment => "Suspicious comment",
        }
    }

    /// The severity of CodeClimate: what opens a door is major.
    pub fn severity(self) -> &'static str {
        match self {
            Smell::HardCodedSecret
            | Smell::EmptyPassword
            | Smell::AdminByDefault
            | Smell::UnrestrictedAddress
            | Smell::CiPermissions => "major",
            Smell::HttpWithoutTls | Smell::WeakCryptography | Smell::Unpinned => "minor",
            Smell::SuspiciousComment => "info",
        }
    }
}

/// A smell found on a line.
#[derive(Clone, Debug, PartialEq)]
pub struct Found {
    pub smell: Smell,
    pub line: usize,
    pub detail: String,
}

/// A smell and what tells it, before its line is known.
type Told = (Smell, String);

/// The smells of a file of infrastructure, by line.
pub fn scan(text: &str, format: Format) -> Vec<Found> {
    let lines = lines::split(text, format);
    let pull_request_target =
        format == Format::Yaml && lines.iter().any(|l| l.code.contains("pull_request_target"));
    let mut found: Vec<Found> = lines
        .iter()
        .flat_map(|line| {
            of_line(line, format, pull_request_target)
                .into_iter()
                .map(|(smell, detail)| Found {
                    smell,
                    line: line.number,
                    detail,
                })
        })
        .collect();
    if format == Format::Dockerfile {
        found.extend(dockerfile(&lines));
    }
    dedup(found)
}

/// The smells of one line.
fn of_line(line: &Line<'_>, format: Format, pull_request_target: bool) -> Vec<Told> {
    let code = line.code;
    let mut found: Vec<Told> = lines::assignments(code)
        .iter()
        .flat_map(|(key, value)| assigned(key, value, format))
        .collect();
    let comment: String = line.comment.trim().chars().take(60).collect();
    found.extend(
        [
            secrets::shaped(code)
                .map(|what| (Smell::HardCodedSecret, format!("{what} in the code"))),
            unrestricted(code).map(|cidr| {
                let detail = format!("`{cidr}` allows every address");
                (Smell::UnrestrictedAddress, detail)
            }),
            plain_http(code).map(|url| (Smell::HttpWithoutTls, url)),
            weak_hash(code).map(|hash| (Smell::WeakCryptography, hash.to_string())),
            suspicious(line.comment).then_some((Smell::SuspiciousComment, comment)),
            ci_permissions(code, pull_request_target).map(|why| (Smell::CiPermissions, why.into())),
        ]
        .into_iter()
        .flatten(),
    );
    found
}

/// One smell of each kind per line: a secret by its key and by its shape
/// is one secret.
fn dedup(mut found: Vec<Found>) -> Vec<Found> {
    let mut seen = HashSet::new();
    found.sort_by_key(|f| (f.line, f.smell));
    found.retain(|f| seen.insert((f.line, f.smell)));
    found
}

/// The smells of an assignment.
fn assigned(key: &str, value: &Value<'_>, format: Format) -> Vec<Told> {
    let lower = key.to_ascii_lowercase();
    [
        secret(key, value, format),
        admin(&lower, value, format).then(|| {
            let detail = format!("`{key}: {}`", value.text);
            (Smell::AdminByDefault, detail)
        }),
        pinned(&lower, value.text, format).map(|why| (Smell::Unpinned, why)),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn secret(key: &str, value: &Value<'_>, format: Format) -> Option<Told> {
    if secrets::names_password(key) && value.quoted && value.text.is_empty() {
        Some((Smell::EmptyPassword, format!("`{key}` is empty")))
    } else if secrets::names_secret(key) && secrets::is_literal(value, format) {
        Some((Smell::HardCodedSecret, format!("`{key}` holds a literal")))
    } else {
        None
    }
}

/// Keys of a user that runs as root when set to these.
const ADMIN_USERS: [&str; 4] = ["root", "admin", "administrator", "0"];

/// Whether an assignment runs as root or with every privilege. A
/// Dockerfile runs as its `USER` instruction, whatever `ENV` says.
fn admin(key: &str, value: &Value<'_>, format: Format) -> bool {
    // Becoming root for a task is how Ansible escalates, not a default.
    let user =
        (key == "user" || key.ends_with("_user") || key == "runasuser") && key != "become_user";
    let text = value.text.to_ascii_lowercase();
    let who = text.split(':').next().unwrap_or("");
    let admin = (user && ADMIN_USERS.contains(&who))
        || (key == "privileged" && text == "true")
        || (key == "runasnonroot" && text == "false");
    admin && format != Format::Dockerfile
}

/// Why an image or an action is not pinned. In Terraform an image is as
/// often a disk as a container.
fn pinned(key: &str, value: &str, format: Format) -> Option<String> {
    match key {
        "image" if format != Format::Hcl => unpinned_image(value),
        "uses" => unpinned_action(value),
        _ => None,
    }
}

/// A workflow that writes everywhere, or that reads the head of a pull
/// request under `pull_request_target`.
fn ci_permissions(code: &str, pull_request_target: bool) -> Option<&'static str> {
    if code.contains("write-all") && code.contains("permissions") {
        Some("`permissions: write-all`")
    } else if pull_request_target && checks_out_head(code) {
        Some("pull_request_target runs the head of the pull request")
    } else {
        None
    }
}

/// `0.0.0.0/0` or `::/0` standing alone.
fn unrestricted(code: &str) -> Option<&'static str> {
    ["0.0.0.0/0", "::/0"].into_iter().find(|cidr| {
        code.match_indices(cidr).any(|(at, _)| {
            let before = code[..at].chars().next_back();
            let after = code[at + cidr.len()..].chars().next();
            !before.is_some_and(|c| c.is_ascii_hexdigit() || matches!(c, '.' | ':'))
                && !after.is_some_and(|c| c.is_ascii_digit())
        })
    })
}

/// Hosts that need no TLS: those of the machine or the local network, of
/// examples and documentation, and the names of XML and JSON schemas.
fn needs_tls(host: &str) -> bool {
    let host = host.to_ascii_lowercase();
    let local = [
        "localhost",
        "127.",
        "0.0.0.0",
        "[::1]",
        "10.",
        "192.168.",
        "169.254.",
    ]
    .iter()
    .any(|p| host.starts_with(p))
        || (16..32).any(|n| host.starts_with(&format!("172.{n}.")));
    let reserved = [
        ".local",
        ".localhost",
        ".internal",
        ".test",
        ".example",
        ".invalid",
        ".svc",
    ]
    .iter()
    .any(|s| host.ends_with(s));
    let documentation = ["example.com", "example.org", "example.net"]
        .iter()
        .any(|d| host == *d || host.ends_with(&format!(".{d}")));
    let schemas = [
        "www.w3.org",
        "json-schema.org",
        "schemas.xmlsoap.org",
        "xmlns.com",
    ];
    host.contains('.')
        && !local
        && !reserved
        && !documentation
        && !schemas.contains(&host.as_str())
        && !host.contains(['$', '{', '<'])
}

/// The first `http://` URL of a line whose host needs TLS.
fn plain_http(code: &str) -> Option<String> {
    let lower = code.to_ascii_lowercase();
    lower.match_indices("http://").find_map(|(at, _)| {
        let rest = &code[at + "http://".len()..];
        let end = rest
            .find(|c: char| c.is_whitespace() || matches!(c, '"' | '\'' | '`' | ')' | ']' | ','))
            .unwrap_or(rest.len());
        let url = &rest[..end];
        let host = url.split(['/', '?', '#']).next().unwrap_or("");
        let host = host.rsplit('@').next().unwrap_or(host);
        let host = match host.starts_with('[') {
            true => host,
            false => host.split(':').next().unwrap_or(host),
        };
        needs_tls(host).then(|| format!("http://{url}"))
    })
}

/// MD5 or SHA-1 named in a line of code.
fn weak_hash(code: &str) -> Option<&'static str> {
    lines::words(code).find_map(|w| match w.as_str() {
        "md5" | "md5sum" => Some("md5"),
        "sha1" | "sha1sum" => Some("sha1"),
        _ => None,
    })
}

/// Words of a comment that point at a defect or a risk left in place.
const SUSPICIOUS: [&str; 7] = [
    "todo",
    "fixme",
    "hack",
    "xxx",
    "bug",
    "workaround",
    "insecure",
];

fn suspicious(comment: &str) -> bool {
    lines::words(comment).any(|w| SUSPICIOUS.contains(&w.as_str()))
}

/// The expressions that give the code of the pull request, not of its
/// base.
fn checks_out_head(code: &str) -> bool {
    ["github.event.pull_request.head.", "github.head_ref"]
        .iter()
        .any(|e| code.contains(e))
}

/// Why an image is not pinned: no tag, or `latest`. Images named through
/// a variable or a placeholder, and those pinned by digest, are left alone.
fn unpinned_image(image: &str) -> Option<String> {
    let image = image.trim();
    if image.is_empty() || image.contains(['$', '{', '<', '@']) {
        return None;
    }
    let name = image.rsplit('/').next().unwrap_or(image);
    match name.split_once(':') {
        None => Some(format!("image `{image}` has no tag")),
        Some((_, "latest")) => Some(format!("image `{image}` is on `latest`")),
        Some(_) => None,
    }
}

/// Why an action is not pinned: its reference is not a full commit. Local
/// actions are the repository's own.
fn unpinned_action(uses: &str) -> Option<String> {
    let uses = uses.trim();
    if uses.starts_with("./") || uses.contains("${{") {
        return None;
    }
    if let Some(image) = uses.strip_prefix("docker://") {
        return unpinned_image(image);
    }
    let commit = |r: &str| r.len() == 40 && r.chars().all(|c| c.is_ascii_hexdigit());
    match uses.rsplit_once('@') {
        Some((_, reference)) if commit(reference) => None,
        _ => Some(format!("action `{uses}` is not pinned to a commit")),
    }
}

/// What a Dockerfile adds: images of `FROM` not pinned, and a last stage
/// that ends as root.
fn dockerfile(lines: &[Line<'_>]) -> Vec<Found> {
    let mut found = Vec::new();
    let mut stages: HashSet<String> = HashSet::new();
    let mut user: Option<(usize, &str)> = None;
    for line in lines {
        let mut words = line.code.split_whitespace();
        let instruction = words.next().unwrap_or("").to_ascii_uppercase();
        let args: Vec<&str> = words.filter(|w| !w.starts_with("--")).collect();
        match (instruction.as_str(), args.as_slice()) {
            ("FROM", [image, rest @ ..]) => {
                user = None;
                found.extend(from(image, &stages).map(|detail| Found {
                    smell: Smell::Unpinned,
                    line: line.number,
                    detail,
                }));
                stages.extend(stage_name(rest));
            }
            ("USER", [name, ..]) => user = Some((line.number, name)),
            _ => {}
        }
    }
    found.extend(
        user.filter(|(_, name)| is_root(name))
            .map(|(line, name)| Found {
                smell: Smell::AdminByDefault,
                line,
                detail: format!("`USER {name}` is the last user of the image"),
            }),
    );
    found
}

/// Why the image of a `FROM` is not pinned; an earlier stage and `scratch`
/// are no image.
fn from(image: &str, stages: &HashSet<String>) -> Option<String> {
    let stage = image.to_ascii_lowercase();
    match stage == "scratch" || stages.contains(&stage) {
        true => None,
        false => unpinned_image(image),
    }
}

/// The name `AS` gives a stage.
fn stage_name(rest: &[&str]) -> Option<String> {
    match rest {
        [r#as, name] if r#as.eq_ignore_ascii_case("as") => Some(name.to_ascii_lowercase()),
        _ => None,
    }
}

fn is_root(user: &str) -> bool {
    let who = user.split(':').next().unwrap_or("");
    who.eq_ignore_ascii_case("root") || who == "0"
}

#[cfg(test)]
#[path = "smells_test.rs"]
mod tests;
