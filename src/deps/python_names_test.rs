use super::*;

const MODULE: &str = "\
\"\"\"Prices of a cart.\"\"\"
import os
from decimal import Decimal

RATE = Decimal(\"0.19\")
LIMITS: dict = {
    \"low\": 1,
}


def tax(amount):
    return amount * RATE


@cached
@traced(
    name=\"total\",
)
async def total(cart):
    # sums the lines
    return sum(tax(line) for line in cart)


class Cart:
    def add(self, line):
        self.lines.append(line)

    def tax(self):
        return 0


def _round(value):
    return round(value, 2)


def shown(value):
    return str(_round(value))


if __name__ == \"__main__\":
    print(total([]))
";

fn names(found: &[&str]) -> BTreeSet<String> {
    found.iter().map(|name| name.to_string()).collect()
}

/// The line, from 1, where `text` is written in `MODULE`.
fn line_of(text: &str) -> usize {
    MODULE.lines().position(|l| l.contains(text)).unwrap() + 1
}

fn changed_at(text: &str) -> Result<BTreeSet<String>, usize> {
    changed(MODULE, &[line_of(text)])
}

#[test]
fn top_level_names_are_what_a_module_defines() {
    assert_eq!(
        defined(MODULE),
        ["Cart", "LIMITS", "RATE", "_round", "shown", "tax", "total"]
    );
}

#[test]
fn a_touched_function_changes_and_so_do_those_that_use_it() {
    // `total` calls `tax`; the method `Cart.tax` is another thing.
    assert_eq!(
        changed_at("return amount * RATE"),
        Ok(names(&["tax", "total"]))
    );
    assert_eq!(changed_at("return sum("), Ok(names(&["total"])));
}

#[test]
fn a_private_name_carries_the_change_to_its_users() {
    assert_eq!(changed_at("return round("), Ok(names(&["_round", "shown"])));
}

#[test]
fn a_touched_method_changes_its_class() {
    assert_eq!(changed_at("self.lines.append"), Ok(names(&["Cart"])));
}

#[test]
fn an_assignment_changes_the_names_that_read_it() {
    assert_eq!(changed_at("RATE = "), Ok(names(&["RATE", "tax", "total"])));
    // On any of its lines.
    assert_eq!(changed_at("\"low\": 1"), Ok(names(&["LIMITS"])));
    assert_eq!(
        changed(MODULE, &[line_of("\"low\": 1") + 1]),
        Ok(names(&["LIMITS"]))
    );
}

#[test]
fn a_decorator_belongs_to_what_it_decorates() {
    assert_eq!(changed_at("@cached"), Ok(names(&["total"])));
    assert_eq!(changed_at("name=\"total\""), Ok(names(&["total"])));
    assert_eq!(
        changed(MODULE, &[line_of("name=\"total\"") + 1]),
        Ok(names(&["total"]))
    );
}

#[test]
fn what_changes_no_name_changes_nothing() {
    assert_eq!(changed_at("import os"), Ok(names(&[])));
    assert_eq!(changed_at("Prices of a cart"), Ok(names(&[])));
    assert_eq!(changed_at("print(total([]))"), Ok(names(&[])));
    // A blank line between two definitions.
    assert_eq!(changed(MODULE, &[line_of("def tax") - 1]), Ok(names(&[])));
    // A line past the end.
    assert_eq!(changed(MODULE, &[999]), Ok(names(&[])));
}

#[test]
fn a_comment_inside_a_function_is_a_change_to_it() {
    assert_eq!(changed_at("# sums the lines"), Ok(names(&["total"])));
}

#[test]
fn code_that_runs_on_import_may_concern_every_name() {
    let source = "\
def tax(amount):
    return amount

register(tax)

for name in NAMES:
    globals()[name] = make(name)
";
    assert_eq!(changed(source, &[2]), Ok(names(&["tax"])));
    assert_eq!(changed(source, &[4]), Err(4));
    assert_eq!(changed(source, &[7]), Err(7));
}

#[test]
fn an_if_that_only_chooses_imports_is_neutral() {
    let source = "\
if TYPE_CHECKING:
    from shop.cart import Cart
try:
    import fast as engine
except ImportError:
    import slow as engine

if DEBUG:
    LEVEL = 10
else:
    LEVEL = 30
";
    assert_eq!(changed(source, &[2]), Ok(names(&[])));
    assert_eq!(changed(source, &[6]), Ok(names(&[])));
    // Which value a name gets depends on what runs.
    assert_eq!(changed(source, &[11]), Err(11));
}

#[test]
fn a_signature_closed_at_the_margin_is_one_statement() {
    let source = "\
def tax(
    amount,
) -> int:
    return amount

X += 1
Y == 1
";
    assert_eq!(changed(source, &[3]), Ok(names(&["tax"])));
    assert_eq!(changed(source, &[6]), Ok(names(&["X"])));
    // A comparison assigns nothing: it is code that runs.
    assert_eq!(changed(source, &[7]), Err(7));
}

#[test]
fn a_multi_line_string_at_the_margin_belongs_to_its_statement() {
    let source = "\
HELP = \"\"\"
def not_code():
usage
\"\"\"

def run():
    return HELP
";
    assert_eq!(defined(source), ["HELP", "run"]);
    assert_eq!(changed(source, &[3]), Ok(names(&["HELP", "run"])));
}

fn code(lines: &[&str]) -> Vec<String> {
    lines.iter().map(|line| line.to_string()).collect()
}

#[test]
fn names_read_on_a_module() {
    let code = code(&[
        "total = prices.total(cart) + prices.tax(1)",
        "other = shop.prices.rate",
        "not_prices.shown()",
        "prices_two.shown()",
    ]);
    assert_eq!(
        taken(&code, "prices"),
        Taken {
            names: names(&["tax", "total"]),
            whole: false
        }
    );
    assert_eq!(taken(&code, "shop.prices").names, names(&["rate"]));
}

#[test]
fn a_module_used_as_a_value_may_give_any_name() {
    let code = code(&["handlers = [prices, stock]", "prices.total(1)"]);
    let taken = taken(&code, "prices");
    assert!(taken.whole);
    assert_eq!(taken.names, names(&["total"]));
    assert_eq!(super::taken(&code, "orders"), Taken::default());
}

#[test]
fn a_mention_is_a_whole_word_that_is_no_attribute() {
    assert!(mentions("return tax(1)", "tax"));
    assert!(!mentions("return self.tax(1)", "tax"));
    assert!(!mentions("return taxes(1)", "tax"));
    assert!(!mentions("return pre_tax", "tax"));
}
