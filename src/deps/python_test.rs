use super::*;

fn known(paths: &[&str]) -> HashSet<PathBuf> {
    paths.iter().map(PathBuf::from).collect()
}

fn resolved(importer: &str, import: &str, paths: &[&str]) -> Option<PathBuf> {
    resolve(Path::new(importer), import, &known(paths))
}

fn path(text: &str) -> Option<PathBuf> {
    Some(PathBuf::from(text))
}

// ── extraction ───────────────────────────────────────────────────────────────

#[test]
fn import_forms() {
    let src = "\
import os
import shop.cart
import shop.orders as orders, shop.stock
from shop.prices import total
from shop.prices import tax as vat, round_up
from shop import *
";
    assert_eq!(
        extract(src),
        vec![
            "os",
            "shop.cart",
            "shop.orders",
            "shop.stock",
            "shop.prices:total",
            "shop.prices:tax",
            "shop.prices:round_up",
            "shop",
        ]
    );
}

#[test]
fn relative_forms() {
    let src = "\
from . import cart
from .. import stock, prices
from .orders import Order
from ..billing.tax import rate
from .orders import *
";
    assert_eq!(
        extract(src),
        vec![
            ".:cart",
            "..:stock",
            "..:prices",
            ".orders:Order",
            "..billing.tax:rate",
            ".orders",
        ]
    );
}

#[test]
fn parenthesised_list_over_several_lines() {
    let src = "\
from shop.prices import (
    total,  # with tax
    tax as vat,
    round_up,
)
import shop.cart
";
    assert_eq!(
        extract(src),
        vec![
            "shop.prices:total",
            "shop.prices:tax",
            "shop.prices:round_up",
            "shop.cart",
        ]
    );
}

#[test]
fn backslash_continuation() {
    let src = "from shop.prices import total, \\\n    tax\nimport shop.cart\n";
    assert_eq!(
        extract(src),
        vec!["shop.prices:total", "shop.prices:tax", "shop.cart"]
    );
}

#[test]
fn indented_imports_count() {
    let src = "\
if TYPE_CHECKING:
    from shop.cart import Cart

def load():
    import shop.stock
";
    assert_eq!(extract(src), vec!["shop.cart:Cart", "shop.stock"]);
}

#[test]
fn several_statements_in_a_line() {
    assert_eq!(
        extract("import shop.cart; import shop.stock\n"),
        vec!["shop.cart", "shop.stock"]
    );
}

#[test]
fn trailing_comment_is_dropped() {
    assert_eq!(extract("import shop.cart  # the cart\n"), vec!["shop.cart"]);
}

#[test]
fn comments_are_skipped() {
    let src = "# import shop.cart\n    # from shop import stock\nimport shop.orders\n";
    assert_eq!(extract(src), vec!["shop.orders"]);
}

#[test]
fn docstrings_are_skipped() {
    let src = "\
\"\"\"Usage:

import shop.cart
from shop import stock
    from shop import prices\"\"\"
import shop.orders
'''
import shop.billing
'''
";
    assert_eq!(extract(src), vec!["shop.orders"]);
}

#[test]
fn text_before_the_end_of_a_docstring_is_not_code() {
    let src = "x = \"\"\"\nimport shop.cart\"\"\"; import shop.stock\n";
    assert!(extract(src).is_empty());
}

#[test]
fn words_that_only_start_like_an_import() {
    let src = "\
important = 1
import_module('shop.cart')
fromage = 2
    from err
";
    assert!(extract(src).is_empty());
}

#[test]
fn crlf_line_endings() {
    assert_eq!(
        extract("import shop.cart\r\nfrom . import stock\r\n"),
        vec!["shop.cart", ".:stock"]
    );
}

// ── absolute imports ─────────────────────────────────────────────────────────

#[test]
fn absolute_from_the_project_root() {
    let files = ["shop/__init__.py", "shop/cart.py", "main.py"];
    assert_eq!(
        resolved("main.py", "shop.cart", &files),
        path("shop/cart.py")
    );
}

#[test]
fn absolute_from_inside_the_package() {
    let files = [
        "shop/__init__.py",
        "shop/cart.py",
        "shop/billing/__init__.py",
        "shop/billing/tax.py",
    ];
    assert_eq!(
        resolved("shop/billing/tax.py", "shop.cart", &files),
        path("shop/cart.py")
    );
}

#[test]
fn absolute_names_a_package() {
    let files = ["shop/__init__.py", "shop/billing/__init__.py"];
    assert_eq!(
        resolved("main.py", "shop.billing", &files),
        path("shop/billing/__init__.py")
    );
}

#[test]
fn src_layout_from_the_tests() {
    let files = ["src/shop/__init__.py", "src/shop/cart.py"];
    assert_eq!(
        resolved("tests/test_cart.py", "shop.cart", &files),
        path("src/shop/cart.py")
    );
}

