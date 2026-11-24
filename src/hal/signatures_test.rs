use super::*;

fn masked(src: &str) -> Vec<usize> {
    let lines: Vec<&str> = src.lines().collect();
    signature_mask(&lines)
        .iter()
        .enumerate()
        .filter(|(_, m)| **m)
        .map(|(i, _)| i + 1)
        .collect()
}

#[test]
fn signatures_and_their_continuations() {
    let src = "\
go, go' :: Int
  -> Int
go n = n + 1
(<+>) :: V -> V -> V
a <+> b = a
main = print (x :: Int)
";
    assert_eq!(masked(src), vec![1, 2, 4]);
}

#[test]
fn signatures_in_a_where_block() {
    let src = "\
f x = g x
  where
    g :: Int -> Int
    g y = y * 2
";
    assert_eq!(masked(src), vec![3]);
}

#[test]
fn a_constructor_or_an_expression_is_no_signature() {
    let src = "\
data T = T { name :: String }
Just x :: Maybe Int
";
    assert_eq!(masked(src), Vec::<usize>::new());
}
