use super::*;

fn masked(src: &str) -> Vec<usize> {
    let lines: Vec<&str> = src.lines().collect();
    type_declaration_mask(&lines)
        .iter()
        .enumerate()
        .filter(|(_, m)| **m)
        .map(|(i, _)| i + 1)
        .collect()
}

#[test]
fn interfaces_aliases_and_type_imports() {
    let src = "\
import type { Env } from './env'
export interface Handler<E extends Env = Env> {
  (c: Context<E>): Response
}
export type Next = () => Promise<void>
type Union =
  | { a: string }
  | { b: number }
const x = 1
export type { Env }
declare module 'hono' {
  interface ContextVariableMap { user: string }
}
function f(n: number): number {
  return n + 1
}";
    assert_eq!(masked(src), [1, 2, 3, 4, 5, 6, 7, 8, 10, 11, 12, 13]);
}

#[test]
fn a_type_alias_over_several_lines_ends_where_its_brackets_balance() {
    let src = "\
type Schema<T> = {
  [K in keyof T]: T[K] extends Function ? never : K
}
let y = 2";
    assert_eq!(masked(src), [1, 2, 3]);
}

#[test]
fn code_that_names_types_is_kept() {
    let src = "\
const typeName = 'x'
let interfaces = []
module.exports = typeName";
    assert!(masked(src).is_empty());
}