#[test]
fn src_layout_of_a_project_inside_the_repository() {
    let files = [
        "services/shop/src/shop/__init__.py",
        "services/shop/src/shop/cart.py",
    ];
    assert_eq!(
        resolved("services/shop/tests/unit/test_cart.py", "shop.cart", &files),
        path("services/shop/src/shop/cart.py")
    );
}

#[test]
fn nearest_root_wins() {
    let files = ["tools/util.py", "util.py"];
    assert_eq!(
        resolved("tools/run.py", "util", &files),
        path("tools/util.py")
    );
}

#[test]
fn a_package_is_not_a_root() {
    // Inside a package `import logging` is the standard library, whatever
    // the package has beside the importer.
    let files = ["shop/__init__.py", "shop/logging.py", "shop/cart.py"];
    assert_eq!(resolved("shop/cart.py", "logging", &files), None);
}

#[test]
fn external_import_resolves_to_nothing() {
    let files = ["shop/__init__.py", "shop/cart.py"];
    assert_eq!(resolved("shop/cart.py", "os.path", &files), None);
    assert_eq!(resolved("shop/cart.py", "requests:get", &files), None);
}

#[test]
fn plain_import_names_a_module_exactly() {
    let files = ["shop/__init__.py"];
    assert_eq!(resolved("main.py", "shop.cart", &files), None);
}

#[test]
fn package_without_init() {
    let files = ["shop/billing/tax.py"];
    assert_eq!(
        resolved("main.py", "shop.billing.tax", &files),
        path("shop/billing/tax.py")
    );
}

// ── `from` imports ───────────────────────────────────────────────────────────

#[test]
fn from_names_a_submodule() {
    let files = ["shop/__init__.py", "shop/cart.py"];
    assert_eq!(
        resolved("main.py", "shop:cart", &files),
        path("shop/cart.py")
    );
}

#[test]
fn from_names_something_the_module_defines() {
    let files = ["shop/__init__.py", "shop/cart.py"];
    assert_eq!(
        resolved("main.py", "shop.cart:Cart", &files),
        path("shop/cart.py")
    );
    assert_eq!(
        resolved("main.py", "shop:Cart", &files),
        path("shop/__init__.py")
    );
}

// ── relative imports ─────────────────────────────────────────────────────────

#[test]
fn relative_module() {
    let files = ["shop/cart.py", "shop/orders.py"];
    assert_eq!(
        resolved("shop/orders.py", ".cart:Cart", &files),
        path("shop/cart.py")
    );
}

#[test]
fn relative_submodule_of_the_package() {
    let files = ["shop/__init__.py", "shop/cart.py", "shop/orders.py"];
    assert_eq!(
        resolved("shop/orders.py", ".:cart", &files),
        path("shop/cart.py")
    );
}

#[test]
fn relative_name_the_package_defines() {
    let files = ["shop/__init__.py", "shop/orders.py"];
    assert_eq!(
        resolved("shop/orders.py", ".:VERSION", &files),
        path("shop/__init__.py")
    );
}

#[test]
fn relative_submodule_of_a_module_beside() {
    let files = [
        "shop/billing/__init__.py",
        "shop/billing/tax.py",
        "shop/orders.py",
    ];
    assert_eq!(
        resolved("shop/orders.py", ".billing:tax", &files),
        path("shop/billing/tax.py")
    );
    assert_eq!(
        resolved("shop/orders.py", ".billing:rate", &files),
        path("shop/billing/__init__.py")
    );
}

#[test]
fn each_extra_dot_goes_one_level_up() {
    let files = ["shop/cart.py", "shop/billing/tax.py"];
    assert_eq!(
        resolved("shop/billing/tax.py", "..cart:Cart", &files),
        path("shop/cart.py")
    );
    assert_eq!(
        resolved("shop/billing/tax.py", "..:cart", &files),
        path("shop/cart.py")
    );
}

#[test]
fn relative_never_looks_in_other_roots() {
    let files = ["cart.py", "shop/orders.py"];
    assert_eq!(resolved("shop/orders.py", ".cart:Cart", &files), None);
}

#[test]
fn more_dots_than_directories() {
    let files = ["cart.py"];
    assert_eq!(resolved("orders.py", "...cart:Cart", &files), None);
}

#[test]
fn relative_star_names_the_module() {
    let files = ["shop/cart.py", "shop/orders.py"];
    assert_eq!(
        resolved("shop/orders.py", ".cart", &files),
        path("shop/cart.py")
    );
}

// ── stubs ────────────────────────────────────────────────────────────────────

#[test]
fn source_comes_before_its_stub() {
    let files = ["shop/cart.py", "shop/cart.pyi"];
    assert_eq!(
        resolved("main.py", "shop.cart", &files),
        path("shop/cart.py")
    );
}

#[test]
fn stub_stands_for_a_module_without_source() {
    let files = ["shop/fast.pyi"];
    assert_eq!(
        resolved("main.py", "shop.fast", &files),
        path("shop/fast.pyi")
    );
}
