//! Long help of `km infra smells`.

/// The smells and how they are read.
pub const SMELLS: &str = "\
Find security smells in infrastructure code: Terraform, Dockerfiles, CI
pipelines, Compose files, Kubernetes manifests and Ansible playbooks (the
files of kind infra, whatever --kind says).

Smells:
  hard-coded-secret     a literal assigned to a key named as a secret
                        (password, token, secret, api_key...), or a value
                        shaped like a known key (AWS, GitHub, GitLab, Slack,
                        Stripe, Google, private keys). Variables, templates
                        and references are not literals
  empty-password        a password set to \"\"
  admin-by-default      user: root, runAsUser: 0, privileged: true,
                        runAsNonRoot: false, USER root left as the last
                        user of a Dockerfile
  unrestricted-address  0.0.0.0/0 or ::/0
  ci-permissions        permissions: write-all, a pull_request_target
                        workflow that reads the head of the pull request
  http-without-tls      an http:// URL to a host outside the machine, the
                        local network and example domains
  weak-cryptography     md5 or sha1 in the code
  unpinned              an image without a tag or on latest, an action
                        not pinned to a full commit SHA
  suspicious-comment    a comment with TODO, FIXME, HACK, XXX, bug,
                        workaround or insecure

The reading is lexical, line by line: comments are told apart by format
(#, // and /* */ in HCL, whole lines in a Dockerfile), and only the code is
searched for every smell but the last. Secrets are never printed.

After the catalog of Rahman et al., \"The Seven Sins: Security Smells in
Infrastructure as Code Scripts\" (ICSE 2019).

--since-ref REF keeps the files changed since REF. --fail-on takes the smells
that fail the run, comma-separated.

Examples:
  km infra smells
  km infra smells --since-ref origin/main --fail-on hard-coded-secret,empty-password
  km infra smells --format github";
