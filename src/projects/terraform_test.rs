use super::*;

#[test]
fn sources_of_module_blocks() {
    let text = r#"module "network" {
  source = "../modules/network"
  cidr   = "10.0.0.0/16"
  tags = {
    source = "not this one"
  }
}

# module "old" { source = "../old" }
module "eks" {
  source  = "terraform-aws-modules/eks/aws"
  version = "~> 20.0"
}
module "inline" { source = "./inline" }
resource "aws_s3_object" "o" {
  source = "files/app.zip"
}
"#;
    assert_eq!(
        sources(text),
        [
            "../modules/network",
            "terraform-aws-modules/eks/aws",
            "./inline"
        ]
    );
}

#[test]
fn only_local_sources_are_dependencies() {
    let text = "module \"a\" {\n  source = \"../a\"\n}\nmodule \"b\" {\n  source = \"git::https://x/b.git\"\n}\n";
    let manifest = read(text);
    assert_eq!(manifest.name, None, "the name is that of the directory");
    assert_eq!(
        manifest.deps,
        [RawDep {
            target: DepTarget::Path("../a".into()),
            scope: Scope::Runtime,
        }]
    );
}
