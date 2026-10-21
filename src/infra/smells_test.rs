use super::*;

fn found(text: &str, format: Format) -> Vec<(Smell, usize)> {
    scan(text, format)
        .into_iter()
        .map(|f| (f.smell, f.line))
        .collect()
}

#[test]
fn terraform_security_group_and_database() {
    let text = r#"resource "aws_security_group_rule" "ssh" {
  cidr_blocks = ["0.0.0.0/0"]
}
resource "aws_db_instance" "db" {
  password = "hunter2"
  username = var.user
  other_password = var.db_password
  empty_password = ""
  # TODO: rotate this
}
"#;
    assert_eq!(
        found(text, Format::Hcl),
        [
            (Smell::UnrestrictedAddress, 2),
            (Smell::HardCodedSecret, 5),
            (Smell::EmptyPassword, 8),
            (Smell::SuspiciousComment, 9),
        ]
    );
}

#[test]
fn dockerfile_images_and_last_user() {
    let text = "FROM node AS build\n\
                RUN echo hi | md5sum\n\
                FROM build\n\
                FROM nginx:latest\n\
                FROM --platform=linux/amd64 alpine:3.20\n\
                ADD http://downloads.vendor.io/tool.tgz /tmp/\n\
                USER root\n";
    assert_eq!(
        found(text, Format::Dockerfile),
        [
            (Smell::Unpinned, 1),
            (Smell::WeakCryptography, 2),
            (Smell::Unpinned, 4),
            (Smell::HttpWithoutTls, 6),
            (Smell::AdminByDefault, 7),
        ]
    );
    let dropped = "FROM alpine:3.20\nUSER root\nENV USER=root\nUSER app\n";
    assert!(found(dropped, Format::Dockerfile).is_empty());
}

#[test]
fn github_workflow() {
    let text = "on: pull_request_target\n\
                permissions: write-all\n\
                jobs:\n  build:\n    steps:\n\
                      - uses: actions/checkout@v4\n\
                        with:\n\
                          ref: ${{ github.event.pull_request.head.sha }}\n\
                          token: ${{ secrets.GITHUB_TOKEN }}\n\
                      - uses: actions/setup-node@1d0ff469b7ec7b3cb9d8673fde0c81c44821de2a\n\
                      - uses: ./local/action\n";
    assert_eq!(
        found(text, Format::Yaml),
        [
            (Smell::CiPermissions, 2),
            (Smell::Unpinned, 6),
            (Smell::CiPermissions, 8),
        ]
    );
}

#[test]
fn kubernetes_and_compose() {
    let text = "spec:\n  containers:\n    - image: redis\n\
                      securityContext:\n        privileged: true\n        runAsUser: 0\n\
                services:\n  db:\n    image: postgres:16\n    environment:\n\
                      - POSTGRES_PASSWORD=postgres\n\
                      - API_URL=http://api:8080\n\
                      - DOCS=http://localhost/docs\n\
                      - REMOTE=http://registry.example.com\n\
                      - METRICS=http://prometheus.monitoring.svc:9090\n\
                      - image: <YOUR-IMAGE>\n";
    assert_eq!(
        found(text, Format::Yaml),
        [
            (Smell::Unpinned, 3),
            (Smell::AdminByDefault, 5),
            (Smell::AdminByDefault, 6),
            (Smell::HardCodedSecret, 11),
        ]
    );
}

#[test]
fn secrets_are_never_in_the_detail() {
    let text = "token: ghp_abcdefghijklmnopqrstuvwxyz0123456789\n";
    let smells = scan(text, Format::Yaml);
    assert_eq!(smells.len(), 1, "one secret per line");
    assert!(!smells[0].detail.contains("ghp_"));
}

#[test]
fn comments_name_no_smell_but_a_suspicious_one() {
    let text = "# password: hunter2 and http://evil.com with md5\n";
    assert!(found(text, Format::Yaml).is_empty());
    let text = "# HACK: workaround for the bug\nkey: value # FIXME\n";
    assert_eq!(
        found(text, Format::Yaml),
        [(Smell::SuspiciousComment, 1), (Smell::SuspiciousComment, 2)]
    );
}

#[test]
fn a_terraform_image_is_no_container() {
    let text = "boot_disk {\n  image = \"debian-cloud/debian-11\"\n}\n";
    assert!(found(text, Format::Hcl).is_empty());
}

#[test]
fn ansible_becomes_root_without_smell() {
    let text = "- hosts: all\n  become_user: root\n  remote_user: root\n";
    assert_eq!(found(text, Format::Yaml), [(Smell::AdminByDefault, 3)]);
}
