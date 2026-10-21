use super::*;

#[test]
fn keys_named_as_secrets() {
    for key in [
        "password",
        "db_password",
        "POSTGRES_PASSWORD",
        "clientSecret",
        "AWS_SECRET_ACCESS_KEY",
        "api-key",
        "github.token",
        "private_key",
    ] {
        assert!(names_secret(key), "{key}");
    }
    for key in [
        "secret_name",
        "secretKeyRef",
        "password_length",
        "token_ttl",
        "key",
        "ssh_key_name",
    ] {
        assert!(!names_secret(key), "{key}");
    }
}

#[test]
fn literals_and_values_read_elsewhere() {
    let value = |text, quoted| Value { text, quoted };
    assert!(is_literal(&value("hunter2", false), Format::Yaml));
    assert!(is_literal(&value("hunter2", true), Format::Hcl));
    for text in [
        "${{ secrets.TOKEN }}",
        "{{ vault_pass }}",
        "$DB_PASSWORD",
        "!Ref DbPassword",
        "/run/secrets/db",
        "true",
        "3600",
        "",
    ] {
        assert!(!is_literal(&value(text, false), Format::Yaml), "{text}");
    }
    assert!(
        !is_literal(&value("var.db_password", false), Format::Hcl),
        "an unquoted value of HCL is an expression"
    );
}

#[test]
fn values_shaped_like_keys() {
    assert_eq!(
        shaped("aws_access_key_id = AKIAIOSFODNN7EXAMPLE"),
        Some("an AWS access key")
    );
    assert_eq!(
        shaped("-----BEGIN RSA PRIVATE KEY-----"),
        Some("a private key")
    );
    assert_eq!(
        shaped("t: ghp_abcdefghijklmnopqrstuvwxyz0123456789"),
        Some("a GitHub token")
    );
    assert_eq!(shaped("name: AKIAshort"), None);
    assert_eq!(shaped("x: BAKIAIOSFODNN7EXAMPLE"), None);
}
