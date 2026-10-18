use super::*;
use crate::deps::graph::FileGraph;
use crate::deps::layout::Layout;

fn cs(path: &str, text: &str) -> Source {
    Source {
        path: PathBuf::from(path),
        language: "C#".to_string(),
        text: text.to_string(),
        templates: Vec::new(),
    }
}

/// The paths `from` uses in the graph of `sources`, sorted.
fn used(sources: &[Source], from: &str) -> Vec<String> {
    let graph = FileGraph::build(sources, &Layout::default(), &|_, _| false);
    let mut used: Vec<String> = graph.edges()[Path::new(from)]
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    used.sort();
    used
}

#[test]
fn a_file_is_read_for_its_namespaces_usings_and_types() {
    let src = "global using App.Core;\nusing System;\nusing static App.Util.Math;\nusing Json = System.Text.Json;\n\nnamespace App.Web\n{\n    namespace Api\n    {\n        [Serializable]\n        public sealed partial class Handler<T> : Base where T : class\n        {\n            class Nested {}\n            void Run() { using (var x = Open()) {} }\n        }\n        public record struct Point(int X, int Y);\n        public delegate void Done<T>(T value);\n        enum Color : byte { Red }\n    }\n    interface IStore {}\n}\n";
    let file = read(src);
    assert_eq!(file.namespaces, ["App.Web", "App.Web.Api"]);
    assert_eq!(
        file.declares,
        [
            "App.Web.Api.Handler",
            "App.Web.Api.Point",
            "App.Web.Api.Done",
            "App.Web.Api.Color",
            "App.Web.IStore"
        ]
    );
    assert_eq!(file.usings, ["System"]);
    assert_eq!(file.global_usings, ["App.Core"]);
    assert_eq!(file.statics, ["App.Util.Math"]);
    assert_eq!(
        file.aliases,
        [("Json".to_string(), "System.Text.Json".to_string())]
    );
    let names: Vec<String> = file.names.iter().map(|n| n.join(".")).collect();
    for name in ["SerializableAttribute", "Base", "Open"] {
        assert!(names.contains(&name.to_string()), "{name} in {names:?}");
    }
}

#[test]
fn file_scoped_namespaces_hold_what_follows() {
    let file = read("namespace App.Models;\n\npublic class User {}\n");
    assert_eq!(file.declares, ["App.Models.User"]);
}

#[test]
fn a_file_uses_the_types_it_names_through_its_namespaces_and_usings() {
    let sources = [
        cs(
            "src/Web/Controllers/UserController.cs",
            "using App.Models;\nnamespace App.Web.Controllers;\npublic class UserController { User u; Services.Mailer m; Helper h; }\n",
        ),
        cs(
            "src/Models/User.cs",
            "namespace App.Models;\npublic class User {}\n",
        ),
        cs(
            "src/Models/Order.cs",
            "namespace App.Models;\npublic class Order {}\n",
        ),
        cs(
            "src/Web/Services/Mailer.cs",
            "namespace App.Web.Services { public class Mailer {} }\n",
        ),
        cs(
            "src/Web/Helper.cs",
            "namespace App.Web { public static class Helper {} }\n",
        ),
    ];
    assert_eq!(
        used(&sources, "src/Web/Controllers/UserController.cs"),
        [
            "src/Models/User.cs",
            "src/Web/Helper.cs",
            "src/Web/Services/Mailer.cs"
        ]
    );
}

#[test]
fn partial_types_are_used_whole() {
    let sources = [
        cs("Form.cs", "namespace Ui; public partial class Form {}\n"),
        cs("Form.Designer.cs", "namespace Ui; partial class Form {}\n"),
        cs("Main.cs", "namespace Ui; class Main { Form f; }\n"),
    ];
    assert_eq!(used(&sources, "Main.cs"), ["Form.Designer.cs", "Form.cs"]);
}

#[test]
fn global_usings_hold_for_the_files_under_them() {
    let sources = [
        cs("app/GlobalUsings.cs", "global using Lib.Text;\n"),
        cs("app/Main.cs", "class Main { Parser p; }\n"),
        cs("tool/Run.cs", "class Run { Parser p; }\n"),
        cs(
            "lib/Parser.cs",
            "namespace Lib.Text { public class Parser {} }\n",
        ),
    ];
    assert_eq!(used(&sources, "app/Main.cs"), ["lib/Parser.cs"]);
    assert!(used(&sources, "tool/Run.cs").is_empty());
}

#[test]
fn aliases_static_usings_and_attributes() {
    let sources = [
        cs(
            "Main.cs",
            "using M = Lib.Models;\nusing static Lib.Util;\nusing Clock = Lib.Time.Clock;\n[Audited]\nclass Main { M.User u; Clock c; }\n",
        ),
        cs(
            "Models.cs",
            "namespace Lib.Models { public class User {} }\n",
        ),
        cs("Util.cs", "namespace Lib { public static class Util {} }\n"),
        cs("Clock.cs", "namespace Lib.Time { public class Clock {} }\n"),
        cs(
            "Audited.cs",
            "public class AuditedAttribute : System.Attribute {}\n",
        ),
    ];
    assert_eq!(
        used(&sources, "Main.cs"),
        ["Audited.cs", "Clock.cs", "Models.cs", "Util.cs"]
    );
}
